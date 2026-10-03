/// Playback session domain boundary.
///
/// A playback session is *not* the download task itself: it has its own
/// lifecycle so that, in later milestones, one task can serve playback
/// while still downloading, and playback can end without affecting the
/// task. M0 fixes identity only — no player behaviour.
use crate::core::resource::ResourceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackSession {
    pub session_id: String,
    pub resource_id: ResourceId,
    /// Last known playback position; `None` until playback reports one.
    pub position_ms: Option<u64>,
    pub created_at: i64,
}

impl PlaybackSession {
    pub fn new(resource_id: ResourceId) -> Self {
        PlaybackSession {
            session_id: crate::core::ids::new_id(),
            resource_id,
            position_ms: None,
            created_at: crate::core::time::now_unix_ms(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_unique_and_independent_of_task_ids() {
        let a = PlaybackSession::new(ResourceId::new());
        let b = PlaybackSession::new(ResourceId::new());
        assert_ne!(a.session_id, b.session_id);
        assert_ne!(a.resource_id, b.resource_id);
        assert_eq!(a.position_ms, None);
    }
}
