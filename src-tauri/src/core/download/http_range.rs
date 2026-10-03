//! HTTP Range / Content-Range parsing and resolution.
//!
//! Production module promoted from the M0.5 spike (was
//! `spike/range.rs`); extended with `Content-Range` response parsing for
//! resume verification. Semantics follow RFC 9110 §14.

/// A parsed `Range` header value (RFC 9110), single-range scope.
///
/// Multi-range headers are reported as [`RangeParse::Unsupported`]; the
/// downloader treats them as "no range" (serve/receive the full body),
/// which RFC 9110 permits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeParse {
    /// `bytes=start-end` (inclusive end) or `bytes=start-` (open ended).
    Single { first: u64, last: Option<u64> },
    /// `bytes=-suffix` — the final N bytes.
    Suffix(u64),
    /// Malformed (e.g. reversed, non-numeric, wrong unit).
    Invalid,
    /// Syntactically valid but not supported (multi-range).
    Unsupported,
}

/// A parsed `Content-Range` response header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentRangeParse {
    /// `bytes <start>-<end>/<total>` — inclusive range, `*` total if unknown.
    Valid {
        start: u64,
        end_inclusive: u64,
        total: Option<u64>,
    },
    /// `bytes */<total>` — the server rejected the requested range but
    /// disclosed the current representation size (used with 416).
    Unsatisfiable { total: u64 },
    /// Malformed or wrong unit.
    Invalid,
    /// Header absent.
    Absent,
}

/// A byte range resolved against a concrete resource length.
///
/// `end` is EXCLUSIVE (Tide convention, see `core::resource::ByteRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedRange {
    pub start: u64,
    pub end: u64,
}

impl ResolvedRange {
    pub fn len(&self) -> u64 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Parses a `Range` header value such as `bytes=0-1023`.
pub fn parse_range_header(raw: &str) -> RangeParse {
    let Some(unit_and_spec) = raw.strip_prefix("bytes=") else {
        return RangeParse::Invalid;
    };
    let spec = unit_and_spec.trim();
    if spec.is_empty() || spec.contains(',') {
        return if spec.contains(',') {
            RangeParse::Unsupported
        } else {
            RangeParse::Invalid
        };
    }
    let (first_str, last_str) = match spec.split_once('-') {
        Some(parts) => parts,
        None => return RangeParse::Invalid,
    };
    if first_str.is_empty() {
        return match last_str.parse::<u64>() {
            Ok(n) if n > 0 => RangeParse::Suffix(n),
            _ => RangeParse::Invalid,
        };
    }
    let Ok(first) = first_str.parse::<u64>() else {
        return RangeParse::Invalid;
    };
    if last_str.is_empty() {
        return RangeParse::Single { first, last: None };
    }
    match last_str.parse::<u64>() {
        Ok(last) if last >= first => RangeParse::Single {
            first,
            last: Some(last),
        },
        _ => RangeParse::Invalid,
    }
}

/// Resolves a parsed header against the resource length.
///
/// Returns `None` when the range is not satisfiable (→ HTTP 416).
pub fn resolve_range(parsed: &RangeParse, total_len: u64) -> Option<ResolvedRange> {
    match parsed {
        RangeParse::Single { first, last } => {
            if *first >= total_len {
                return None;
            }
            let end = match last {
                // saturating: bytes=0-18446744073709551615 must not
                // overflow (found by the M1 property-style test).
                Some(last) => last.saturating_add(1).min(total_len),
                None => total_len,
            };
            Some(ResolvedRange {
                start: *first,
                end: end.max(*first),
            })
        }
        RangeParse::Suffix(n) => {
            if *n == 0 || total_len == 0 {
                return None;
            }
            let start = total_len.saturating_sub(*n);
            Some(ResolvedRange {
                start,
                end: total_len,
            })
        }
        RangeParse::Invalid | RangeParse::Unsupported => None,
    }
}

/// Formats a `Range` request header value for resuming at `offset`.
pub fn resume_range_header(offset: u64) -> String {
    format!("bytes={offset}-")
}

/// Parses a `Content-Range` response header value.
pub fn parse_content_range_header(raw: Option<&str>) -> ContentRangeParse {
    let Some(value) = raw else {
        return ContentRangeParse::Absent;
    };
    let value = value.trim();
    let Some(rest) = value.strip_prefix("bytes ") else {
        return ContentRangeParse::Invalid;
    };
    let (range_part, total_part) = match rest.split_once('/') {
        Some(parts) => parts,
        None => return ContentRangeParse::Invalid,
    };
    if range_part.trim() == "*" {
        return match total_part.trim().parse::<u64>() {
            Ok(total) => ContentRangeParse::Unsatisfiable { total },
            _ => ContentRangeParse::Invalid,
        };
    }
    let Some((start_str, end_str)) = range_part.split_once('-') else {
        return ContentRangeParse::Invalid;
    };
    let (Ok(start), Ok(end_inclusive)) = (
        start_str.trim().parse::<u64>(),
        end_str.trim().parse::<u64>(),
    ) else {
        return ContentRangeParse::Invalid;
    };
    if end_inclusive < start {
        return ContentRangeParse::Invalid;
    }
    let total = match total_part.trim() {
        "*" => None,
        t => match t.parse::<u64>() {
            // A total smaller than the inclusive end is inconsistent.
            Ok(total) if total > end_inclusive => Some(total),
            _ => return ContentRangeParse::Invalid,
        },
    };
    ContentRangeParse::Valid {
        start,
        end_inclusive,
        total,
    }
}

/// Formats a `Content-Range` value for a resolved range.
pub fn content_range(resolved: &ResolvedRange, total_len: u64) -> String {
    format!(
        "bytes {}-{}/{}",
        resolved.start,
        resolved.end.saturating_sub(1),
        total_len
    )
}

/// Outcome of resolving a `Range` header against a resource length,
/// mapped 1:1 to HTTP responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeResolution {
    /// No header (or unsupported multi-range): serve the full body (200).
    Full,
    /// Serve the byte range (206).
    Satisfied(ResolvedRange),
    /// Syntactically invalid header (400).
    Invalid,
    /// Out of bounds (416).
    NotSatisfiable,
}

