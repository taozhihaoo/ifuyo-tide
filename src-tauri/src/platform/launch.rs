//! LaunchRequest parsing + validation (M8 §2.2/§9/§40/§57/§58).
//!
//! EVERY external input — CLI args, deep links, second-instance
//! forwards — is UNTRUSTED. Parsing classifies; validation rejects:
//! - scheme must be http/https/magnet or an existing file path
//! - length caps before any heavy work (§58)
//! - magnet: parsed structurally (librqbit Magnet), never
//!   starts_with-only (§9.2)
//! - http(s): reused M1 URL validator (§9.1 — no duplicate validation)
//! - torrent/media paths: canonicalized, existence + regular-file +
//!   extension + size checks (§9.3)
//!
//! A LaunchRequest is NOT a task (§46): it only carries validated
//! intent; task creation stays behind explicit user confirmation.

use crate::core::download::url::validate_download_url;
use crate::error::TideError;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Maximum accepted input length for any single argument (§58):
/// magnets can be legitimately large (trackers), URIs are small;
/// 64 KiB is generous for both and blocks multi-megabyte queries.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Reasonable .torrent metadata size (§9.3): real metadata is
/// typically < a few MiB.
pub const MAX_TORRENT_FILE_LEN: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum LaunchKind {
    /// http/https URL → download task (after user-visible confirmation
    /// flow in the UI).
    OpenHttpUrl(String),
    /// magnet URI → source intake.
    OpenMagnet(String),
    /// canonical path to an existing .torrent file.
    OpenTorrentFile(PathBuf),
    /// canonical path to a local media file → library/ playback entry.
    OpenMediaFile(PathBuf),
    /// just activate the window.
    Focus,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LaunchRequest {
    pub kind: LaunchKind,
}

impl LaunchRequest {
    pub fn focus() -> LaunchRequest {
        LaunchRequest {
            kind: LaunchKind::Focus,
        }
    }
}

fn validation_error(what: &str) -> TideError {
    TideError::Streaming(crate::error::StreamingFailure::Gateway(format!(
        "launch request rejected: {what}"
    )))
}

/// Media extensions Tide can open locally (delegating actual decoding
/// to the M5 player — §9.4).
fn is_media_extension(ext: &str) -> bool {
    matches!(
        ext,
        "mp4"
            | "m4v"
            | "mkv"
            | "webm"
            | "mov"
            | "avi"
            | "mp3"
            | "m4a"
            | "flac"
            | "wav"
            | "ogg"
            | "opus"
    )
}

/// Classifies ONE raw CLI/deep-link argument into a LaunchRequest.
/// `Parse` = classify + validate. Returns Err for anything malformed,
/// unsupported or oversized (§9/§40: never routes garbage onward).
pub fn parse_argument(raw: &str) -> Result<LaunchRequest, TideError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(validation_error("empty argument"));
    }
    if raw.len() > MAX_INPUT_LEN {
        return Err(validation_error("argument exceeds maximum length"));
    }

    // tide:// deep link: tide://add?<param>=<value> (§10.1).
    if let Some(rest) = raw.strip_prefix("tide://") {
        return parse_tide_deeplink(rest);
    }

    // magnet: structural validation via librqbit (v1 hash required —
    // the same validator M3 uses), then normalize (§9.2).
    if raw.starts_with("magnet:") {
        crate::core::torrent::source::validate_magnet_uri(raw)?;
        return Ok(LaunchRequest {
            kind: LaunchKind::OpenMagnet(raw.to_string()),
        });
    }

    // http/https: M1 URL validator (§9.1), private-address policy is
    // applied later at task creation (allow_private like the UI flow).
    if raw.starts_with("http://") || raw.starts_with("https://") {
        validate_download_url(raw, true)?;
        return Ok(LaunchRequest {
            kind: LaunchKind::OpenHttpUrl(raw.to_string()),
        });
    }

    // Everything else: treat as a FILE PATH (§9.3). Must exist.
    parse_file_path(Path::new(raw))
}

