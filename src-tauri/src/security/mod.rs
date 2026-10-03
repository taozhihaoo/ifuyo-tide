//! Input validation at the Rust trust boundary.
//!
//! The frontend is semi-trusted: its checks are UX only. Every value that
//! crosses IPC is (re)validated here before it reaches domain code or
//! persistence.

use crate::error::TideError;

pub const MAX_TASK_NAME_LEN: usize = 200;
pub const MAX_DISPLAY_NAME_LEN: usize = 200;
pub const MAX_PATH_LEN: usize = 1024;

/// Validates a task name coming from user input.
pub fn validate_task_name(raw: &str) -> Result<String, TideError> {
    validate_display_name_impl(raw, "task name")
}

/// Validates a generic display name.
pub fn validate_display_name(raw: &str) -> Result<String, TideError> {
    validate_display_name_impl(raw, "display name")
}

fn validate_display_name_impl(raw: &str, what: &str) -> Result<String, TideError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TideError::InvalidInput(format!("{what} must not be empty")));
    }
    if trimmed.chars().count() > MAX_TASK_NAME_LEN {
        return Err(TideError::InvalidInput(format!(
            "{what} is too long (max {MAX_TASK_NAME_LEN} characters)"
        )));
    }
    if trimmed.chars().any(|c| c.is_control() && c != '\t') {
        return Err(TideError::InvalidInput(format!(
            "{what} must not contain control characters"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates a task/resource identifier crossing IPC.
pub fn validate_id(raw: &str, what: &str) -> Result<String, TideError> {
    if !crate::core::ids::is_valid_id(raw) {
        return Err(TideError::InvalidInput(format!("{what} is not a valid id")));
    }
    Ok(raw.to_string())
}

/// Validates a user-chosen download directory path.
///
/// The path must be absolute; M0 performs no download with it, so this is
/// about honest configuration storage, not yet about fs containment (M1
/// introduces the full path normalizer / containment checks).
pub fn validate_download_dir(raw: &str) -> Result<String, TideError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TideError::InvalidInput(
            "download directory must not be empty".into(),
        ));
    }
    if trimmed.chars().count() > MAX_PATH_LEN {
        return Err(TideError::InvalidInput(
            "download directory path is too long".into(),
        ));
    }
    let path = std::path::Path::new(trimmed);
    if !path.is_absolute() {
        return Err(TideError::InvalidInput(
            "download directory must be an absolute path".into(),
        ));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_name_rules() {
        assert!(validate_task_name("My File.mkv").is_ok());
        assert!(validate_task_name("").is_err());
        assert!(validate_task_name("   ").is_err());
        assert!(validate_task_name("a\u{0}b").is_err());
        assert!(validate_task_name(&"x".repeat(201)).is_err());
        assert_eq!(validate_task_name("  ok  ").unwrap(), "ok");
    }

    #[test]
    fn id_rules() {
        let uuid = crate::core::ids::new_id();
        assert!(validate_id(&uuid, "task_id").is_ok());
        assert!(validate_id("", "task_id").is_err());
        assert!(validate_id("../escape", "task_id").is_err());
    }

    #[test]
    fn download_dir_rules() {
        assert!(validate_download_dir("C:\\Users\\me\\Downloads").is_ok());
        assert!(validate_download_dir("relative/path").is_err());
        assert!(validate_download_dir("").is_err());
    }
}