pub fn resolve_range_header(raw: Option<&str>, total_len: u64) -> RangeResolution {
    let Some(value) = raw else {
        return RangeResolution::Full;
    };
    match parse_range_header(value) {
        RangeParse::Invalid => RangeResolution::Invalid,
        RangeParse::Unsupported => RangeResolution::Full,
        parsed => match resolve_range(&parsed, total_len) {
            Some(resolved) => RangeResolution::Satisfied(resolved),
            None => RangeResolution::NotSatisfiable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TideError;

    fn single(first: u64, last: Option<u64>) -> RangeParse {
        RangeParse::Single { first, last }
    }

    #[test]
    fn parses_closed_range() {
        assert_eq!(parse_range_header("bytes=0-1023"), single(0, Some(1023)));
        assert_eq!(
            parse_range_header("bytes=1000-1999"),
            single(1000, Some(1999))
        );
    }

    #[test]
    fn parses_open_ended_range() {
        assert_eq!(parse_range_header("bytes=1024-"), single(1024, None));
    }

    #[test]
    fn parses_suffix_range() {
        assert_eq!(parse_range_header("bytes=-1024"), RangeParse::Suffix(1024));
    }

    #[test]
    fn rejects_reversed_and_garbage() {
        assert_eq!(parse_range_header("bytes=10000-9999"), RangeParse::Invalid);
        assert_eq!(parse_range_header("bytes=abc-def"), RangeParse::Invalid);
        assert_eq!(parse_range_header("bytes="), RangeParse::Invalid);
        assert_eq!(parse_range_header("chunks=0-1"), RangeParse::Invalid);
        assert_eq!(parse_range_header(""), RangeParse::Invalid);
        // Property-style: hostile inputs must never panic and must parse
        // or be Invalid.
        for hostile in [
            "bytes=-1",
            "bytes=abc",
            "bytes=100-50",
            "bytes=18446744073709551615-",
            "bytes=0-18446744073709551615",
            "bytes=+-",
            "bytes=0-1,5-9",
            "bytes=999999999999999999999999-",
        ] {
            // Must never panic; any parse outcome is acceptable as long
            // as resolution is total.
            let parsed = parse_range_header(hostile);
            let _ = resolve_range(&parsed, 1000);
        }
    }

    #[test]
    fn huge_values_do_not_panic_or_overflow() {
        // u64::MAX open-ended against a small file: unsatisfiable.
        let parsed = parse_range_header("bytes=18446744073709551615-");
        assert!(resolve_range(&parsed, 1000).is_none());
        // Suffix larger than any possible file.
        let parsed = parse_range_header("bytes=-18446744073709551615");
        assert!(matches!(parsed, RangeParse::Suffix(_)));
        let resolved = resolve_range(&parsed, 1000);
        assert_eq!(
            resolved,
            Some(ResolvedRange {
                start: 0,
                end: 1000
            })
        );
    }

    #[test]
    fn flags_multi_range_as_unsupported() {
        assert_eq!(parse_range_header("bytes=0-1,5-9"), RangeParse::Unsupported);
    }

    #[test]
    fn resolves_closed_range_clamped_to_length() {
        let resolved = resolve_range(&parse_range_header("bytes=0-1023"), 5000).unwrap();
        assert_eq!(
            resolved,
            ResolvedRange {
                start: 0,
                end: 1024
            }
        );

        let resolved = resolve_range(&parse_range_header("bytes=4000-99999"), 5000).unwrap();
        assert_eq!(resolved.end, 5000);
        assert_eq!(resolved.len(), 1000);
    }

    #[test]
    fn out_of_bounds_is_unsatisfiable() {
        assert!(resolve_range(&parse_range_header("bytes=9999-10000"), 5000).is_none());
        assert!(resolve_range(&parse_range_header("bytes=-0"), 5000).is_none());
        assert!(resolve_range(&parse_range_header("bytes=0-"), 0).is_none());
    }

    #[test]
    fn content_range_is_inclusive() {
        let resolved = resolve_range(&parse_range_header("bytes=0-1023"), 5000).unwrap();
        assert_eq!(content_range(&resolved, 5000), "bytes 0-1023/5000");
    }

    #[test]
    fn parses_content_range_valid() {
        assert_eq!(
            parse_content_range_header(Some("bytes 1000-1999/5000")),
            ContentRangeParse::Valid {
                start: 1000,
                end_inclusive: 1999,
                total: Some(5000)
            }
        );
        assert_eq!(
            parse_content_range_header(Some("bytes 0-0/*")),
            ContentRangeParse::Valid {
                start: 0,
                end_inclusive: 0,
                total: None
            }
        );
    }

    #[test]
    fn parses_content_range_unsatisfiable() {
        assert_eq!(
            parse_content_range_header(Some("bytes */12345")),
            ContentRangeParse::Unsatisfiable { total: 12345 }
        );
    }

    #[test]
    fn content_range_invalid_and_absent() {
        assert_eq!(parse_content_range_header(None), ContentRangeParse::Absent);
        for bad in [
            Some("bytes 100-50/5000"),
            Some("bytes 100-/5000"),
            Some("items 0-5/10"),
            Some("bytes 0-5/3"), // total smaller than range end
            Some("bytes 0-5"),
            Some(""),
            Some("bytes */abc"),
        ] {
            assert_eq!(
                parse_content_range_header(bad),
                ContentRangeParse::Invalid,
                "{bad:?} must be Invalid"
            );
        }
    }

    #[test]
    fn resume_header_format() {
        assert_eq!(resume_range_header(524288000), "bytes=524288000-");
    }

    #[test]
    fn range_resolution_outcomes() {
        assert_eq!(resolve_range_header(None, 100), RangeResolution::Full);
        assert_eq!(
            resolve_range_header(Some("bytes=bad"), 100),
            RangeResolution::Invalid
        );
        assert_eq!(
            resolve_range_header(Some("bytes=500-600"), 100),
            RangeResolution::NotSatisfiable
        );
        assert_eq!(
            resolve_range_header(Some("bytes=0-1,5-9"), 100),
            RangeResolution::Full
        );
    }

    #[test]
    fn tide_error_paths_still_work() {
        // ensure module stays coherent with error model (no unused import
        // rot when TideError usage evolves).
        let _err = TideError::InvalidInput("range test".into());
    }
}