/// `tide://add?magnet=...` / `tide://add?url=...` (percent-encoded).
/// Unknown commands and oversized payloads are rejected (§10.3).
fn parse_tide_deeplink(rest: &str) -> Result<LaunchRequest, TideError> {
    let (command, query) = rest
        .split_once('?')
        .ok_or_else(|| validation_error("deep link missing query"))?;
    if command != "add" {
        return Err(validation_error("unsupported deep link command"));
    }
    if query.len() > MAX_INPUT_LEN {
        return Err(validation_error("deep link query exceeds maximum length"));
    }
    // §10.3: exactly ONE parameter is accepted — unknown keys, missing
    // values and extra parameters are all rejected (no silent drops).
    let (key, value) = query
        .split_once('=')
        .ok_or_else(|| validation_error("deep link missing parameter"))?;
    if query.contains('&') {
        return Err(validation_error(
            "multiple deep link parameters unsupported",
        ));
    }
    match key {
        "magnet" | "url" => {
            let decoded = percent_decode(value)?;
            parse_argument(&decoded)
        }
        _ => Err(validation_error("unknown deep link parameter")),
    }
}

/// Percent-decoding with strict validation: malformed escapes are
/// rejected (§40 "invalid percent encoding"), not silently kept.
fn percent_decode(value: &str) -> Result<String, TideError> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(validation_error("truncated percent escape"));
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|_| validation_error("invalid percent escape"))?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| validation_error("invalid percent escape"))?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| validation_error("decoded value is not valid UTF-8"))
}

/// File-path classification (§9.3): canonicalize (rejects nonexistent
/// paths), require a regular file, check extension, check size for
/// .torrent. Device names / reserved names are rejected by the M6
/// path helpers. UNC/reparse handling follows M6: canonicalization
/// resolves reparse points and the result must remain a plain path.
pub fn parse_file_path(path: &Path) -> Result<LaunchRequest, TideError> {
    let canonical = crate::core::library::path::canonicalize(path)?;
    if !canonical.is_file() {
        return Err(validation_error("path is not a regular file"));
    }
    let name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if crate::core::library::path::has_reserved_device_name(&canonical) {
        return Err(TideError::Security("reserved device name".into()));
    }

    match extension.as_str() {
        "torrent" => {
            let len = std::fs::metadata(&canonical)?.len();
            if len == 0 || len > MAX_TORRENT_FILE_LEN {
                return Err(validation_error(".torrent metadata size out of range"));
            }
            Ok(LaunchRequest {
                kind: LaunchKind::OpenTorrentFile(canonical),
            })
        }
        ext if is_media_extension(ext) => Ok(LaunchRequest {
            kind: LaunchKind::OpenMediaFile(canonical),
        }),
        _ => Err(validation_error("unsupported file type")),
    }
}

