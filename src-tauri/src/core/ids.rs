/// Generates a new random identifier (UUID v4).
///
/// Tide never exposes enumerable (incrementing) ids across IPC or, in the
/// future, the local stream gateway.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Checks whether `value` is a well-formed UUID string.
pub fn is_valid_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_is_a_uuid_v4_string() {
        let id = new_id();
        assert_eq!(id.len(), 36);
        assert!(is_valid_id(&id));
    }

    #[test]
    fn new_id_is_random() {
        assert_ne!(new_id(), new_id());
    }

    #[test]
    fn is_valid_id_rejects_non_uuid_input() {
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("1"));
        assert!(!is_valid_id("not-a-uuid"));
        assert!(!is_valid_id("../../etc/passwd"));
    }
}
