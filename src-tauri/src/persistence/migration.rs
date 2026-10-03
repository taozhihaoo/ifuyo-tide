//! Versioned, transactional schema migrations.
//!
//! Each migration runs inside a single transaction together with the
//! `PRAGMA user_version` bump, so a crash mid-migration can never leave
//! the database half-migrated. Ad-hoc `CREATE TABLE IF NOT EXISTS` calls
//! in startup code are not used.

use crate::error::TideError;
use rusqlite::Connection;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: include_str!("migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "download_tasks",
        sql: include_str!("migrations/0002_download_tasks.sql"),
    },
];

/// Applies all pending migrations, in order.
pub fn run_migrations(conn: &mut Connection) -> Result<(), TideError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
        crate::diagnostics::info(
            "db.migration_applied",
            &[
                ("version", &migration.version.to_string()),
                ("name", migration.name),
            ],
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_versioned_and_ordered() {
        let mut last = 0;
        for m in MIGRATIONS {
            assert!(m.version > last, "migration versions must be ascending");
            last = m.version;
            assert!(!m.sql.trim().is_empty());
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert!(version >= 1);
    }
}
