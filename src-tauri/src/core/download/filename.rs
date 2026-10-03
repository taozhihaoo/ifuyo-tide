//! Download filename resolution and Windows sanitization (M1 §9–§10).
//!
//! Priority (M1 §10):
//!   1. user-specified filename
//!   2. Content-Disposition `filename*` (RFC 5987 / RFC 6266)
//!   3. Content-Disposition `filename`
//!   4. URL path last segment
//!   5. fallback
//!
//! A server-provided name is a FILE NAME, never a path: separators,
//! drive letters and traversal are stripped, then Windows-specific
//! sanitization applies.

use crate::error::TideError;
use std::path::Path;

pub const MAX_FILENAME_LEN: usize = 200;

/// Windows reserved device names (case-insensitive, no extension needed).
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitizes a file name for Windows: strips path components, illegal
/// characters, reserved device names, trailing dots/spaces, and clamps
/// length. Returns a non-empty name.
pub fn sanitize_filename(raw: &str) -> String {
    let mut name = raw.trim().to_string();

    // Never allow a name to become a path: keep only the last segment
    // after both separators, then drop any drive-letter prefix.
    for sep in ['\\', '/'] {
        if let Some(pos) = name.rfind(sep) {
            name = name[pos + sep.len_utf8()..].to_string();
        }
    }
    if name.len() >= 2 && name.as_bytes()[1] == b':' && name.as_bytes()[0].is_ascii_alphabetic() {
        name = name[2..].to_string();
    }
    if name.contains(':') {
        name = name.split(':').last().unwrap_or_default().to_string();
    }

    // Illegal Windows name characters (also blocks control chars).
    name = name
        .chars()
        .map(|c| match c {
            '<' | '>' | '|' | '"' | '?' | '*' | '\0'..='\x1f' => '_',
            c => c,
        })
        .collect();

    // Reserved device names: "CON", "CON.txt", "con .mkv" all collide.
    let stem = name.split('.').next().unwrap_or_default();
    if RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        name = format!("_{name}");
    }

    // Trailing dots and spaces are silently dropped by Windows.
    let trimmed = name.trim_end_matches(['.', ' ']).to_string();
    name = trimmed;

    if name.is_empty() {
        name = "download.bin".to_string();
    }

    // Clamp length, preserving the extension where possible.
    if name.chars().count() > MAX_FILENAME_LEN {
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        let stem_len = MAX_FILENAME_LEN.saturating_sub(ext.chars().count());
        let stem: String = name.chars().take(stem_len).collect();
        name = format!("{stem}{ext}");
    }

    name
}

/// Extracts a filename from a `Content-Disposition` header value.
///
/// Handles `filename*` (RFC 5987: charset'lang percent-encoded) with
/// priority, then quoted or bare `filename`.
pub fn filename_from_content_disposition(header: &str) -> Option<String> {
    let mut plain_fallback: Option<String> = None;
    for part in header.split(';').map(str::trim) {
        if let Some(value) = part
            .strip_prefix("filename*")
            .and_then(|rest| rest.strip_prefix('='))
        {
            // e.g. utf-8''%E4%BD%A0%E5%A5%BD.mkv
            let value = value.trim_matches('"');
            let mut segments = value.splitn(2, '\'');
            let charset = segments.next().unwrap_or("utf-8");
            let encoded = segments
                .next()
                .and_then(|rest| rest.splitn(2, '\'').nth(1))
                .unwrap_or(value);
            if !charset.eq_ignore_ascii_case("utf-8") && !charset.eq_ignore_ascii_case("iso-8859-1")
            {
                continue;
            }
            if let Ok(decoded) = percent_decode_utf8(encoded) {
                if !decoded.trim().is_empty() {
                    return Some(decoded);
                }
            }
        } else if let Some(value) = part
            .strip_prefix("filename")
            .and_then(|rest| rest.strip_prefix('='))
        {
            let value = value.trim().trim_matches('"');
            if !value.trim().is_empty() {
                plain_fallback = Some(value.to_string());
            }
        }
    }
    plain_fallback
}

