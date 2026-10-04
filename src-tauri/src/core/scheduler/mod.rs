//! M7(上) — Smart Scheduler control plane.
//!
//! Components: config (§73 all knobs, INITIAL DEFAULTS), model
//! (snapshot views + decisions + decomposable DecisionReason §16),
//! policy (the PURE deterministic four-phase allocation §55 with the
//! playback state machine §27–§34, weighted fairness §20, capped
//! aging §21), bandwidth (EMA §41 + token bucket), adapters (§81
//! capability-carrying engine contracts), runtime (event-driven +
//! low-frequency tick §18, decision diff §17).
//!
//! HARD BOUNDARIES (§3): the scheduler is a CONTROL PLANE. It never
//! opens sockets, never touches peers/pieces, never writes files, and
//! never bypasses the download engines. Its output is
//! `AllocationDecision` intents that the adapters translate into
//! engine commands (HTTP: per-task rate via the engine's limiter;
//! Torrent: session rate via librqbit's PUBLIC `ratelimits` API —
//! no private modules, §12).
//!
//! Determinism (§51): `policy::compute(snapshot, config)` is pure —
//! each of the spec's §91–§96 examples is a unit test in `policy.rs`.
//!
//! The M0-era coarse `Priority` enum was absorbed into
//! [`config::PriorityClass`] (its only consumer was its own test).

pub mod adapters;
pub mod bandwidth;
pub mod config;
pub mod model;
pub mod policy;
pub mod range;
pub mod runtime;

pub use adapters::{EngineAdapter, HttpAdapter, TorrentAdapter};
pub use bandwidth::{Ema, TokenBucket};
pub use config::{PlaybackWatermarks, PriorityClass, SchedulerConfig, UserPriority};
pub use model::{
    AllocationDecision, DecisionReason, DemandKind, EngineKind, GlobalLimits, NetworkSnapshot,
    PlaybackSchedulingState, PlaybackView, ReasonCode, SchedulerSnapshot, TaskRunState, TaskView,
};
pub use policy::{classify_playback, compute};
pub use range::RangeSet;
pub use runtime::{AppSnapshotProvider, SchedulerRuntime, SnapshotProvider};
