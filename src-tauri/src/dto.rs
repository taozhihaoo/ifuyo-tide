//! DTO boundary between the Rust core and the frontend.
//!
//! Domain structs evolve freely; these types are the stable IPC contract.
//! Field names are camelCase on the wire.

use crate::core::source::SourceType;
use crate::core::task::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub task_id: String,
    pub source_id: String,
    pub name: String,
    /// Canonical snake_case state string (e.g. "ready").
    pub state: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<Task> for TaskDto {
    fn from(task: Task) -> Self {
        TaskDto {
            task_id: task.task_id,
            source_id: task.source_id,
            name: task.name,
            state: task.state.as_str().to_string(),
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoDto {
    pub app_name: String,
    pub version: String,
    /// Current implementation milestone; the UI uses this to explain
    /// what is real and what is not yet implemented.
    pub milestone: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub download_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskStateUpdate {
    /// Canonical snake_case state string, validated by `TaskState::parse`.
    pub state: String,
}

/// Parses a frontend-supplied source type string.
pub fn parse_source_type(raw: &str) -> Result<SourceType, crate::error::TideError> {
    SourceType::parse(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_dto_uses_camel_case_wire_names() {
        let task = Task::new(
            crate::core::ids::new_id(),
            crate::core::ids::new_id(),
            "dto test".into(),
            crate::core::task::TaskState::Ready,
        )
        .unwrap();
        let json = serde_json::to_value(TaskDto::from(task)).unwrap();
        assert!(json.get("taskId").is_some());
        assert!(json.get("sourceId").is_some());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("updatedAt").is_some());
        assert!(json.get("task_id").is_none());
        assert_eq!(json.get("state").unwrap(), "ready");
    }

    #[test]
    fn app_info_shape() {
        let info = AppInfoDto {
            app_name: "ifuyo Tide".into(),
            version: "0.1.0".into(),
            milestone: "M0".into(),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("appName").is_some());
        assert!(json.get("milestone").is_some());
    }
}
