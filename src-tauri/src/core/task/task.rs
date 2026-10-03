use super::TaskState;
use crate::core::{ids, time};
use crate::error::TideError;

/// A download task domain object.
///
/// M0 establishes identity, state and persistence boundaries only — no
/// download behaviour lives here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub task_id: String,
    pub source_id: String,
    pub name: String,
    pub state: TaskState,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Task {
    /// Creates a task after validating its fields.
    pub fn new(
        task_id: String,
        source_id: String,
        name: String,
        state: TaskState,
    ) -> Result<Task, TideError> {
        let name = crate::security::validate_task_name(&name)?;
        if !ids::is_valid_id(&task_id) {
            return Err(TideError::InvalidInput("task_id is not a valid id".into()));
        }
        if !ids::is_valid_id(&source_id) {
            return Err(TideError::InvalidInput(
                "source_id is not a valid id".into(),
            ));
        }
        let now = time::now_unix_ms();
        Ok(Task {
            task_id,
            source_id,
            name,
            state,
            created_at: now,
            updated_at: now,
        })
    }

    /// Applies a state transition through the state machine.
    ///
    /// Illegal transitions are rejected and leave the task untouched.
    pub fn transition_to(&mut self, next: TaskState) -> Result<(), TideError> {
        self.state = self.state.transition(next)?;
        self.updated_at = time::now_unix_ms();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids;

    fn valid_task() -> Task {
        Task::new(
            ids::new_id(),
            ids::new_id(),
            "Sample task".into(),
            TaskState::Created,
        )
        .unwrap()
    }

    #[test]
    fn new_generates_utc_timestamps() {
        let t = valid_task();
        assert_eq!(t.created_at, t.updated_at);
        assert!(t.created_at > 1_767_225_600_000);
    }

    #[test]
    fn new_rejects_invalid_ids() {
        assert!(Task::new("nope".into(), ids::new_id(), "n".into(), TaskState::Created).is_err());
        assert!(Task::new(ids::new_id(), "1".into(), "n".into(), TaskState::Created).is_err());
    }

    #[test]
    fn new_rejects_invalid_names() {
        assert!(Task::new(ids::new_id(), ids::new_id(), "".into(), TaskState::Created).is_err());
        assert!(Task::new(
            ids::new_id(),
            ids::new_id(),
            "   ".into(),
            TaskState::Created
        )
        .is_err());
        let long = "x".repeat(500);
        assert!(Task::new(ids::new_id(), ids::new_id(), long, TaskState::Created).is_err());
    }

    #[test]
    fn name_is_trimmed_on_create() {
        let t = Task::new(
            ids::new_id(),
            ids::new_id(),
            "  padded name  ".into(),
            TaskState::Created,
        )
        .unwrap();
        assert_eq!(t.name, "padded name");
    }
}
