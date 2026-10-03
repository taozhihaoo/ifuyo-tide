use crate::error::TideError;
use serde::{Deserialize, Serialize};

/// Lifecycle state of a download task.
///
/// This is a real state machine, not a set of booleans: every change goes
/// through [`TaskState::transition`], and illegal transitions are rejected
/// by the Rust core — never by the frontend.
///
/// ```text
/// Created → Ready → Running ⇄ Paused
///                 Running → Completed | Failed
///                 any active state → Cancelled
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Created,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl TaskState {
    /// States from which cancellation is allowed.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            TaskState::Created | TaskState::Ready | TaskState::Running | TaskState::Paused
        )
    }

    /// Whether `self → next` is a legal transition.
    pub fn can_transition_to(self, next: TaskState) -> bool {
        use TaskState::*;
        matches!(
            (self, next),
            (Created, Ready)
                | (Created, Cancelled)
                | (Ready, Running)
                | (Ready, Cancelled)
                | (Running, Paused)
                | (Running, Completed)
                | (Running, Failed)
                | (Running, Cancelled)
                | (Paused, Running)
                | (Paused, Cancelled)
        )
    }

    /// Validates and returns the next state.
    pub fn transition(self, next: TaskState) -> Result<TaskState, TideError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(TideError::StateTransition {
                from: self.as_str().to_string(),
                to: next.as_str().to_string(),
            })
        }
    }

    /// Canonical storage / wire representation (snake_case).
    pub fn as_str(self) -> &'static str {
        match self {
            TaskState::Created => "created",
            TaskState::Ready => "ready",
            TaskState::Running => "running",
            TaskState::Paused => "paused",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<TaskState, TideError> {
        match value {
            "created" => Ok(TaskState::Created),
            "ready" => Ok(TaskState::Ready),
            "running" => Ok(TaskState::Running),
            "paused" => Ok(TaskState::Paused),
            "completed" => Ok(TaskState::Completed),
            "failed" => Ok(TaskState::Failed),
            "cancelled" => Ok(TaskState::Cancelled),
            other => Err(TideError::InvalidInput(format!(
                "unknown task state: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_uses_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskState::Cancelled).unwrap(),
            "\"cancelled\""
        );
        let state: TaskState = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(state, TaskState::Running);
    }

    #[test]
    fn parse_rejects_unknown_state() {
        assert!(TaskState::parse("flying").is_err());
        assert!(TaskState::parse("").is_err());
    }

    #[test]
    fn storage_roundtrip_is_lossless() {
        for state in [
            TaskState::Created,
            TaskState::Ready,
            TaskState::Running,
            TaskState::Paused,
            TaskState::Completed,
            TaskState::Failed,
            TaskState::Cancelled,
        ] {
            assert_eq!(TaskState::parse(state.as_str()).unwrap(), state);
        }
    }

    #[test]
    fn only_active_states_can_be_cancelled() {
        assert!(TaskState::Created.can_transition_to(TaskState::Cancelled));
        assert!(TaskState::Ready.can_transition_to(TaskState::Cancelled));
        assert!(TaskState::Running.can_transition_to(TaskState::Cancelled));
        assert!(TaskState::Paused.can_transition_to(TaskState::Cancelled));
        assert!(!TaskState::Completed.can_transition_to(TaskState::Cancelled));
        assert!(!TaskState::Failed.can_transition_to(TaskState::Cancelled));
    }
}
