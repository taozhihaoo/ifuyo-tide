//! Persistence boundary for torrent tasks and file selection (M3 §37/§65).
//!
//! Stores Tide's product-level view only; librqbit owns piece/fast-resume
//! state. Recovery reconciles the two (see core/recovery).

use crate::error::TideError;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// A persisted torrent task.
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentTaskRecord {
    pub id: String,
    /// Hex info hash; NULL while magnet metadata is unresolved (M3 §7).
    pub info_hash: Option<String>,
    /// `magnet` or `torrent_file`.
    pub source_kind: String,
    pub magnet_uri: Option<String>,
    /// Stable metadata copy: `<app-data>\torrent-metadata\<id>.torrent`.
    pub metadata_path: Option<PathBuf>,
    pub metadata_sha256: Option<String>,
    pub display_name: String,
    pub storage_root: PathBuf,
    pub state: String,
    /// Torrent-specific phase (M3 §7): metadata_pending / downloading /
    /// checking / seeding / completed / metadata_failed / error.
    pub phase: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
    pub selected_count: Option<u32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub recovery_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

/// One torrent file row (selection + identity per M3 §11).
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentFileRecord {
    pub file_index: u32,
    pub info_hash: String,
    pub relative_path: String,
    pub size: u64,
    pub selected: bool,
}

pub struct TorrentRepository {
    conn: Mutex<Connection>,
}

impl TorrentRepository {
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn open(db_path: &Path) -> Result<TorrentRepository, TideError> {
        let mut conn = Connection::open(db_path)
            .map_err(|_| TideError::Persistence("failed to open torrent repository".into()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_millis(2_000))?;
        crate::persistence::migration::run_migrations(&mut conn)?;
        Ok(TorrentRepository {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<TorrentRepository, TideError> {
        let mut conn = Connection::open_in_memory()
            .map_err(|_| TideError::Persistence("failed to open in-memory torrent db".into()))?;
        crate::persistence::migration::run_migrations(&mut conn)?;
        Ok(TorrentRepository {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_task(&self, record: &TorrentTaskRecord) -> Result<(), TideError> {
        self.conn().execute(
            "INSERT INTO torrent_tasks (
                id, info_hash, source_kind, magnet_uri, metadata_path, metadata_sha256,
                display_name, storage_root, state, phase, total_bytes,
                downloaded_bytes, uploaded_bytes, selected_count,
                error_code, error_message, recovery_reason,
                created_at, updated_at, completed_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                record.id,
                record.info_hash,
                record.source_kind,
                record.magnet_uri,
                record.metadata_path.as_ref().map(|p| p.to_string_lossy()),
                record.metadata_sha256,
                record.display_name,
                record.storage_root.to_string_lossy(),
                record.state,
                record.phase,
                record.total_bytes,
                record.downloaded_bytes as i64,
                record.uploaded_bytes as i64,
                record.selected_count.map(|c| c as i64),
                record.error_code,
                record.error_message,
                record.recovery_reason,
                record.created_at,
                record.updated_at,
                record.completed_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TorrentTaskRecord>, TideError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, info_hash, source_kind, magnet_uri, metadata_path, metadata_sha256, \
                        display_name, storage_root, state, phase, total_bytes, downloaded_bytes, \
                        uploaded_bytes, selected_count, error_code, error_message, \
                        recovery_reason, created_at, updated_at, completed_at \
                 FROM torrent_tasks WHERE id = ?1",
                params![id],
                row_to_task,
            )
            .optional()?)
    }

    pub fn list_tasks(&self) -> Result<Vec<TorrentTaskRecord>, TideError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, info_hash, source_kind, magnet_uri, metadata_path, metadata_sha256, \
                    display_name, storage_root, state, phase, total_bytes, downloaded_bytes, \
                    uploaded_bytes, selected_count, error_code, error_message, \
                    recovery_reason, created_at, updated_at, completed_at \
             FROM torrent_tasks ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], row_to_task)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Atomic multi-field task update (M2 §78 style).
    #[allow(clippy::too_many_arguments)]
    pub fn update_task(
        &self,
        id: &str,
        state: &str,
        phase: &str,
        info_hash: Option<&str>,
        total_bytes: Option<u64>,
        downloaded_bytes: u64,
        uploaded_bytes: u64,
        selected_count: Option<u32>,
        error: Option<(&str, &str)>,
        recovery_reason: Option<&str>,
        completed_at: Option<i64>,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE torrent_tasks
             SET state = ?3, phase = ?4, info_hash = ?5, total_bytes = ?6,
                 downloaded_bytes = ?7, uploaded_bytes = ?8, selected_count = ?9,
                 error_code = ?10, error_message = ?11, recovery_reason = ?12,
                 completed_at = ?13, updated_at = ?14
             WHERE id = ?2",
            params![
                id,
                id,
                state,
                phase,
                info_hash,
                total_bytes,
                downloaded_bytes as i64,
                uploaded_bytes as i64,
                selected_count.map(|c| c as i64),
                error.map(|(c, _)| c),
                error.map(|(_, m)| m),
                recovery_reason,
                completed_at,
                now_ms(),
            ],
        )?;
        Ok(())
    }

    pub fn update_metadata_resolution(
        &self,
        id: &str,
        info_hash: &str,
        display_name: &str,
        metadata_path: &Path,
        metadata_sha256: &str,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE torrent_tasks
             SET info_hash = ?2, display_name = ?3, metadata_path = ?4,
                 metadata_sha256 = ?5, phase = 'downloading', updated_at = ?6
             WHERE id = ?1",
            params![
                id,
                info_hash,
                display_name,
                metadata_path.to_string_lossy(),
                metadata_sha256,
                now_ms(),
            ],
        )?;
        Ok(())
    }

    /// Persists magnet metadata resolution (M3 §7): real display name,
    /// total size and selection count once the DHT delivers the
    /// metadata. Phase advances metadata_pending → downloading; a
    /// paused task keeps its paused STATE (the phase still advances —
    /// the engine knows the metadata either way).
    pub fn set_magnet_metadata_resolved(
        &self,
        id: &str,
        display_name: &str,
        total_bytes: u64,
        selected_count: u32,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE torrent_tasks
             SET display_name = ?2, total_bytes = ?3, selected_count = ?4,
                 phase = 'downloading', updated_at = ?5
             WHERE id = ?1",
            params![id, display_name, total_bytes, selected_count, now_ms()],
        )?;
        Ok(())
    }

    pub fn delete_task(&self, id: &str) -> Result<(), TideError> {
        self.conn()
            .execute("DELETE FROM torrent_files WHERE task_id = ?1", params![id])?;
        self.conn()
            .execute("DELETE FROM torrent_tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Replaces the file rows for a task (called once when metadata is
    /// resolved; selection defaults to selected = true, M3 §10).
    pub fn replace_files(
        &self,
        task_id: &str,
        info_hash: &str,
        files: &[TorrentFileRecord],
    ) -> Result<(), TideError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM torrent_files WHERE task_id = ?1",
            params![task_id],
        )?;
        for f in files {
            tx.execute(
                "INSERT INTO torrent_files (task_id, info_hash, file_index, relative_path, size, selected)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    task_id,
                    info_hash,
                    f.file_index as i64,
                    f.relative_path,
                    f.size as i64,
                    f.selected as i64,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_files(&self, task_id: &str) -> Result<Vec<TorrentFileRecord>, TideError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT file_index, info_hash, relative_path, size, selected
             FROM torrent_files WHERE task_id = ?1 ORDER BY file_index ASC",
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Ok(TorrentFileRecord {
                file_index: row.get::<_, i64>(0)? as u32,
                info_hash: row.get(1)?,
                relative_path: row.get(2)?,
                size: row.get::<_, i64>(3)?.max(0) as u64,
                selected: row.get::<_, i64>(4)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn update_selection(
        &self,
        task_id: &str,
        _info_hash: &str,
        selected: &[(u32, bool)],
    ) -> Result<(), TideError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        for (index, is_selected) in selected {
            tx.execute(
                "UPDATE torrent_files SET selected = ?3
                 WHERE task_id = ?1 AND file_index = ?2",
                params![task_id, *index as i64, *is_selected as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn now_ms() -> i64 {
    crate::core::time::now_unix_ms()
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TorrentTaskRecord> {
    Ok(TorrentTaskRecord {
        id: row.get(0)?,
        info_hash: row.get(1)?,
        source_kind: row.get(2)?,
        magnet_uri: row.get(3)?,
        metadata_path: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
        metadata_sha256: row.get(5)?,
        display_name: row.get(6)?,
        storage_root: PathBuf::from(row.get::<_, String>(7)?),
        state: row.get(8)?,
        phase: row.get(9)?,
        total_bytes: row.get(10)?,
        downloaded_bytes: row.get::<_, i64>(11)?.max(0) as u64,
        uploaded_bytes: row.get::<_, i64>(12)?.max(0) as u64,
        selected_count: row.get::<_, Option<i64>>(13)?.map(|v| v.max(0) as u32),
        error_code: row.get(14)?,
        error_message: row.get(15)?,
        recovery_reason: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        completed_at: row.get(19)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, state: &str, phase: &str) -> TorrentTaskRecord {
        TorrentTaskRecord {
            id: id.into(),
            info_hash: Some("a".repeat(40)),
            source_kind: "torrent_file".into(),
            magnet_uri: None,
            metadata_path: None,
            metadata_sha256: None,
            display_name: "sample".into(),
            storage_root: PathBuf::from("C:\\d"),
            state: state.into(),
            phase: phase.into(),
            total_bytes: Some(4096),
            downloaded_bytes: 0,
            uploaded_bytes: 0,
            selected_count: None,
            error_code: None,
            error_message: None,
            recovery_reason: None,
            created_at: 1,
            updated_at: 1,
            completed_at: None,
        }
    }

    #[test]
    fn task_roundtrip() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        repo.insert_task(&sample("t1", "running", "downloading"))
            .unwrap();
        let loaded = repo.get_task("t1").unwrap().unwrap();
        assert_eq!(loaded.state, "running");
        assert_eq!(loaded.phase, "downloading");
        assert_eq!(loaded.info_hash, Some("a".repeat(40)));
    }

    #[test]
    fn magnet_task_allows_null_info_hash() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        let mut r = sample("m1", "running", "metadata_pending");
        r.source_kind = "magnet".into();
        r.magnet_uri = Some("magnet:?xt=urn:btih:x".into());
        r.info_hash = None;
        repo.insert_task(&r).unwrap();
        assert!(repo.get_task("m1").unwrap().unwrap().info_hash.is_none());
    }

    #[test]
    fn update_task_atomic() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        repo.insert_task(&sample("t1", "running", "downloading"))
            .unwrap();
        repo.update_task(
            "t1",
            "completed",
            "completed",
            Some("a".repeat(40).as_str()),
            Some(4096),
            4096,
            128,
            Some(1),
            None,
            None,
            Some(999),
        )
        .unwrap();
        let r = repo.get_task("t1").unwrap().unwrap();
        assert_eq!(r.state, "completed");
        assert_eq!(r.downloaded_bytes, 4096);
        assert_eq!(r.uploaded_bytes, 128);
        assert_eq!(r.completed_at, Some(999));
    }

    #[test]
    fn files_replace_and_selection() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        repo.insert_task(&sample("t1", "running", "downloading"))
            .unwrap();
        let files = vec![
            TorrentFileRecord {
                file_index: 0,
                info_hash: "a".repeat(40),
                relative_path: "a.bin".into(),
                size: 100,
                selected: true,
            },
            TorrentFileRecord {
                file_index: 1,
                info_hash: "a".repeat(40),
                relative_path: "b.bin".into(),
                size: 200,
                selected: true,
            },
        ];
        repo.replace_files("t1", &"a".repeat(40), &files).unwrap();
        repo.update_selection("t1", &"a".repeat(40), &[(1, false)])
            .unwrap();
        let loaded = repo.list_files("t1").unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded[0].selected);
        assert!(!loaded[1].selected);
        // replace wipes previous rows
        repo.replace_files("t1", &"a".repeat(40), &files[..1])
            .unwrap();
        assert_eq!(repo.list_files("t1").unwrap().len(), 1);
    }

    #[test]
    fn delete_cascades_files() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        repo.insert_task(&sample("t1", "running", "downloading"))
            .unwrap();
        repo.replace_files(
            "t1",
            &"a".repeat(40),
            &[TorrentFileRecord {
                file_index: 0,
                info_hash: "a".repeat(40),
                relative_path: "x".into(),
                size: 1,
                selected: true,
            }],
        )
        .unwrap();
        repo.delete_task("t1").unwrap();
        assert!(repo.get_task("t1").unwrap().is_none());
        assert!(repo.list_files("t1").unwrap().is_empty());
    }

    /// M12(1.0.1) regression: pause is a TASK STATE — the schema CHECK
    /// rejects phase='paused', and the pause write must keep the row's
    /// engine phase so the pause actually persists.
    #[test]
    fn pause_writes_preserved_phase_not_phase_paused() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        repo.insert_task(&sample("t1", "running", "downloading"))
            .unwrap();
        // The exact write pause_torrent used to make — CHECK-rejected.
        assert!(repo
            .update_task(
                "t1",
                "paused",
                "paused",
                Some(&"a".repeat(40)),
                Some(4096),
                0,
                0,
                None,
                None,
                None,
                None,
            )
            .is_err());
        // The fixed write: state flips, the engine phase is preserved.
        repo.update_task(
            "t1",
            "paused",
            "downloading",
            Some(&"a".repeat(40)),
            Some(4096),
            0,
            0,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let loaded = repo.get_task("t1").unwrap().unwrap();
        assert_eq!(loaded.state, "paused");
        assert_eq!(loaded.phase, "downloading");
    }

    /// M12(1.0.1): magnet metadata resolution backfill — display name,
    /// total size, selection count and file rows land in the DB, and
    /// the phase advances metadata_pending → downloading.
    #[test]
    fn magnet_metadata_resolution_backfills_row_and_files() {
        let repo = TorrentRepository::open_in_memory().unwrap();
        let mut r = sample("m1", "ready", "metadata_pending");
        r.total_bytes = None; // magnet-shaped: nothing known yet
        repo.insert_task(&r).unwrap();

        repo.set_magnet_metadata_resolved("m1", "Some Release", 8192, 2)
            .unwrap();
        repo.replace_files(
            "m1",
            &"a".repeat(40),
            &[
                TorrentFileRecord {
                    file_index: 0,
                    info_hash: "a".repeat(40),
                    relative_path: "a.mkv".into(),
                    size: 8000,
                    selected: true,
                },
                TorrentFileRecord {
                    file_index: 1,
                    info_hash: "a".repeat(40),
                    relative_path: "b.txt".into(),
                    size: 192,
                    selected: true,
                },
            ],
        )
        .unwrap();

        let loaded = repo.get_task("m1").unwrap().unwrap();
        assert_eq!(loaded.display_name, "Some Release");
        assert_eq!(loaded.total_bytes, Some(8192));
        assert_eq!(loaded.selected_count, Some(2));
        assert_eq!(loaded.phase, "downloading");
        assert_eq!(repo.list_files("m1").unwrap().len(), 2);
    }
}
