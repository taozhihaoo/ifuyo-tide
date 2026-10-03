// M0.5 SPIKE — HTTP Range header parser.
//
// This file is a PRODUCTION CANDIDATE: pure logic, no I/O, fully unit
// tested. It is expected to move into the real M4 gateway with the same
// semantics. Everything else in `spike/` is throwaway.

/// A parsed `Range` header value (RFC 7233), single-range scope.
///
/// Multi-range headers are reported as [`RangeParse::Unsupported`]; the
/// spike gateway responds 200 (full body) for those, which RFC 7233
/// explicitly permits ("A server MAY ignore the Range header").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeParse {
    /// `bytes=start-end` (inclusive end) or `bytes=start-` (open ended).
    Single { first: u64, last: Option<u64> },
    /// `bytes=-suffix` — the final N bytes.
    Suffix(u64),
    /// Malformed (e.g. reversed, non-numeric, wrong unit).
    Invalid,
    /// Syntactically valid but not supported by the spike (multi-range).
    Unsupported,
}

/// Parses a `Range` header value such as `bytes=0-1023`.
pub fn parse_range_header(raw: &str) -> RangeParse {
    let Some(unit_and_spec) = raw.strip_prefix("bytes=") else {
        return RangeParse::Invalid;
    };
    let spec = unit_and_spec.trim();
    if spec.is_empty() || spec.contains(',') {
        // empty spec or multi-range
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
        // suffix form: bytes=-N
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
                Some(last) => (last + 1).min(total_len),
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

/// Formats a `Content-Range` value for a resolved range.
pub fn content_range(resolved: &ResolvedRange, total_len: u64) -> String {
    // HTTP ranges are inclusive: bytes start-end/total
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
    }

    #[test]
    fn flags_multi_range_as_unsupported() {
        assert_eq!(parse_range_header("bytes=0-1,5-9"), RangeParse::Unsupported);
    }

    #[test]
    fn resolves_closed_range_clamped_to_length() {
        let parsed = parse_range_header("bytes=0-1023");
        let resolved = resolve_range(&parsed, 5000).unwrap();
        assert_eq!(resolved.start, 0);
        assert_eq!(resolved.end, 1024);
        assert_eq!(resolved.len(), 1024);

        // end beyond EOF clamps to file length
        let parsed = parse_range_header("bytes=4000-99999");
        let resolved = resolve_range(&parsed, 5000).unwrap();
        assert_eq!(resolved.end, 5000);
        assert_eq!(resolved.len(), 1000);
    }

    #[test]
    fn resolves_open_ended_to_eof() {
        let parsed = parse_range_header("bytes=1024-");
        let resolved = resolve_range(&parsed, 5000).unwrap();
        assert_eq!(resolved.start, 1024);
        assert_eq!(resolved.end, 5000);
    }

    #[test]
    fn resolves_suffix_from_eof() {
        let parsed = parse_range_header("bytes=-1024");
        let resolved = resolve_range(&parsed, 5000).unwrap();
        assert_eq!(resolved.start, 3976);
        assert_eq!(resolved.end, 5000);

        // suffix larger than file → whole file
        let resolved = resolve_range(&parse_range_header("bytes=-999999"), 100).unwrap();
        assert_eq!(resolved.start, 0);
        assert_eq!(resolved.end, 100);
    }

    #[test]
    fn out_of_bounds_is_unsatisfiable() {
        // bytes=9999-10000 against a 5000-byte file
        assert!(resolve_range(&parse_range_header("bytes=9999-10000"), 5000).is_none());
        assert!(resolve_range(&parse_range_header("bytes=-0"), 5000).is_none());
        assert!(resolve_range(&parse_range_header("bytes=0-"), 0).is_none());
    }

    #[test]
    fn content_range_is_inclusive() {
        let resolved = resolve_range(&parse_range_header("bytes=0-1023"), 5000).unwrap();
        assert_eq!(content_range(&resolved, 5000), "bytes 0-1023/5000");
        let resolved = resolve_range(&parse_range_header("bytes=100-"), 5000).unwrap();
        assert_eq!(content_range(&resolved, 5000), "bytes 100-4999/5000");
    }

    #[test]
    fn resolve_range_header_maps_outcomes() {
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
        assert_eq!(
            resolve_range_header(Some("bytes=0-9"), 100),
            RangeResolution::Satisfied(ResolvedRange { start: 0, end: 10 })
        );
    }
}
