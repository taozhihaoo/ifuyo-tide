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
}
