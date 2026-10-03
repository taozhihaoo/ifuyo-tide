mod state;
mod task;

pub use state::TaskState;
pub use task::Task;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TideError;

    fn task_in(state: TaskState) -> Task {
        Task::new("task-1".into(), "src-1".into(), "Sample".into(), state)
    }

    #[test]
    fn legal_transitions_from_spec() {
        // Created → Ready → Running → Paused → Running → Completed
        let mut t = task_in(TaskState::Created);
        t.transition_to(TaskState::Ready).unwrap();
        t.transition_to(TaskState::Running).unwrap();
        t.transition_to(TaskState::Paused).unwrap();
        t.transition_to(TaskState::Running).unwrap();
        t.transition_to(TaskState::Completed).unwrap();
        assert_eq!(t.state, TaskState::Completed);
    }

    #[test]
    fn running_to_failed_is_legal() {
        let mut t = task_in(TaskState::Running);
        t.transition_to(TaskState::Failed).unwrap();
        assert_eq!(t.state, TaskState::Failed);
    }

    #[test]
    fn running_to_cancelled_is_legal() {
        let mut t = task_in(TaskState::Running);
        t.transition_to(TaskState::Cancelled).unwrap();
        assert_eq!(t.state, TaskState::Cancelled);
    }

    #[test]
    fn created_to_cancelled_is_legal() {
        let mut t = task_in(TaskState::Created);
        t.transition_to(TaskState::Cancelled).unwrap();
        assert_eq!(t.state, TaskState::Cancelled);
    }

    #[test]
    fn ready_to_cancelled_is_legal() {
        let mut t = task_in(TaskState::Ready);
        t.transition_to(TaskState::Cancelled).unwrap();
        assert_eq!(t.state, TaskState::Cancelled);
    }

    #[test]
    fn paused_to_cancelled_is_legal() {
        let mut t = task_in(TaskState::Paused);
        t.transition_to(TaskState::Cancelled).unwrap();
        assert_eq!(t.state, TaskState::Cancelled);
    }

    #[test]
    fn terminal_states_cannot_transition() {
        // Completed → Running must fail.
        let mut t = task_in(TaskState::Completed);
        assert!(matches!(
            t.transition_to(TaskState::Running),
            Err(TideError::StateTransition { .. })
        ));

        // Cancelled → Running must fail.
        let mut t = task_in(TaskState::Cancelled);
        assert!(matches!(
            t.transition_to(TaskState::Running),
            Err(TideError::StateTransition { .. })
        ));

        // Failed is terminal as well.
        let mut t = task_in(TaskState::Failed);
        assert!(matches!(
            t.transition_to(TaskState::Running),
            Err(TideError::StateTransition { .. })
        ));
    }

    #[test]
    fn arbitrary_jumps_are_rejected() {
        // Created → Running skips Ready.
        let mut t = task_in(TaskState::Created);
        assert!(matches!(
            t.transition_to(TaskState::Running),
            Err(TideError::StateTransition { .. })
        ));

        // Paused → Completed skips Running.
        let mut t = task_in(TaskState::Paused);
        assert!(matches!(
            t.transition_to(TaskState::Completed),
            Err(TideError::StateTransition { .. })
        ));

        // Ready → Completed skips Running.
        let mut t = task_in(TaskState::Ready);
        assert!(matches!(
            t.transition_to(TaskState::Completed),
            Err(TideError::StateTransition { .. })
        ));
    }

    #[test]
    fn same_state_transition_is_rejected() {
        let mut t = task_in(TaskState::Running);
        assert!(matches!(
            t.transition_to(TaskState::Running),
            Err(TideError::StateTransition { .. })
        ));
    }

    #[test]
    fn failed_transition_keeps_previous_state() {
        let mut t = task_in(TaskState::Completed);
        assert!(t.transition_to(TaskState::Running).is_err());
        assert_eq!(t.state, TaskState::Completed);
    }

    #[test]
    fn successful_transition_updates_updated_at() {
        let mut t = task_in(TaskState::Created);
        let before = t.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(2));
        t.transition_to(TaskState::Ready).unwrap();
        assert!(t.updated_at >= before);
    }
}