/// Classifies the FULL initial-process argument list (§8.2/§75):
/// empty → Focus; multiple valid requests are ALL returned (§75:
/// never silently dropped); one invalid argument fails the batch
/// with a structured error rather than partially executing.
pub fn parse_args(args: &[String]) -> Result<Vec<LaunchRequest>, TideError> {
    if args.is_empty() {
        return Ok(vec![LaunchRequest::focus()]);
    }
    args.iter().map(|a| parse_argument(a)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_whitespace_rejected() {
        assert!(parse_argument("").is_err());
        assert!(parse_argument("   ").is_err());
    }

    #[test]
    fn oversized_input_rejected() {
        let huge = "magnet:?xt=urn:btih:".to_string() + &"a".repeat(MAX_INPUT_LEN + 1);
        assert!(parse_argument(&huge).is_err());
    }

    #[test]
    fn http_urls_classified_via_m1_validator() {
        let ok = parse_argument("http://127.0.0.1/file.zip").unwrap();
        assert!(matches!(ok.kind, LaunchKind::OpenHttpUrl(_)));
        // M1 policy: private addresses need allow_private (true here).
        let bad = parse_argument("ftp://example.com/file.zip");
        assert!(bad.is_err());
        let malformed = parse_argument("http://");
        assert!(malformed.is_err());
    }

    #[test]
    fn magnet_requires_structure() {
        // starts_with("magnet:") alone is NOT valid (§9.2).
        assert!(parse_argument("magnet:").is_err());
        assert!(parse_argument("magnet:?xt=not-a-hash").is_err());
        let ok =
            parse_argument("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567").unwrap();
        assert!(matches!(ok.kind, LaunchKind::OpenMagnet(_)));
    }

    #[test]
    fn unsupported_scheme_rejected() {
        assert!(parse_argument("javascript:alert(1)").is_err());
        assert!(parse_argument("file:///C:/Windows/System32/calc.exe").is_err());
        assert!(parse_argument("random.exe.argument").is_err());
    }

    #[test]
    fn tide_deeplink_parsed_and_validated() {
        let ok = parse_argument(
            "tide://add?magnet=magnet:%3Fxt%3Durn:btih:0123456789abcdef0123456789abcdef01234567",
        )
        .expect("valid magnet deep link");
        assert!(matches!(ok.kind, LaunchKind::OpenMagnet(_)));

        // Unsupported command.
        assert!(parse_argument("tide://delete?everything=1").is_err());
        // Unknown parameter.
        assert!(parse_argument("tide://add?evil=1").is_err());
        // Missing parameter.
        assert!(parse_argument("tide://add?").is_err());
        // Malformed percent encoding rejected (§40).
        assert!(parse_argument("tide://add?url=http%3A%2F%2F%ZZ").is_err());
        // Oversized query rejected.
        let huge = format!("tide://add?url=http://a.com/{}", "a".repeat(MAX_INPUT_LEN));
        assert!(parse_argument(&huge).is_err());
    }

    #[test]
    fn percent_decode_strict() {
        assert_eq!(percent_decode("a%20b").unwrap(), "a b");
        assert!(percent_decode("%ZZ").is_err());
        assert!(percent_decode("%A").is_err());
    }

    #[test]
    fn parse_args_empty_is_focus() {
        let requests = parse_args(&[]).unwrap();
        assert_eq!(requests, vec![LaunchRequest::focus()]);
    }

    /// §75: multiple requests are ALL returned, never dropped.
    #[test]
    fn multiple_args_all_returned() {
        let args = vec![
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".to_string(),
            "http://127.0.0.1/file.zip".to_string(),
        ];
        let requests = parse_args(&args).unwrap();
        assert_eq!(requests.len(), 2);
    }

    /// §40: one invalid argument fails the batch structurally.
    #[test]
    fn invalid_arg_fails_batch() {
        let args = vec![
            "http://127.0.0.1/ok".to_string(),
            "garbage no scheme".to_string(),
        ];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn torrent_path_classification() {
        let dir = std::env::temp_dir().join(format!("tide-m8-{}", crate::core::ids::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let torrent = dir.join("real.torrent");
        std::fs::write(&torrent, b"d4:infod4:name4:teste").unwrap();

        let ok = parse_argument(torrent.to_string_lossy().as_ref()).unwrap();
        assert!(matches!(ok.kind, LaunchKind::OpenTorrentFile(_)));

        // Missing file rejected.
        assert!(parse_argument(dir.join("missing.torrent").to_string_lossy().as_ref()).is_err());
        // Directory-as-file rejected.
        assert!(parse_argument(dir.to_string_lossy().as_ref()).is_err());
        // Unsupported extension rejected.
        let exe = dir.join("something.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        assert!(parse_argument(exe.to_string_lossy().as_ref()).is_err());
        // Oversized .torrent rejected (§9.3).
        let big = dir.join("big.torrent");
        std::fs::write(&big, vec![0u8; (MAX_TORRENT_FILE_LEN + 1) as usize]).unwrap();
        assert!(parse_argument(big.to_string_lossy().as_ref()).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn media_path_classification() {
        let dir = std::env::temp_dir().join(format!("tide-m8-{}", crate::core::ids::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let media = dir.join("movie.mp4");
        std::fs::write(&media, b"fake").unwrap();
        let ok = parse_argument(media.to_string_lossy().as_ref()).unwrap();
        assert!(matches!(ok.kind, LaunchKind::OpenMediaFile(_)));
        let _ = std::fs::remove_dir_all(dir);
    }
}
