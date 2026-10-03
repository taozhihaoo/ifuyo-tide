//! Persistence boundary for sources.

use crate::core::source::{Source, SourceType};
use crate::error::TideError;
use rusqlite::{params, Connection};

#[derive(Debug, Default, Clone, Copy)]
pub struct SourceRepository;

impl SourceRepository {
    pub fn insert(&self, conn: &Connection, source: &Source) -> Result<(), TideError> {
        conn.execute(
            "INSERT INTO sources (source_id, source_type, display_name, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                source.source_id,
                source.source_type.as_str(),
                source.display_name,
                source.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, conn: &Connection, source_id: &str) -> Result<Source, TideError> {
        conn.query_row(
            "SELECT source_id, source_type, display_name, created_at
             FROM sources WHERE source_id = ?1",
            params![source_id],
            row_to_source,
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => {
                TideError::NotFound(format!("source not found: {source_id}"))
            }
            other => other.into(),
        })
    }

    pub fn list(&self, conn: &Connection) -> Result<Vec<Source>, TideError> {
        let mut stmt = conn.prepare(
            "SELECT source_id, source_type, display_name, created_at
             FROM sources ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], row_to_source)?;
        let mut sources = Vec::new();
        for row in rows {
            sources.push(row?);
        }
        Ok(sources)
    }

    pub fn update(&self, conn: &Connection, source: &Source) -> Result<(), TideError> {
        let changed = conn.execute(
            "UPDATE sources SET source_type = ?2, display_name = ?3, created_at = ?4
             WHERE source_id = ?1",
            params![
                source.source_id,
                source.source_type.as_str(),
                source.display_name,
                source.created_at
            ],
        )?;
        if changed == 0 {
            return Err(TideError::NotFound(format!(
                "source not found: {}",
                source.source_id
            )));
        }
        Ok(())
    }

    pub fn delete(&self, conn: &Connection, source_id: &str) -> Result<(), TideError> {
        let changed = conn.execute(
            "DELETE FROM sources WHERE source_id = ?1",
            params![source_id],
        )?;
        if changed == 0 {
            return Err(TideError::NotFound(format!(
                "source not found: {source_id}"
            )));
        }
        Ok(())
    }
}

fn row_to_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<Source> {
    let type_text: String = row.get(1)?;
    let source_type = SourceType::parse(&type_text).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            format!("invalid source type in database: {type_text}").into(),
        )
    })?;
    Ok(Source {
        source_id: row.get(0)?,
        source_type,
        display_name: row.get(2)?,
        created_at: row.get(3)?,
    })
}
