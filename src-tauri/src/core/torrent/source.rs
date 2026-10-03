//! Torrent source validation and metadata extraction (M3 §6/§12/§23/§83).
//!
//! Torrent metadata is UNTRUSTED input: every file path inside a torrent
//! is validated before the torrent is accepted. Path components are
//! checked against the same Windows-hostile patterns as HTTP filenames
//! (M1), plus multi-component traversal rejection.

use crate::core::download::url::DownloadFailure;
use crate::error::TideError;
use std::path::PathBuf;

/// A validated torrent source, ready to hand to the engine.
#[derive(Debug, Clone)]
pub enum TorrentSource {
    /// Parsed and validated magnet URI.
    Magnet { uri: String, info_hash: String },
    /// Raw `.torrent` bytes (already read from the imported file).
    TorrentFile { bytes: Vec<u8> },
}

/// Metadata of one file inside a torrent (extracted BEFORE download).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentMetadataFile {
    pub index: usize,
    /// Multi-component relative path inside the torrent (verified safe).
    pub relative_path: String,
    pub size: u64,
    /// Offset inside the torrent's logical byte space (M3 §8: piece
    /// boundaries can cross files — never conflate file i with piece i).
    pub offset: u64,
}

/// Full metadata snapshot (from `.torrent` bytes / resolved metadata).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentMetadata {
    pub info_hash: String,
    pub name: String,
    pub total_size: u64,
    pub piece_length: u32,
    pub files: Vec<TorrentMetadataFile>,
}

const MAX_TORRENT_PATH_LEN: usize = 400;

/// Checks ONE path component of a torrent file path.
fn component_ok(component: &str) -> bool {
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    // Windows drops trailing dots/spaces when creating the file: check
    // the RESERVED list and emptiness against the effective name.
    // M3 §12: trailing dots/spaces are themselves rejection cases for
    // torrent paths (Windows drops them silently — two torrent files
    // could collide on disk).
    let trimmed = component.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() || trimmed.len() != component.len() {
        return false;
    }
    // Reserved device names also match with an extension (NUL.txt is
    // still the NUL device on Windows): check the first dot-separated
    // stem of the trimmed name.
    let stem = trimmed.split('.').next().unwrap_or(trimmed);
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        return false;
    }
    !component
        .chars()
        .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control())
}

/// Validates a torrent-relative file path (multi-component).
///
/// Rejects absolute paths, traversal, drive/UNC/device syntax and
/// per-component abuse. Returns the normalized (kept-as-is) path.
pub fn validate_torrent_relative_path(raw: &str) -> Result<PathBuf, TideError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(TideError::Download(DownloadFailure::InvalidTorrentPath {
            path: raw.to_string(),
        }));
    }
    if raw.starts_with(['/', '\\'])
        || raw.contains(':')
        || raw.starts_with("\\\\")
        || raw.starts_with("\\\\?\\")
    {
        return Err(TideError::Download(DownloadFailure::InvalidTorrentPath {
            path: raw.to_string(),
        }));
    }
    let mut out = PathBuf::new();
    for component in raw.split(['/', '\\']) {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            return Err(TideError::Download(DownloadFailure::InvalidTorrentPath {
                path: raw.to_string(),
            }));
        }
        if !component_ok(component) {
            return Err(TideError::Download(DownloadFailure::InvalidTorrentPath {
                path: raw.to_string(),
            }));
        }
        out.push(component);
    }
    if out.as_os_str().is_empty() {
        return Err(TideError::Download(DownloadFailure::InvalidTorrentPath {
            path: raw.to_string(),
        }));
    }
    let joined = out.to_string_lossy().to_string();
    if joined.chars().count() > MAX_TORRENT_PATH_LEN {
        return Err(TideError::Download(DownloadFailure::InvalidTorrentPath {
            path: raw.to_string(),
        }));
    }
    Ok(out)
}

