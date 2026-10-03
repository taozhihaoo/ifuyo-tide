//! Task IPC commands.
//!
//! M0 supports real, persisted test tasks only: create → list →
//! transition → delete, all validated in Rust and stored in SQLite.

use crate::app::state::AppState;
use crate::commands::ipc_result;
use crate::core::ids;
use crate::core::source::{Source, SourceType};
use crate::core::task::{Task, TaskState};
use crate::diagnostics;
use crate::dto::{TaskDto, TaskStateUpdate};
use crate::error::TideError;
use crate::security;
use tauri::State;

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> Result<Vec<TaskDto>, TideError> {
    let result = state.with_conn(|conn| {
        let tasks = state.task_repository.list(conn)?;
        Ok(tasks.into_iter().map(TaskDto::from).collect())
    });
    ipc_result("list_tasks", result)
}

/// Creates a real persisted test task (plus its source record).
///
/// This is the M0-only way to exercise the task pipeline end to end;
/// actual download sources arrive with M1/M3.
#[tauri::command]
pub fn create_test_task(
    state: State<'_, AppState>,
    name: Option<String>,
) -> Result<TaskDto, TideError> {
    let name = name.unwrap_or_else(|| "Test Task".to_string());
    let result = state.with_tx(|conn| {
        let source = Source::new(
            ids::new_id(),
            SourceType::Http,
            "Test source (M0)".to_string(),
        )?;
        let mut task = Task::new(
            ids::new_id(),
            source.source_id.clone(),
            name,
            TaskState::Created,
        )?;
        // Document the intended lifecycle: a freshly created task becomes
        // Ready through the state machine, never by direct assignment.
        task.transition_to(TaskState::Ready)?;

        state.source_repository.insert(conn, &source)?;
        state.task_repository.insert(conn, &task)?;

        diagnostics::info("task.created", &[("task_id", &task.task_id)]);
        Ok(TaskDto::from(task))
    });
    ipc_result("create_test_task", result)
}

#[tauri::command]
pub fn delete_task(state: State<'_, AppState>, task_id: String) -> Result<(), TideError> {
    let result = state.with_tx(|conn| {
        let task_id = security::validate_id(&task_id, "task_id")?;
        let task = state.task_repository.get(conn, &task_id)?;
        state.task_repository.delete(conn, &task_id)?;
        // M0 test tasks own their source record; remove it in the same
        // transaction so no orphan rows remain.
        state.source_repository.delete(conn, &task.source_id)?;
        diagnostics::info("task.deleted", &[("task_id", &task_id)]);
        Ok(())
    });
    ipc_result("delete_task", result)
}

/// Applies a state transition through the Rust state machine.
///
/// Illegal transitions come back as a structured STATE_TRANSITION error —
/// the frontend cannot corrupt task state.
#[tauri::command]
pub fn update_task_state(
    state: State<'_, AppState>,
    task_id: String,
    update: TaskStateUpdate,
) -> Result<TaskDto, TideError> {
    let result = state.with_tx(|conn| {
        let task_id = security::validate_id(&task_id, "task_id")?;
        let next = TaskState::parse(&update.state)?;
        let mut task = state.task_repository.get(conn, &task_id)?;
        let from = task.state;
        task.transition_to(next)?;
        state.task_repository.update(conn, &task)?;
        diagnostics::info(
            "task.state_changed",
            &[
                ("task_id", &task_id),
                ("from", from.as_str()),
                ("to", task.state.as_str()),
            ],
        );
        Ok(TaskDto::from(task))
    });
    ipc_result("update_task_state", result)
}
