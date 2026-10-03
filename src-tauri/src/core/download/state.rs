//! Download state model (M1 §23): REUSES the M0 `TaskState` — no second
//! parallel task-state machine — plus a download-specific `DownloadPhase`
//! carried while the task is active.

use crate::core::task::TaskState;
use serde::Serialize;

/// Where within the active download the engine currently is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadPhase {
    Preparing,
    Connecting,
    Downloading,
    Retrying,
    Finalizing,
}

impl DownloadPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            DownloadPhase::Preparing => "preparing",
            DownloadPhase::Connecting => "connecting",
            DownloadPhase::Downloading => "downloading",
            DownloadPhase::Retrying => "retrying",
            DownloadPhase::Finalizing => "finalizing",
        }
    }
}

/// Engine-level lifecycle (maps onto M0 TaskState; M1 §23 legal flows:
/// Created → Running ⇄ Paused → Completed/Failed/Cancelled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadLifecycle {
    Created,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            DownloadLifecycle::Created => "created",
            DownloadLifecycle::Ready => "ready",
            DownloadLifecycle::Running => "running",
            DownloadLifecycle::Paused => "paused",
            DownloadLifecycle::Completed => "completed",
            DownloadLifecycle::Failed => "failed",
            DownloadLifecycle::Cancelled => "cancelled",
        }
    }

    /// Restores a lifecycle from a persisted state string (M2 §8):
    /// a stale "running" row is not a durable fact and becomes Ready
    /// (resumable, idle); terminal states are preserved.
    pub fn from_state_str(state: &str) -> DownloadLifecycle {
        match state {
            "created" | "running" => DownloadLifecycle::Ready,
            "ready" => DownloadLifecycle::Ready,
            "paused" => DownloadLifecycle::Paused,
            "completed" => DownloadLifecycle::Completed,
            "failed" => DownloadLifecycle::Failed,
            "cancelled" => DownloadLifecycle::Cancelled,
            _ => DownloadLifecycle::Ready,
        }
    }

    /// The equivalent M0 TaskState (single shared state vocabulary).
    pub fn to_task_state(self) -> TaskState {
        match self {
            DownloadLifecycle::Created => TaskState::Created,
            DownloadLifecycle::Ready => TaskState::Ready,
            DownloadLifecycle::Running => TaskState::Running,
            DownloadLifecycle::Paused => TaskState::Paused,
            DownloadLifecycle::Completed => TaskState::Completed,
            DownloadLifecycle::Failed => TaskState::Failed,
            DownloadLifecycle::Cancelled => TaskState::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_maps_onto_m0_task_state() {
        // The M0 vocabulary stays authoritative — no parallel states.
        for lifecycle in [
            DownloadLifecycle::Created,
            DownloadLifecycle::Running,
            DownloadLifecycle::Paused,
            DownloadLifecycle::Completed,
            DownloadLifecycle::Failed,
            DownloadLifecycle::Cancelled,
        ] {
            let task = lifecycle.to_task_state();
            assert_eq!(task.as_str(), lifecycle.as_str());
        }
    }

    #[test]
    fn phases_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&DownloadPhase::Retrying).unwrap(),
            "\"retrying\""
        );
    }
}
