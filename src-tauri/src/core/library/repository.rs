//! Library persistence boundary (M6(上) §29/§31/§32).
//!
//! ALL library SQL lives here: the scanner and the service never touch
//! a Connection. The scan commit is ONE transaction — a crashed scan
//! can never leave half-upserted state (§67).

use super::model::{
    AvailabilityStatus, ItemPage, ItemQuery, LibraryItem, LibraryRoot, MediaCategory, OriginType,
};
use crate::error::TideError;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A file discovered by a scan (already validated + classified).
pub struct ScanCandidate {
    pub canonical_path: PathBuf,
    pub relative_path: String,
    pub display_name: String,
    pub extension: Option<String>,
    pub media_category: MediaCategory,
    pub mime_type: Option<String>,
    pub size_bytes: u64,
    pub modified_at: i64,
}

pub struct LibraryRepository {
    conn: Mutex<Connection>,
}

fn now_ms() -> i64 {
    crate::core::time::now_unix_ms()
}

impl LibraryRepository {
    pub fn open(db_path: &Path) -> Result<LibraryRepository, TideError> {
        let mut conn = Connection::open(db_path)
            .map_err(|_| TideError::Persistence("failed to open library repository".into()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_millis(2_000))?;
        crate::persistence::migration::run_migrations(&mut conn)?;
        Ok(LibraryRepository {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<LibraryRepository, TideError> {
        let mut conn = Connection::open_in_memory()
            .map_err(|_| TideError::Persistence("failed to open in-memory library db".into()))?;
        crate::persistence::migration::run_migrations(&mut conn)?;
        Ok(LibraryRepository {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    // ----- roots --------------------------------------------------------

    pub fn insert_root(&self, root: &LibraryRoot) -> Result<(), TideError> {
        self.conn().execute(
            "INSERT INTO library_roots (id, path, display_name, enabled, scan_status, last_scan_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                root.id,
                root.path.to_string_lossy(),
                root.display_name,
                root.enabled as i64,
                root.scan_status,
                root.last_scan_at,
                root.created_at,
                root.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_root(&self, id: &str) -> Result<Option<LibraryRoot>, TideError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, path, display_name, enabled, scan_status, last_scan_at, created_at, updated_at
                 FROM library_roots WHERE id = ?1",
                params![id],
                row_to_root,
            )
            .optional()?)
    }

    pub fn get_root_by_path(&self, path: &Path) -> Result<Option<LibraryRoot>, TideError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, path, display_name, enabled, scan_status, last_scan_at, created_at, updated_at
                 FROM library_roots WHERE path = ?1 COLLATE NOCASE",
                params![path.to_string_lossy()],
                row_to_root,
            )
            .optional()?)
    }

    pub fn list_roots(&self) -> Result<Vec<LibraryRoot>, TideError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, path, display_name, enabled, scan_status, last_scan_at, created_at, updated_at
             FROM library_roots ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], row_to_root)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn set_root_enabled(&self, id: &str, enabled: bool) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE library_roots SET enabled = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, enabled as i64, now_ms()],
        )?;
        Ok(())
    }

    pub fn update_root_scan_status(
        &self,
        id: &str,
        status: &str,
        last_scan_at: Option<i64>,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE library_roots SET scan_status = ?2, last_scan_at = COALESCE(?3, last_scan_at), updated_at = ?4
             WHERE id = ?1",
            params![id, status, last_scan_at, now_ms()],
        )?;
        Ok(())
    }

    /// Removes the root AND its items (the scan index of a removed
    /// root is meaningless). Files on disk are never touched here.
    pub fn remove_root(&self, id: &str) -> Result<(), TideError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM library_items WHERE root_id = ?1", params![id])?;
        tx.execute("DELETE FROM library_roots WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    // ----- scan commit (§21/§67/§68) ------------------------------------

    /// One transaction: upsert every discovered candidate, then mark
    /// previously-available items as Missing EXCEPT those under
    /// `uncovered_prefixes` (§68/§69: only directories the scan really
    /// covered may produce Missing verdicts). Idempotent: re-running
    /// with the same candidates changes nothing but timestamps.
    pub fn apply_scan(
        &self,
        root_id: &str,
        candidates: &[ScanCandidate],
        uncovered_prefixes: &[PathBuf],
    ) -> Result<(u64, u64), TideError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "CREATE TEMP TABLE IF NOT EXISTS scan_seen (path TEXT PRIMARY KEY)",
            [],
        )?;
        tx.execute("DELETE FROM scan_seen", [])?;
        let now = now_ms();
        let mut added = 0u64;
        for c in candidates {
            let path_str = c.canonical_path.to_string_lossy();
            let id = crate::core::ids::new_id();
            // Existence is checked BEFORE the upsert: after it, the
            // just-written row would always count as "existing", and
            // same-millisecond scans would defeat a created_at
            // comparison.
            let exists_before = tx
                .query_row(
                    "SELECT COUNT(*) FROM library_items WHERE root_id = ?1 AND canonical_path = ?2",
                    params![root_id, path_str],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);
            if exists_before == 0 {
                added += 1;
            }
            tx.execute(
                "INSERT INTO library_items (
                    id, root_id, canonical_path, relative_path, display_name, extension,
                    media_category, mime_type, size_bytes, modified_at,
                    availability_status, origin_type, task_id, checksum,
                    last_verified_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'available', 'imported_local', NULL, NULL, ?11, ?11, ?11)
                 ON CONFLICT (root_id, canonical_path) DO UPDATE SET
                    relative_path = excluded.relative_path,
                    display_name = excluded.display_name,
                    extension = excluded.extension,
                    media_category = excluded.media_category,
                    mime_type = excluded.mime_type,
                    size_bytes = excluded.size_bytes,
                    modified_at = excluded.modified_at,
                    availability_status = 'available',
                    last_verified_at = excluded.last_verified_at,
                    updated_at = excluded.updated_at",
                params![
                    id,
                    root_id,
                    path_str,
                    c.relative_path,
                    c.display_name,
                    c.extension,
                    c.media_category.as_str(),
                    c.mime_type,
                    c.size_bytes as i64,
                    c.modified_at,
                    now,
                ],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO scan_seen (path) VALUES (?1)",
                params![path_str],
            )?;
        }

        // Missing marking, excluding uncovered prefixes (§69): only
        // items under directories this scan really covered may be
        // declared Missing.
        let missing;
        if uncovered_prefixes.is_empty() {
            missing = tx.execute(
                "UPDATE library_items SET availability_status = 'missing', updated_at = ?2
                 WHERE root_id = ?1 AND availability_status = 'available'
                 AND canonical_path NOT IN (SELECT path FROM scan_seen)",
                params![root_id, now],
            )? as u64;
        } else {
            let mut sql = String::from(
                "UPDATE library_items SET availability_status = 'missing', updated_at = ?2
                 WHERE root_id = ?1 AND availability_status = 'available'
                 AND canonical_path NOT IN (SELECT path FROM scan_seen)",
            );
            for i in 0..uncovered_prefixes.len() {
                sql.push_str(&format!(
                    " AND canonical_path NOT LIKE ?{} ESCAPE '\\'",
                    i + 3
                ));
            }
            let mut stmt = tx.prepare(&sql)?;
            let mut all_params: Vec<Box<dyn rusqlite::ToSql>> =
                vec![Box::new(root_id.to_string()), Box::new(now)];
            for prefix in uncovered_prefixes {
                // LIKE wildcard is % (not GLOB's *): prefix + anything.
                all_params.push(Box::new(format!(
                    "{}%",
                    escape_like(&prefix.to_string_lossy())
                )));
            }
            missing = stmt.execute(rusqlite::params_from_iter(
                all_params.iter().map(|p| p.as_ref()),
            ))? as u64;
        }

        tx.execute("DELETE FROM scan_seen", [])?;
        tx.commit()?;
        Ok((added, missing))
    }

    // ----- items --------------------------------------------------------

    pub fn get_item(&self, id: &str) -> Result<Option<LibraryItem>, TideError> {
        Ok(self
            .conn()
            .query_row(&item_select("WHERE i.id = ?1"), params![id], row_to_item)
            .optional()?)
    }

    pub fn insert_imported_item(&self, item: &LibraryItem) -> Result<(), TideError> {
        self.conn().execute(
            "INSERT INTO library_items (
                id, root_id, canonical_path, relative_path, display_name, extension,
                media_category, mime_type, size_bytes, modified_at,
                availability_status, origin_type, task_id, checksum,
                last_verified_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
             ON CONFLICT (root_id, canonical_path) DO UPDATE SET
                availability_status = 'available',
                origin_type = excluded.origin_type,
                task_id = excluded.task_id,
                checksum = COALESCE(excluded.checksum, library_items.checksum),
                size_bytes = excluded.size_bytes,
                modified_at = excluded.modified_at,
                last_verified_at = excluded.last_verified_at,
                updated_at = excluded.updated_at",
            params![
                item.id,
                item.root_id,
                item.canonical_path.to_string_lossy(),
                item.relative_path,
                item.display_name,
                item.extension,
                item.media_category.as_str(),
                item.mime_type,
                item.size_bytes as i64,
                item.modified_at,
                item.availability.as_str(),
                item.origin.as_str(),
                item.task_id,
                item.checksum,
                item.last_verified_at,
                item.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn set_item_availability(
        &self,
        id: &str,
        status: AvailabilityStatus,
    ) -> Result<(), TideError> {
        self.conn().execute(
            "UPDATE library_items SET availability_status = ?2, last_verified_at = ?3, updated_at = ?3
             WHERE id = ?1",
            params![id, status.as_str(), now_ms()],
        )?;
        Ok(())
    }

    /// Removes the index entry only — never touches the file (§24).
    pub fn remove_item(&self, id: &str) -> Result<bool, TideError> {
        let affected = self
            .conn()
            .execute("DELETE FROM library_items WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_item_paths_of_root(
        &self,
        root_id: &str,
    ) -> Result<Vec<(String, PathBuf)>, TideError> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT id, canonical_path FROM library_items WHERE root_id = ?1")?;
        let rows = stmt.query_map(params![root_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PathBuf::from(row.get::<_, String>(1)?),
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_item_by_path(
        &self,
        canonical_path: &Path,
    ) -> Result<Option<LibraryItem>, TideError> {
        Ok(self
            .conn()
            .query_row(
                &item_select("WHERE i.canonical_path = ?1 COLLATE NOCASE"),
                params![canonical_path.to_string_lossy()],
                row_to_item,
            )
            .optional()?)
    }

    pub fn item_id_by_task_and_path(
        &self,
        task_id: &str,
        path: &Path,
    ) -> Result<Option<String>, TideError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id FROM library_items WHERE task_id = ?1 AND canonical_path = ?2 COLLATE NOCASE",
                params![task_id, path.to_string_lossy()],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// §41–§45: paginated, filtered, sorted in SQL.
    pub fn list_items(&self, query: &ItemQuery) -> Result<ItemPage, TideError> {
        let conn = self.conn();
        let mut where_clause = String::from("1 = 1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(search) = &query.search {
            params_vec.push(Box::new(format!("%{}%", escape_like(search))));
            where_clause.push_str(&format!(
                " AND (i.display_name LIKE ?{} ESCAPE '\\' OR i.relative_path LIKE ?{} ESCAPE '\\')",
                params_vec.len(),
                params_vec.len()
            ));
        }
        if let Some(category) = &query.category {
            params_vec.push(Box::new(category.as_str().to_string()));
            where_clause.push_str(&format!(" AND i.media_category = ?{}", params_vec.len()));
        }
        if let Some(availability) = &query.availability {
            params_vec.push(Box::new(availability.as_str().to_string()));
            where_clause.push_str(&format!(
                " AND i.availability_status = ?{}",
                params_vec.len()
            ));
        }
        if let Some(origin) = &query.origin {
            params_vec.push(Box::new(origin.as_str().to_string()));
            where_clause.push_str(&format!(" AND i.origin_type = ?{}", params_vec.len()));
        }

        let order = match query.sort.as_deref() {
            Some("size") => "size_bytes",
            Some("modified") => "modified_at",
            Some("added") => "created_at",
            _ => "display_name COLLATE NOCASE",
        };
        let direction = if query.ascending { "ASC" } else { "DESC" };
        let page_size = query.page_size.clamp(1, 200) as i64;
        let offset = (query.page.saturating_mul(page_size as u32)) as i64;

        let count_sql = format!("SELECT COUNT(*) FROM library_items i WHERE {where_clause}");
        let total: u64 = {
            let mut stmt = conn.prepare(&count_sql)?;
            stmt.query_row(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |row| row.get::<_, i64>(0),
            )? as u64
        };

        let sql = format!(
            "{} WHERE {where_clause} ORDER BY {order} {direction} LIMIT {page_size} OFFSET {offset}",
            item_select_as_table()
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
            row_to_item,
        )?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(ItemPage {
            items,
            total,
            page: query.page,
            page_size: page_size as u32,
        })
    }
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn item_select(condition: &str) -> String {
    format!(
        "SELECT i.id, i.root_id, i.canonical_path, i.relative_path, i.display_name, i.extension, \
                i.media_category, i.mime_type, i.size_bytes, i.modified_at, \
                i.availability_status, i.origin_type, i.task_id, i.checksum, \
                i.last_verified_at, i.created_at, i.updated_at \
         FROM library_items i {condition}"
    )
}

fn item_select_as_table() -> String {
    "SELECT i.id, i.root_id, i.canonical_path, i.relative_path, i.display_name, i.extension, \
            i.media_category, i.mime_type, i.size_bytes, i.modified_at, \
            i.availability_status, i.origin_type, i.task_id, i.checksum, \
            i.last_verified_at, i.created_at, i.updated_at \
     FROM library_items i"
        .to_string()
}

fn row_to_root(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryRoot> {
    Ok(LibraryRoot {
        id: row.get(0)?,
        path: PathBuf::from(row.get::<_, String>(1)?),
        display_name: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        scan_status: row.get(4)?,
        last_scan_at: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryItem> {
    Ok(LibraryItem {
        id: row.get(0)?,
        root_id: row.get(1)?,
        canonical_path: PathBuf::from(row.get::<_, String>(2)?),
        relative_path: row.get(3)?,
        display_name: row.get(4)?,
        extension: row.get(5)?,
        media_category: MediaCategory::parse(&row.get::<_, String>(6)?)
            .unwrap_or(MediaCategory::Other),
        mime_type: row.get(7)?,
        size_bytes: row.get::<_, i64>(8)?.max(0) as u64,
        modified_at: row.get(9)?,
        availability: AvailabilityStatus::parse(&row.get::<_, String>(10)?)
            .unwrap_or(AvailabilityStatus::Inaccessible),
        origin: OriginType::parse(&row.get::<_, String>(11)?).unwrap_or(OriginType::Unknown),
        task_id: row.get(12)?,
        checksum: row.get(13)?,
        last_verified_at: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}
