//! Windows path safety for the library (M6(上) §11/§12/§25).
//!
//! P0 rules enforced here:
//! * only canonicalized, EXISTING paths enter the database — a
//!   frontend string can never become a trusted path;
//! * containment is checked COMPONENT-WISE: `C:\Library-Evil` is NOT
//!   inside `C:\Library` (a string `starts_with` check would say it
//!   is);
//! * Windows reserved device names and protected system locations are
//!   rejected as item/root candidates;
//! * reparse points (junctions/symlinks) are detected so the scanner
//!   does not traverse out of the root.

use crate::error::TideError;
use std::path::{Component, Path, PathBuf};

/// Returns the canonical path of `path` (resolves junctions, symlinks,
/// `.`/`..` and separators), or a structured error when the path does
/// not exist or cannot be resolved.
pub fn canonicalize(path: &Path) -> Result<PathBuf, TideError> {
    std::fs::canonicalize(path)
        .map_err(|err| crate::error::TideError::Streaming(crate::error::StreamingFailure::Resource(
            format!("path could not be resolved ({})", io_error_kind(&err)),
        )))
}

fn io_error_kind(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "not found",
        std::io::ErrorKind::PermissionDenied => "permission denied",
        _ => "filesystem error",
    }
}

/// Component-wise containment (§12): `C:\Library-Evil` is NOT inside
/// `C:\Library`; a trailing-separator or case difference must not
/// matter (canonical paths from `canonicalize` are absolute and
/// normalized; Windows canonical paths keep their case, so compare
/// case-insensitively per Windows filesystem semantics).
pub fn is_contained(root: &Path, candidate: &Path) -> bool {
    let mut root_components = root.components().peekable();
    let mut candidate_components = candidate.components().peekable();
    loop {
        match (root_components.next(), candidate_components.next()) {
            (None, _) => return true,
            (Some(_), None) => return false,
            (Some(a), Some(b)) => {
                let a = component_text(&a);
                let b = component_text(&b);
                if !a.eq_ignore_ascii_case(&b) {
                    return false;
                }
            }
        }
    }
}

fn component_text(component: &Component<'_>) -> String {
    match component {
        Component::Prefix(prefix) => prefix.as_os_str().to_string_lossy().to_string(),
        Component::RootDir => "\\".to_string(),
        Component::CurDir => ".".to_string(),
        Component::ParentDir => "..".to_string(),
        Component::Normal(part) => part.to_string_lossy().to_string(),
    }
}

/// Validates a LIBRARY ROOT candidate: must exist, be a directory,
/// be canonicalizable, not be a drive/system location (§13: no
/// automatic C:\ scanning).
pub fn validate_root(path: &Path) -> Result<PathBuf, TideError> {
    let canonical = canonicalize(path)?;
    if !canonical.is_dir() {
        return Err(crate::error::TideError::InvalidInput(
            "library root must be a directory".into(),
        ));
    }
    if is_drive_root(&canonical) {
        return Err(crate::error::TideError::InvalidInput(
            "a whole drive cannot be a library root — choose a subfolder".into(),
        ));
    }
    if is_protected_system_path(&canonical) {
        return Err(crate::error::TideError::Security(
            "system locations cannot be library roots".into(),
        ));
    }
    Ok(canonical)
}

/// Validates a FILE candidate for the library: exists, is a file, is
/// inside the (already validated) root, not a reserved/protected
/// location. Re-validation happens on every use (§11).
pub fn validate_item_path(root: &Path, path: &Path) -> Result<PathBuf, TideError> {
    let canonical = canonicalize(path)?;
    if !canonical.is_file() {
        return Err(crate::error::TideError::InvalidInput(
            "library item must be a regular file".into(),
        ));
    }
    if !is_contained(root, &canonical) {
        return Err(crate::error::TideError::Security(
            "path escapes the library root".into(),
        ));
    }
    if has_reserved_device_name(&canonical) {
        return Err(crate::error::TideError::Security(
            "reserved device names cannot be library items".into(),
        ));
    }
    if is_protected_system_path(&canonical) {
        return Err(crate::error::TideError::Security(
            "protected system files cannot be library items".into(),
        ));
    }
    Ok(canonical)
}

