use std::time::{SystemTime, UNIX_EPOCH};

/// Current time as UTC Unix epoch milliseconds.
///
/// Tide stores all timestamps in UTC internally; conversion to the user's
/// local timezone happens only at the UI layer.
pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_after_2026() {
        // 2026-01-01T00:00:00Z
        assert!(now_unix_ms() > 1_767_225_600_000);
    }
}
