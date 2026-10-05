//! Persistence boundary for application settings.
//!
//! Settings are flat key/value rows; M0 stores only the download
//! directory. Values are validated by the security layer before they
//! reach this repository.

use crate::error::TideError;
use rusqlite::{params, Connection};

pub const KEY_DOWNLOAD_DIR: &str = "download_dir";

#[derive(Debug, Default, Clone, Copy)]
pub struct SettingsRepository;

impl SettingsRepository {
    pub fn get(&self, conn: &Connection, key: &str) -> Result<Option<String>, TideError> {
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(result)
    }

    /// All key/value pairs whose key starts with `prefix` (M7: the
    /// `sched:` namespace for user priorities + bandwidth prefs).
    pub fn get_by_prefix(
        &self,
        conn: &Connection,
        prefix: &str,
    ) -> Result<Vec<(String, String)>, TideError> {
        // Escape LIKE wildcards in the prefix itself and declare a
        // single-character ESCAPE. (`ESCAPE '\'` in a Rust literal is
        // `ESCAPE ''` on the wire — `\'` escapes the quote — which
        // SQLite rejects and silently disabled preference loading.)
        let escaped = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let mut stmt = conn.prepare(
            "SELECT key, value FROM settings WHERE key LIKE ?1 ESCAPE '\\' ORDER BY key",
        )?;
        let rows = stmt.query_map([format!("{escaped}%")], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn delete(&self, conn: &Connection, key: &str) -> Result<(), TideError> {
        conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    pub fn set(&self, conn: &Connection, key: &str, value: &str) -> Result<(), TideError> {
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, crate::core::time::now_unix_ms()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::open_in_memory;

    #[test]
    fn set_then_get_roundtrips() {
        let conn = open_in_memory().unwrap();
        let repo = SettingsRepository;
        assert_eq!(repo.get(&conn, KEY_DOWNLOAD_DIR).unwrap(), None);
        repo.set(&conn, KEY_DOWNLOAD_DIR, "C:\\Downloads\\Tide")
            .unwrap();
        assert_eq!(
            repo.get(&conn, KEY_DOWNLOAD_DIR).unwrap(),
            Some("C:\\Downloads\\Tide".to_string())
        );
        repo.set(&conn, KEY_DOWNLOAD_DIR, "D:\\Tide").unwrap();
        assert_eq!(
            repo.get(&conn, KEY_DOWNLOAD_DIR).unwrap(),
            Some("D:\\Tide".to_string())
        );
    }

    #[test]
    fn unknown_key_is_none_not_error() {
        let conn = open_in_memory().unwrap();
        let repo = SettingsRepository;
        assert_eq!(repo.get(&conn, "no_such_key").unwrap(), None);
    }

    /// 1.0.2 regression: get_by_prefix used to ship `ESCAPE ''` (the
    /// Rust literal `ESCAPE '''` escapes the quote), so the statement
    /// failed to prepare and scheduler preferences NEVER loaded after
    /// a restart. Prefixes containing LIKE wildcards must also survive.
    #[test]
    fn get_by_prefix_loads_and_escapes_wildcards() {
        let conn = open_in_memory().unwrap();
        let repo = SettingsRepository;
        repo.set(&conn, "sched:task-a", "high").unwrap();
        repo.set(&conn, "sched:task-b", "normal").unwrap();
        repo.set(&conn, "other:key", "x").unwrap();

        let rows = repo.get_by_prefix(&conn, "sched:").unwrap();
        assert_eq!(rows.len(), 2);

        // A prefix containing wildcards must match literally, not expand.
        repo.set(&conn, "weird%key", "v").unwrap();
        let rows = repo.get_by_prefix(&conn, "weird%").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "weird%key");
    }
}