fn percent_decode_utf8(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 < bytes.len() + 1 {
            let hex = bytes.get(i + 1..i + 3).ok_or(())?;
            let hex = std::str::from_utf8(hex).map_err(|_| ())?;
            let byte = u8::from_str_radix(hex, 16).map_err(|_| ())?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

/// Derives a filename from the URL path's last segment (percent-decoded).
pub fn filename_from_url(url: &url::Url) -> Option<String> {
    let path = url.path();
    if path.is_empty() || path == "/" {
        return None;
    }
    let segment = path.rsplit('/').find(|s| !s.is_empty())?;
    if segment.is_empty() {
        return None;
    }
    let decoded = if segment.contains('%') {
        percent_decode_utf8(segment).unwrap_or_else(|_| segment.to_string())
    } else {
        segment.to_string()
    };
    if decoded.trim().is_empty() {
        None
    } else {
        Some(decoded)
    }
}

/// Resolves the final download filename by the M1 priority order.
pub fn resolve_filename(
    user_specified: Option<&str>,
    content_disposition: Option<&str>,
    url: &url::Url,
) -> Result<String, TideError> {
    let raw = if let Some(user) = user_specified {
        let user = user.trim();
        if user.is_empty() {
            return Err(TideError::Download(
                crate::core::download::url::DownloadFailure::InvalidUrl {
                    reason: "filename must not be empty".into(),
                },
            ));
        }
        user.to_string()
    } else if let Some(cd) = content_disposition.and_then(filename_from_content_disposition) {
        cd
    } else if let Some(from_url) = filename_from_url(url) {
        from_url
    } else {
        "download.bin".to_string()
    };
    Ok(sanitize_filename(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_components() {
        assert_eq!(sanitize_filename("file.bin"), "file.bin");
        assert_eq!(sanitize_filename("../../secret.exe"), "secret.exe");
        assert_eq!(sanitize_filename("..\\..\\secret.exe"), "secret.exe");
        assert_eq!(sanitize_filename("C:\\Users\\x\\f.mkv"), "f.mkv");
        assert_eq!(sanitize_filename("/etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("a:b.bin"), "b.bin");
    }

    #[test]
    fn sanitize_handles_windows_hostiles() {
        assert_eq!(sanitize_filename("file<1>.bin"), "file_1_.bin");
        assert_eq!(sanitize_filename("a|b?c*d.bin"), "a_b_c_d.bin");
        assert_eq!(sanitize_filename("name."), "name");
        assert_eq!(sanitize_filename("name  "), "name");
        assert_eq!(sanitize_filename("CON"), "_CON");
        assert_eq!(sanitize_filename("con.txt"), "_con.txt");
        assert_eq!(sanitize_filename("NUL.mkv"), "_NUL.mkv");
        assert_eq!(sanitize_filename("   "), "download.bin");
        assert_eq!(sanitize_filename(""), "download.bin");
    }

    #[test]
    fn sanitize_clamps_length_preserving_extension() {
        let long = format!("{}.bin", "x".repeat(300));
        let out = sanitize_filename(&long);
        assert!(out.chars().count() <= MAX_FILENAME_LEN);
        assert!(out.ends_with(".bin"));
    }

    #[test]
    fn content_disposition_simple_and_quoted() {
        assert_eq!(
            filename_from_content_disposition("attachment; filename=\"report.pdf\""),
            Some("report.pdf".to_string())
        );
        assert_eq!(
            filename_from_content_disposition("attachment; filename=report.pdf"),
            Some("report.pdf".to_string())
        );
    }

    #[test]
    fn content_disposition_utf8_ext_has_priority() {
        let header =
            "attachment; filename=\"fallback.bin\"; filename*=UTF-8''%E4%BD%A0%E5%A5%BD.mkv";
        assert_eq!(
            filename_from_content_disposition(header),
            Some("你好.mkv".to_string())
        );
    }

    #[test]
    fn content_disposition_traversal_is_only_a_name() {
        let header = "attachment; filename=\"../../secret.exe\"";
        assert_eq!(
            filename_from_content_disposition(header),
            Some("../../secret.exe".to_string())
        );
        // resolution sanitizes afterwards:
        assert_eq!(
            resolve_filename(None, Some(header), &"http://x/a".parse().unwrap()).unwrap(),
            "secret.exe"
        );
    }

    #[test]
    fn url_segment_extraction_and_decoding() {
        let url: url::Url = "http://example.com/path/to/My%20File.zip".parse().unwrap();
        assert_eq!(
            resolve_filename(None, None, &url).unwrap(),
            "My File.zip"
        );
        let url: url::Url = "http://example.com/".parse().unwrap();
        assert_eq!(resolve_filename(None, None, &url).unwrap(), "download.bin");
    }

    #[test]
    fn user_filename_wins_and_is_sanitized() {
        let url: url::Url = "http://example.com/server-name.bin".parse().unwrap();
        assert_eq!(
            resolve_filename(Some("  my name.mkv "), None, &url).unwrap(),
            "my name.mkv"
        );
        assert_eq!(
            resolve_filename(Some("../../evil.exe"), None, &url).unwrap(),
            "evil.exe"
        );
        assert!(resolve_filename(Some("   "), None, &url).is_err());
    }
}
