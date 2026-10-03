//! Persistence boundary for download tasks (M2 §18/§19).
//!
//! Owns its own SQLite connection (WAL allows one writer + readers); the
//! engine's checkpoint writes never contend with the M0 app-state
//! connection. The repository stores LOGICAL state only — resume
//! decisions stay in the recovery/engine layer (M1 §19).

use crate::error::TideError;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Mutex, PoisonError};
use std::path::{Path, PathBuf};

/// A persisted download task row mapped to the domain vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadTaskRecord {
    pub id: String,
    pub source_url: String,
    pub allow_private: bool,
    pub destination_dir: PathBuf,
    pub filename: String,
    pub part_path: PathBuf,
    pub final_path: PathBuf,
    pub overwrite_policy: String,
    /// M0 lifecycle state string (`ready`, `paused`, `running`, …).
    pub state: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum_expected: Option<String>,
    pub retry_count: u32,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    /// Why the last recovery made its decision (M2 §96 reason codes).
    pub recovery_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DownloadCheckpoint {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// The connection sits behind a Mutex: rusqlite connections are !Sync,
/// and the engine checkpoints from arbitrary tokio workers.
pub struct DownloadRepository {
    conn: Mutex<Connection>,
}

fn now_ms() -> i64 {
    crate::core::time::now_unix_ms()
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<DownloadTaskRecord> {
    Ok(DownloadTaskRecord {
        id: row.get(0)?,
        source_url: row.get(1)?,
        allow_private: row.get::<_, i64>(2)? != 0,
        destination_dir: PathBuf::from(row.get::<_, String>(3)?),
        filename: row.get(4)?,
        part_path: PathBuf::from(row.get::<_, String>(5)?),
        final_path: PathBuf::from(row.get::<_, String>(6)?),
        overwrite_policy: row.get(7)?,
        state: row.get(8)?,
        total_bytes: row.get(9)?,
        downloaded_bytes: row.get::<_, i64>(10)?.max(0) as u64,
        content_type: row.get(11)?,
        etag: row.get(12)?,
        last_modified: row.get(13)?,
        checksum_algorithm: row.get(14)?,
        checksum_expected: row.get(15)?,
        retry_count: row.get::<_, i64>(16)?.max(0) as u32,
        last_error_code: row.get(17)?,
        last_error_message: row.get(18)?,
        recovery_reason: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
        started_at: row.get(22)?,
        completed_at: row.get(23)?,
    })
}

const RECORD_COLUMNS: &str = "id, source_url, allow_private, destination_dir, filename, \
     part_path, final_path, overwrite_policy, state, total_bytes, downloaded_bytes, \
     content_type, etag, last_modified, checksum_algorithm, checksum_expected, \
     retry_count, last_error_code, last_error_message, recovery_reason, \
     created_at, updated_at, started_at, completed_at";

impl DownloadRepository {
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // Poison recovery: rows are plain data; a panicked task must not
        // brick persistence.
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Opens the repository against the app database file. Migrations are
    /// re-run here idempotently: this connection may be the first to open
    /// the database file.
    pub fn open(db_path: &Path) -> Result<DownloadRepository, TideError> {
        let mut conn = Connection::open(db_path)
            .map_err(|_| TideError::Persistence("failed to open download repository".into()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_millis(2_000))?;
        crate::persistence::migration::run_migrations(&mut conn)?;
        Ok(DownloadRepository {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory repository for tests.
    pub fn open_in_memory() -> Result<DownloadRepository, TideError> {
        let mut conn = Connection::open_in_memory()
            .map_err(|_| TideError::Persistence("failed to open in-memory database".into()))?;
        crate::persistence::migration::run_migrations(&mut conn)?;
        Ok(DownloadRepository {
            conn: Mutex::new(conn),
        })
    }

    /// Inserts a new task record. Fails on duplicate id.
    pub fn insert(&self, record: &DownloadTaskRecord) -> Result<(), TideError> {
        self.conn()
            .execute(
                "INSERT INTO download_tasks (
                    id, source_url, allow_private, destination_dir, filename,
                    part_path, final_path, overwrite_policy, state,
                    total_bytes, downloaded_bytes, content_type, etag, last_modified,
                    checksum_algorithm, checksum_expected, retry_count,
                    last_error_code, last_error_message, recovery_reason,
                    created_at, updated_at, started_at, completed_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
                params![
                    record.id,
                    record.source_url,
                    record.allow_private as i64,
                    record.destination_dir.to_string_lossy(),
                    record.filename,
                    record.part_path.to_string_lossy(),
                    record.final_path.to_string_lossy(),
                    record.overwrite_policy,
                    record.state,
                    record.total_bytes,
                    record.downloaded_bytes as i64,
                    record.content_type,
                    record.etag,
                    record.last_modified,
                    record.checksum_algorithm,
                    record.checksum_expected,
                    record.retry_count as i64,
                    record.last_error_code,
                    record.last_error_message,
                    record.recovery_reason,
                    record.created_at,
                    record.updated_at,
                    record.started_at,
                    record.completed_at,
                ],
            )
            .map_err(|err| match &err {
                rusqlite::Error::SqliteFailure(
                    e,
                    _,
                ) if e.code == rusqlite::ffi::ErrorCode::ConstraintViolation => {
                    TideError::InvalidInput("download task already exists".into())
                }
                _ => TideError::from(err),
            })?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<DownloadTaskRecord>, TideError> {
        let record = self
            .conn()
            .query_row(
                &format!("SELECT {RECORD_COLUMNS} FROM download_tasks WHERE id = ?1"),
                params![id],
                row_to_record,
            )
            .optional()?;
        Ok(record)
    }

    pub fn list(&self) -> Result<Vec<DownloadTaskRecord>, TideError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {RECORD_COLUMNS} FROM download_tasks ORDER BY created_at ASC"
        ))?;
        let rows = stmt.query_map([], row_to_record)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Time/byte-throttled checkpoint (M2 §20-§23): single-statement
    /// atomic update of progress + validators. Written AFTER the bytes
    /// are durably in the .part file, so DB ≤ file holds.
    pub fn update_checkpoint(&self, checkpoint: &DownloadCheckpoint, id: &str) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE download_tasks
             SET downloaded_bytes = ?2, total_bytes = ?3, etag = ?4,
                 last_modified = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                id,
                checkpoint.downloaded_bytes as i64,
                checkpoint.total_bytes,
                checkpoint.etag,
                checkpoint.last_modified,
                now_ms(),
            ],
        )?;
        Ok(())
    }

    /// Atomic multi-field state update (M2 §78): lifecycle state,
    /// progress, error and recovery reason change together.
    #[allow(clippy::too_many_arguments)]
    pub fn update_state(
        &self,
        id: &str,
        state: &str,
        downloaded_bytes: u64,
        error: Option<(&str, &str)>,
        recovery_reason: Option<&str>,
        completed_at: Option<i64>,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE download_tasks
             SET state = ?2, downloaded_bytes = ?3,
                 last_error_code = ?4, last_error_message = ?5,
                 recovery_reason = ?6, completed_at = ?7, updated_at = ?8
             WHERE id = ?1",
            params![
                id,
                state,
                downloaded_bytes as i64,
                error.map(|(c, _)| c),
                error.map(|(_, m)| m),
                recovery_reason,
                completed_at,
                now_ms(),
            ],
        )?;
        Ok(())
    }

    pub fn update_retry_count(&self, id: &str, retry_count: u32) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE download_tasks SET retry_count = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, retry_count as i64, now_ms()],
        )?;
        Ok(())
    }

    /// Records the resolved filename/paths after the first response
    /// (before this, recovery cannot locate artifacts — the row is
    /// updated in place, id never changes, M2 §14).
    pub fn update_resolved_paths(
        &self,
        id: &str,
        filename: &str,
        part_path: &Path,
        final_path: &Path,
        content_type: Option<&str>,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE download_tasks
             SET filename = ?2, part_path = ?3, final_path = ?4,
                 content_type = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                id,
                filename,
                part_path.to_string_lossy(),
                final_path.to_string_lossy(),
                content_type,
                now_ms(),
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), TideError> {
        self.conn()
            .execute("DELETE FROM download_tasks WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, state: &str) -> DownloadTaskRecord {
        DownloadTaskRecord {
            id: id.to_string(),
            source_url: "http://example.com/file.bin".into(),
            allow_private: false,
            destination_dir: PathBuf::from("C:\\downloads"),
            filename: "file.bin".into(),
            part_path: PathBuf::from("C:\\downloads\\file.bin.part"),
            final_path: PathBuf::from("C:\\downloads\\file.bin"),
            overwrite_policy: "auto_rename".into(),
            state: state.into(),
            total_bytes: Some(1000),
            downloaded_bytes: 0,
            content_type: Some("application/octet-stream".into()),
            etag: Some("\"v1\"".into()),
            last_modified: None,
            checksum_algorithm: Some("sha256".into()),
            checksum_expected: Some("ab".repeat(32)),
            retry_count: 0,
            last_error_code: None,
            last_error_message: None,
            recovery_reason: None,
            created_at: 1000,
            updated_at: 1000,
            started_at: None,
            completed_at: None,
        }
    }

    #[test]
    fn insert_get_roundtrip() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        let record = sample("t1", "ready");
        repo.insert(&record).unwrap();
        let loaded = repo.get("t1").unwrap().expect("exists");
        assert_eq!(loaded, record);
        assert!(repo.get("missing").unwrap().is_none());
    }

    #[test]
    fn insert_duplicate_id_fails() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        repo.insert(&sample("t1", "ready")).unwrap();
        assert!(repo.insert(&sample("t1", "ready")).is_err());
    }

    #[test]
    fn list_orders_by_created_at() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        let mut late = sample("late", "ready");
        late.created_at = 2000;
        repo.insert(&sample("early", "ready")).unwrap();
        repo.insert(&late).unwrap();
        let ids: Vec<String> = repo.list().unwrap().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, vec!["early", "late"]);
    }

    #[test]
    fn checkpoint_updates_progress_and_validators() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        repo.insert(&sample("t1", "running")).unwrap();
        repo.update_checkpoint(
            &DownloadCheckpoint {
                downloaded_bytes: 4096,
                total_bytes: Some(8192),
                etag: Some("\"v2\"".into()),
                last_modified: Some("yesterday".into()),
            },
            "t1",
        )
        .unwrap();
        let r = repo.get("t1").unwrap().unwrap();
        assert_eq!(r.downloaded_bytes, 4096);
        assert_eq!(r.total_bytes, Some(8192));
        assert_eq!(r.etag.as_deref(), Some("\"v2\""));
        assert_eq!(r.state, "running");
    }

    #[test]
    fn state_update_is_atomic_multi_field() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        repo.insert(&sample("t1", "running")).unwrap();
        repo.update_state("t1", "paused", 1234, Some(("DOWNLOAD_TIMEOUT", "timed out")), None, None)
            .unwrap();
        let r = repo.get("t1").unwrap().unwrap();
        assert_eq!(r.state, "paused");
        assert_eq!(r.downloaded_bytes, 1234);
        assert_eq!(r.last_error_code.as_deref(), Some("DOWNLOAD_TIMEOUT"));
    }

    #[test]
    fn delete_removes_record() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        repo.insert(&sample("t1", "failed")).unwrap();
        repo.delete("t1").unwrap();
        assert!(repo.get("t1").unwrap().is_none());
    }

    #[test]
    fn unknown_total_stays_null_not_zero() {
        let repo = DownloadRepository::open_in_memory().unwrap();
        let mut record = sample("t1", "running");
        record.total_bytes = None; // chunked / unknown length (M2 §62)
        repo.insert(&record).unwrap();
        assert_eq!(repo.get("t1").unwrap().unwrap().total_bytes, None);
    }
}
