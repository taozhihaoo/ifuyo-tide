//! Engine adapter contracts (M7(上) §3/§81/§82/§83).
//!
//! The scheduler is a control plane: its decisions reach the data
//! plane ONLY through these adapters. Each adapter declares its
//! CAPABILITIES (§82) and degrades unsupported decisions GRACEFULLY
//! with a reason (§83) — never a panic, never a fork of engine
//! internals (§12).

use super::model::{AllocationDecision, EngineKind};

/// What an engine can actually do with a decision (§82).
#[derive(Debug, Clone, Copy)]
pub struct AdapterCapability {
    /// Accepts per-task bandwidth rates.
    pub per_task_bandwidth: bool,
    /// Accepts range prioritization intent (HTTP: yes conceptually,
    /// Torrent: through the resource stream / rqbit's own selector).
    pub range_prioritization: bool,
}

/// §81: the contract between scheduler and data plane.
pub trait EngineAdapter {
    fn kind(&self) -> EngineKind;
    fn capabilities(&self) -> AdapterCapability;

    /// Applies one decision. Returns a short degradation note when the
    /// decision was only partially applicable (§83), None on full
    /// application. Implementations must never panic.
    fn apply_decision(&self, decision: &AllocationDecision) -> Option<String>;

    /// Observation is the adapter's job too: engines report REAL
    /// throughput (§41) — never estimates (§77).
    fn observe_throughput_bps(&self, task_id: &str) -> f64;
}

/// HTTP data plane: the M1 engine enforces per-task token buckets set
/// here (real enforcement, `core::download::engine`).
pub struct HttpAdapter {
    rates: std::sync::Mutex<std::collections::HashMap<String, Arc<TokenBucket>>>,
}

use super::bandwidth::TokenBucket;
use std::sync::Arc;

impl HttpAdapter {
    pub fn new() -> HttpAdapter {
        HttpAdapter {
            rates: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Registers the limiter handle a download task reads its rate
    /// from (wired by the engine at task start).
    pub fn register_limiter(&self, task_id: &str, bucket: Arc<TokenBucket>) {
        self.rates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(task_id.to_string(), bucket);
    }

    pub fn unregister_limiter(&self, task_id: &str) {
        self.rates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(task_id);
    }
}

impl Default for HttpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineAdapter for HttpAdapter {
    fn kind(&self) -> EngineKind {
        EngineKind::Http
    }

    fn capabilities(&self) -> AdapterCapability {
        AdapterCapability {
            per_task_bandwidth: true,
            range_prioritization: true, // M4 origin read-through serves ranges
        }
    }

    fn apply_decision(&self, decision: &AllocationDecision) -> Option<String> {
        let rates = self
            .rates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match rates.get(&decision.task_id) {
            Some(bucket) => {
                bucket.set_rate_bps(decision.allocated_bps);
                if decision.allocated_bps == 0 {
                    Some("rate 0 — starved this tick".into())
                } else {
                    None
                }
            }
            None => Some("no limiter registered for task (not running?)".into()),
        }
    }

    fn observe_throughput_bps(&self, _task_id: &str) -> f64 {
        // Throughput observation comes from the engine's own speed
        // tracker via the snapshot provider — the adapter returns 0
        // here; runtime fills the real value.
        0.0
    }
}

/// Torrent data plane: librqbit's PUBLIC `Session.ratelimits` (§12 —
/// public API, no fork). Session-level (not per-task) — recorded as a
/// capability limitation (§82/§83).
pub struct TorrentAdapter {
    rate: Arc<TokenBucket>,
}

impl TorrentAdapter {
    pub fn new(rate: Arc<TokenBucket>) -> TorrentAdapter {
        TorrentAdapter { rate }
    }
}

impl EngineAdapter for TorrentAdapter {
    fn kind(&self) -> EngineKind {
        EngineKind::Torrent
    }

    fn capabilities(&self) -> AdapterCapability {
        AdapterCapability {
            per_task_bandwidth: false,   // session-level only (public API)
            range_prioritization: false, // rqbit's own streaming selector owns pieces (§35)
        }
    }

    fn apply_decision(&self, decision: &AllocationDecision) -> Option<String> {
        self.rate.set_rate_bps(decision.allocated_bps);
        Some(
            "torrent rate is SESSION-level (librqbit public API); \
             range prioritization delegated to rqbit"
                .into(),
        )
    }

    fn observe_throughput_bps(&self, _task_id: &str) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::scheduler::config::PriorityClass;
    use crate::core::scheduler::model::DecisionReason;

    fn decision(task_id: &str, engine: EngineKind, bps: u64) -> AllocationDecision {
        AllocationDecision {
            task_id: task_id.into(),
            engine,
            priority_class: PriorityClass::Normal,
            effective_weight: 1.0,
            allocated_bps: bps,
            min_bps: 0,
            target_bps: 0,
            session_id: None,
            reason: DecisionReason {
                code: crate::core::scheduler::model::ReasonCode::FairShare,
                detail: "test".into(),
            },
        }
    }

    /// §83: an unsupported/unknown task degrades with a reason, never
    /// panics; supported tasks get their rate applied.
    #[test]
    fn http_adapter_applies_and_degrades() {
        let adapter = HttpAdapter::new();
        let bucket = Arc::new(TokenBucket::new(0));
        adapter.register_limiter("t1", bucket.clone());
        assert!(adapter
            .apply_decision(&decision("t1", EngineKind::Http, 1234))
            .is_none());
        assert_eq!(bucket.rate_bps(), 1234);
        let note = adapter.apply_decision(&decision("ghost", EngineKind::Http, 999));
        assert!(note.is_some(), "unknown task degrades with a note");
    }

    #[test]
    fn torrent_adapter_reports_capability_limitation() {
        let adapter = TorrentAdapter::new(Arc::new(TokenBucket::new(0)));
        let note = adapter.apply_decision(&decision("t", EngineKind::Torrent, 5000));
        assert!(note.is_some(), "§82: session-level limitation is recorded");
        assert!(!adapter.capabilities().per_task_bandwidth);
        assert!(!adapter.capabilities().range_prioritization);
    }
}
