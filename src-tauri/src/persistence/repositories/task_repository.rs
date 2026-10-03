//! Persistence boundary for tasks.

use crate::core::task::{Task, TaskState};
use crate::error::TideError;
use rusqlite::{params, Connection};

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskRepository;

impl TaskRepository {
    pub fn insert(&self, conn: &Connection, task: &Task) -> Result<(), TideError> {
        conn.execute(
            "INSERT INTO tasks (task_id, source_id, name, state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                task.task_id,
                task.source_id,
                task.name,
                task.state.as_str(),
                task.created_at,
                task.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, conn: &Connection, task_id: &str) -> Result<Task, TideError> {
        conn.query_row(
            "SELECT task_id, source_id, name, state, created_at, updated_at
             FROM tasks WHERE task_id = ?1",
            params![task_id],
            row_to_task,
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => {
                TideError::NotFound(format!("task not found: {task_id}"))
            }
            other => other.into(),
        })
    }

    pub fn list(&self, conn: &Connection) -> Result<Vec<Task>, TideError> {
        let mut stmt = conn.prepare(
            "SELECT task_id, source_id, name, state, created_at, updated_at
             FROM tasks ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], row_to_task)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    /// Full-row update keyed by task_id.
    pub fn update(&self, conn: &Connection, task: &Task) -> Result<(), TideError> {
        let changed = conn.execute(
            "UPDATE tasks
             SET source_id = ?2, name = ?3, state = ?4, created_at = ?5, updated_at = ?6
             WHERE task_id = ?1",
            params![
                task.task_id,
                task.source_id,
                task.name,
                task.state.as_str(),
                task.created_at,
                task.updated_at
            ],
        )?;
        if changed == 0 {
            return Err(TideError::NotFound(format!(
                "task not found: {}",
                task.task_id
            )));
        }
        Ok(())
    }

    /// Deletes a task. Fails with NotFound when the id is unknown.
    pub fn delete(&self, conn: &Connection, task_id: &str) -> Result<(), TideError> {
        let changed = conn.execute("DELETE FROM tasks WHERE task_id = ?1", params![task_id])?;
        if changed == 0 {
            return Err(TideError::NotFound(format!("task not found: {task_id}")));
        }
        Ok(())
    }
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let state_text: String = row.get(3)?;
    let state = TaskState::parse(&state_text).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            format!("invalid task state in database: {state_text}").into(),
        )
    })?;
    Ok(Task {
        task_id: row.get(0)?,
        source_id: row.get(1)?,
        name: row.get(2)?,
        state,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}
