//! Scheduler runtime (M7(上) §17/§18/§49): event-driven trigger +
//! low-frequency reconciliation tick. Stage B wires the snapshot
//! provider to the live engines; the pure core stays in `policy`.

use super::config::SchedulerConfig;
use super::model::SchedulerSnapshot;

/// Where the runtime gets its immutable input (§50). The app wires
/// this to the download engine + playback manager.
pub trait SnapshotProvider: Send + Sync {
    fn collect(&self) -> SchedulerSnapshot;
}

/// Where decisions go (adapters). Kept async-free: apply is quick and
/// non-blocking by contract (engine rate setters are atomics).
pub trait DecisionSink: Send + Sync {
    fn apply(&self, decisions: &[super::model::AllocationDecision]);
}

pub struct SchedulerRuntime {
    pub config: std::sync::RwLock<SchedulerConfig>,
    pub tick_interval: std::time::Duration,
}

impl SchedulerRuntime {
    pub fn new(config: SchedulerConfig) -> SchedulerRuntime {
        SchedulerRuntime {
            config: std::sync::RwLock::new(config),
            tick_interval: std::time::Duration::from_millis(500), // §18: low-frequency
        }
    }

    pub fn config(&self) -> SchedulerConfig {
        self.config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn set_config(&self, config: SchedulerConfig) {
        *self
            .config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = config;
    }
}