/// Validates every file path of a torrent; a single violation rejects
/// the whole torrent (M3 §12).
pub fn validate_torrent_paths(paths: &[String]) -> Result<(), TideError> {
    for path in paths {
        validate_torrent_relative_path(path).map_err(|_| {
            TideError::Download(
                crate::core::download::url::DownloadFailure::InvalidTorrentPath {
                    path: path.clone(),
                },
            )
        })?;
    }
    Ok(())
}

/// Validates a magnet URI and extracts the identity fields Tide needs.
/// Parsing itself is delegated to librqbit (M3 §23 — no custom parser).
pub fn validate_magnet_uri(raw: &str) -> Result<(String, String, Option<String>), TideError> {
    let raw = raw.trim();
    if raw.len() > 8 * 1024 {
        return Err(TideError::Download(
            crate::core::download::url::DownloadFailure::InvalidUrl {
                reason: "magnet URI too long".into(),
            },
        ));
    }
    let magnet = librqbit::Magnet::parse(raw).map_err(|_| {
        TideError::Download(crate::core::download::url::DownloadFailure::InvalidUrl {
            reason: "malformed magnet URI".into(),
        })
    })?;
    let info_hash = magnet
        .as_id20()
        .ok_or_else(|| {
            TideError::Download(crate::core::download::url::DownloadFailure::InvalidUrl {
                reason: "magnet has no v1 info hash".into(),
            })
        })?
        .as_string();
    if raw.len() < "magnet:?xt=".len() {
        return Err(TideError::Download(
            crate::core::download::url::DownloadFailure::InvalidUrl {
                reason: "magnet URI truncated".into(),
            },
        ));
    }
    Ok((raw.to_string(), info_hash, magnet.name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_absolute() {
        for bad in [
            "../foo",
            "..\\foo",
            "C:\\foo",
            "C:/foo",
            "\\\\server\\share\\f",
            "\\\\?\\C:\\foo",
            "a/../b",
            "foo/../../bar",
            "foo:/bar",
        ] {
            assert!(
                validate_torrent_relative_path(bad).is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_reserved_and_abusive_components() {
        assert!(validate_torrent_relative_path("CON").is_err());
        assert!(validate_torrent_relative_path("nul.txt").is_err());
        assert!(validate_torrent_relative_path("COM1.dat").is_err());
        assert!(validate_torrent_relative_path("dir/aux/b").is_err());
        assert!(validate_torrent_relative_path("trailing. ").is_err());
        assert!(validate_torrent_relative_path("a<b>.bin").is_err());
    }

    #[test]
    fn accepts_normal_paths() {
        assert_eq!(
            validate_torrent_relative_path("movie.mkv").unwrap(),
            PathBuf::from("movie.mkv")
        );
        assert_eq!(
            validate_torrent_relative_path("folder/sub/data.dat").unwrap(),
            PathBuf::from("folder").join("sub").join("data.dat")
        );
        // dots INSIDE a component are fine
        assert!(validate_torrent_relative_path("some.file.name.mkv").is_ok());
        // unicode is fine
        assert!(validate_torrent_relative_path("é»).mkv").is_ok());
    }

    #[test]
    fn validate_all_paths_rejects_whole_torrent() {
        assert!(validate_torrent_paths(&["ok.bin".into()]).is_ok());
        assert!(validate_torrent_paths(&["ok.bin".into(), "../bad".into()]).is_err());
    }

    #[test]
    fn magnet_validation_shapes() {
        // malformed
        assert!(validate_magnet_uri("http://x").is_err());
        assert!(validate_magnet_uri("magnet:").is_err());
        assert!(validate_magnet_uri("magnet:?dn=only-name").is_err());
        // valid (checksum-free, xt btih)
        let (uri, hash, name) = validate_magnet_uri(
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=test",
        )
        .unwrap();
        assert_eq!(hash, "0123456789abcdef0123456789abcdef01234567");
        assert_eq!(name.as_deref(), Some("test"));
        assert!(uri.starts_with("magnet:?xt="));
    }
}
