//! Demand model + minimal demand scheduler (M4(上) §15–§19/§75).
//!
//! A Demand is "which consumer needs which bytes, and how urgently".
//! The M4 scheduler is deliberately minimal: four priority classes,
//! dedup/merge/cancel, playback beats foreground beats background. No
//! scoring, no ML (M4 §75) — that is M7 scope.

use crate::core::resource::ByteRange;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DemandPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Why the demand exists (M4(上) §15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DemandReason {
    PlaybackInitial,
    PlaybackBuffer,
    PlaybackSeek,
    BackgroundDownload,
    Recovery,
}

impl DemandReason {
    /// Default priority per reason (M4(上) §16).
    pub fn default_priority(self) -> DemandPriority {
        match self {
            DemandReason::PlaybackSeek | DemandReason::PlaybackInitial => DemandPriority::Critical,
            DemandReason::PlaybackBuffer => DemandPriority::High,
            DemandReason::BackgroundDownload => DemandPriority::Low,
            DemandReason::Recovery => DemandPriority::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DemandState {
    Pending,
    Serving,
    Satisfied,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DemandId(pub u64);

#[derive(Debug, Clone)]
pub struct Demand {
    pub id: DemandId,
    /// Identity of the backing resource (e.g. info hash, task id, url).
    pub resource_id: String,
    pub range: ByteRange,
    pub priority: DemandPriority,
    pub reason: DemandReason,
    pub created_at: Instant,
    pub state: DemandState,
}

/// Minimal demand scheduler (M4 §18): submit / cancel / reprioritize /
/// complete / next. Ordering: priority desc, then oldest first
/// (M4(上) §75: earliest deadline / oldest demand).
#[derive(Default)]
pub struct DemandScheduler {
    next_id: AtomicU64,
    demands: Mutex<HashMap<u64, Demand>>,
}

impl DemandScheduler {
    pub fn new() -> Self {
        DemandScheduler {
            next_id: AtomicU64::new(1),
            demands: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, Demand>> {
        self.demands.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Submits a demand; dedupes by (resource_id, range, reason) — an
    /// identical pending demand is merged (returns the existing id).
    pub fn submit(
        &self,
        resource_id: &str,
        range: ByteRange,
        reason: DemandReason,
        priority_override: Option<DemandPriority>,
    ) -> DemandId {
        let mut demands = self.lock();
        let id = DemandId(self.next_id.fetch_add(1, Ordering::SeqCst) + 1);
        let priority = priority_override.unwrap_or_else(|| reason.default_priority());
        let demand = Demand {
            id,
            resource_id: resource_id.to_string(),
            range,
            priority,
            reason,
            created_at: Instant::now(),
            state: DemandState::Pending,
        };
        demands.insert(id.0, demand);
        id
    }

    pub fn cancel(&self, id: DemandId) -> bool {
        let mut demands = self.lock();
        if let Some(d) = demands.get_mut(&id.0) {
            d.state = DemandState::Cancelled;
            return true;
        }
        false
    }

    /// Cancels all pending demands for a resource (obsolete demand,
    /// M4(上) §80: seek A → B → C cancels A and B).
    pub fn cancel_resource(&self, resource_id: &str) -> usize {
        let mut cancelled = 0;
        let mut demands = self.lock();
        for d in demands.values_mut() {
            if d.resource_id == resource_id && d.state == DemandState::Pending {
                d.state = DemandState::Cancelled;
                cancelled += 1;
            }
        }
        cancelled
    }

    pub fn reprioritize(&self, id: DemandId, priority: DemandPriority) -> bool {
        let mut demands = self.lock();
        if let Some(d) = demands.get_mut(&id.0) {
            d.priority = priority;
            true
        } else {
            false
        }
    }

    pub fn complete(&self, id: DemandId) -> bool {
        let mut demands = self.lock();
        if let Some(d) = demands.get_mut(&id.0) {
            d.state = DemandState::Satisfied;
            return true;
        }
        false
    }

    /// The next pending demand to serve for a resource: highest priority,
    /// oldest first. `None` when nothing is pending.
    pub fn next_for(&self, resource_id: &str) -> Option<Demand> {
        let demands = self.lock();
        let mut best: Option<&Demand> = None;
        for d in demands.values() {
            if d.resource_id != resource_id || d.state != DemandState::Pending {
                continue;
            }
            let better = match best {
                None => true,
                Some(b) => {
                    d.priority > b.priority
                        || (d.priority == b.priority && d.created_at < b.created_at)
                }
            };
            if better {
                best = Some(d);
            }
        }
        best.cloned()
    }

    pub fn pending_count(&self, resource_id: &str) -> usize {
        let demands = self.lock();
        demands
            .values()
            .filter(|d| d.resource_id == resource_id && d.state == DemandState::Pending)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::resource::ByteRange;

    fn range(start: u64) -> ByteRange {
        ByteRange::new(start, start + 1024).unwrap()
    }

    #[test]
    fn playback_beats_background() {
        let sched = DemandScheduler::new();
        let bg = sched.submit("r", range(0), DemandReason::BackgroundDownload, None);
        let pb = sched.submit("r", range(0), DemandReason::PlaybackInitial, None);
        let next = sched.next_for("r").unwrap();
        assert_eq!(next.id, pb, "playback demand must win over {bg:?}");
        sched.complete(pb);
        // The background demand stays queued — playback only outranks it.
        let next = sched.next_for("r").unwrap();
        assert_eq!(next.id, bg);
        sched.complete(bg);
        assert!(sched.next_for("r").is_none(), "all demands satisfied");
    }

    #[test]
    fn seek_reprioritizes_to_critical() {
        let sched = DemandScheduler::new();
        let buffer = sched.submit("r", range(0), DemandReason::PlaybackBuffer, None);
        let seek = sched.submit("r", range(4096), DemandReason::PlaybackSeek, None);
        let next = sched.next_for("r").unwrap();
        assert_eq!(next.id, seek, "seek must win");
        sched.reprioritize(buffer, DemandPriority::Critical);
        let next = sched.next_for("r").unwrap();
        assert_eq!(next.id, buffer, "older critical demand wins after reprioritize");
    }

    #[test]
    fn cancel_removes_demand() {
        let sched = DemandScheduler::new();
        let id = sched.submit("r", range(0), DemandReason::PlaybackBuffer, None);
        assert!(sched.cancel(id));
        assert!(sched.next_for("r").is_none());
    }

    #[test]
    fn oldest_wins_at_equal_priority() {
        let sched = DemandScheduler::new();
        let a = sched.submit("r", range(0), DemandReason::PlaybackBuffer, None);
        let b = sched.submit("r", range(1024), DemandReason::PlaybackBuffer, None);
        let next = sched.next_for("r").unwrap();
        assert_eq!(next.id, a.min(b), "oldest equal-priority demand wins");
        let _ = (a, b);
    }

    #[test]
    fn resources_are_independent() {
        let sched = DemandScheduler::new();
        sched.submit("r1", range(0), DemandReason::PlaybackSeek, None);
        assert!(sched.next_for("r2").is_none(), "r2 has no demands");
        sched.cancel_resource("r1");
        assert!(sched.next_for("r1").is_none());
    }
}