fn is_drive_root(path: &Path) -> bool {
    path.parent().is_none() || path.components().count() <= 2 // prefix + root
}

/// Windows reserved device names (§12): CON PRN AUX NUL COM1-9 LPT1-9,
/// including with an extension ("CON.txt" is reserved too).
pub fn has_reserved_device_name(path: &Path) -> bool {
    let Some(name) = path.file_name() else { return false };
    let name = name.to_string_lossy();
    let stem = name.split('.').next().unwrap_or_default();
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
    ) || (upper.starts_with("COM")
        && upper.len() <= 4
        && upper[3..].chars().all(|c| c.is_ascii_digit())
        && upper.len() > 3)
        || (upper.starts_with("LPT")
            && upper.len() <= 4
            && upper[3..].chars().all(|c| c.is_ascii_digit())
            && upper.len() > 3)
}

/// Protected/system locations (§25): the Windows directory, Program
/// Files, and file targets that would be system-critical. Checked on
/// the CANONICAL path.
pub fn is_protected_system_path(path: &Path) -> bool {
    let system_root = std::env::var("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows"));
    if is_contained(&system_root, path) {
        return true;
    }
    for program_dir in ["Program Files", "Program Files (x86)", "ProgramData"] {
        if let Some(drive) = system_root.ancestors().nth(1) {
            let candidate = drive.join(program_dir);
            if candidate.is_dir() && is_contained(&candidate, path) {
                return true;
            }
        }
    }
    false
}

/// True when the file is a reparse point (junction/symlink). The
/// scanner skips these — following them could escape the root (§12).
pub fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::prelude::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.is_symlink()
    }
}

/// Relative path of `item` under `root`, forward-slash normalized for
/// stable display/storage.
pub fn relative_path(root: &Path, item: &Path) -> String {
    item.strip_prefix(root)
        .map(|rel| {
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|_| {
            item.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containment_is_component_wise() {
        let root = Path::new("C:\\Library");
        assert!(is_contained(root, Path::new("C:\\Library\\Movie.mp4")));
        assert!(is_contained(root, Path::new("C:\\Library")));
        // The §12 trap: string prefix would wrongly accept these.
        assert!(!is_contained(root, Path::new("C:\\Library-Evil\\x.mp4")));
        assert!(!is_contained(root, Path::new("C:\\Libraryary\\x")));
        assert!(!is_contained(root, Path::new("C:\\Windows\\Temp\\x")));
        assert!(!is_contained(root, Path::new("D:\\Library\\x")));
    }

    #[test]
    fn reserved_device_names_rejected() {
        assert!(has_reserved_device_name(Path::new("C:\\x\\CON")));
        assert!(has_reserved_device_name(Path::new("C:\\x\\CON.txt")));
        assert!(has_reserved_device_name(Path::new("C:\\x\\com1")));
        assert!(has_reserved_device_name(Path::new("C:\\x\\LPT2")));
        assert!(has_reserved_device_name(Path::new("C:\\x\\NUL")));
        assert!(!has_reserved_device_name(Path::new("C:\\x\\Comrade.mp4")));
        assert!(!has_reserved_device_name(Path::new("C:\\x\\COM10.txt")));
        assert!(!has_reserved_device_name(Path::new("C:\\x\\movie.mp4")));
    }

    #[test]
    fn windows_directory_is_protected() {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let sys = PathBuf::from(system_root);
        assert!(is_protected_system_path(&sys.join("System32\\x.dll")));
        assert!(!is_protected_system_path(&PathBuf::from("C:\\Users\\someone\\movie.mp4")));
    }

    #[test]
    fn relative_paths_use_forward_slashes() {
        let rel = relative_path(Path::new("C:\\Media"), Path::new("C:\\Media\\a\\b\\c.mp4"));
        assert_eq!(rel, "a/b/c.mp4");
    }
}
