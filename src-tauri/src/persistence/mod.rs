//! SQLite access: connection setup, migrations and repository boundary.
//!
//! Domain code never talks to SQL directly — `Domain → Repository → SQLite`.

pub mod migration;
pub mod repositories;

use crate::error::TideError;
use rusqlite::Connection;
use std::path::Path;

/// Opens (or creates) the application database and brings the schema
/// up to date.
pub fn open(path: &Path) -> Result<Connection, TideError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)
        .map_err(|_| TideError::Persistence("failed to open database".into()))?;
    configure(&conn)?;
    let mut conn = conn;
    migration::run_migrations(&mut conn)?;
    crate::diagnostics::info("db.ready", &[]);
    Ok(conn)
}

/// Opens an in-memory database with the same configuration — used by tests.
pub fn open_in_memory() -> Result<Connection, TideError> {
    let conn = Connection::open_in_memory()
        .map_err(|_| TideError::Persistence("failed to open in-memory database".into()))?;
    configure(&conn)?;
    let mut conn = conn;
    migration::run_migrations(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<(), TideError> {
    // WAL + NORMAL give durability with good write throughput; a power
    // loss can only lose the last checkpoint, never corrupt the DB.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}
