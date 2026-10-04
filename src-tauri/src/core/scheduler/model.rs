//! Scheduler snapshot & decision model (M7(上) §15/§50/§16).
//!
//! The scheduler core is a PURE function: `policy::compute(snapshot,
//! config) -> Vec<AllocationDecision>`. Everything it needs is in the
//! snapshot — `SystemTime::now()` never appears inside the core (§51);
//! time travels as `now_ms`.

use super::config::{PriorityClass, UserPriority};
use serde::Serialize;

/// §40: what the scheduler knows about the network — REAL engine
/// telemetry only (§41), smoothed via EMA in `bandwidth.rs`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct NetworkSnapshot {
    /// EMA of total observed download throughput (bytes/sec).
    pub smoothed_bps: f64,
    /// Last-sample total throughput.
    pub instant_bps: f64,
}

/// §24: the global cap (`None` = unlimited).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct GlobalLimits {
    pub bandwidth_bps: Option<u64>,
}

/// Which data plane a task belongs to (§14 — the scheduler knows
/// engines only as capability-carrying adapters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Http,
    Torrent,
}

/// Observed task lifecycle state (§45: the scheduler OBSERVES the M0/M2
/// TaskState — it never defines its own conflicting lifecycle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunState {
    Running,
    Paused,
    Completed,
    Failed,
}

/// A download task as the scheduler sees it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskView {
    pub task_id: String,
    pub engine: EngineKind,
    pub state: TaskRunState,
    /// §23 user preference, if any.
    pub user_priority: Option<UserPriority>,
    /// §41 smoothed throughput of this task (bytes/sec).
    pub smoothed_bps: f64,
    /// §21: seconds since this task last received an allocation.
    pub waited_secs: u64,
}

/// §27: the playback scheduling state machine (hysteresis applied by
/// the policy; the RAW input is the buffer seconds below).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackSchedulingState {
    Startup,
    Playing,
    LowBuffer,
    CriticalBuffer,
    Seeking,
    Paused,
    Ended,
    Error,
}

/// A playback session as the scheduler sees it. Buffer seconds come
/// from the REAL media element (M5 reports `bufferedAhead`), never
/// invented (§77); `None` means "unknown — assume healthy" so an
/// un-reporting client cannot trigger fake emergencies.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlaybackView {
    pub session_id: String,
    /// The download task feeding this session, when one exists
    /// (in-progress HTTP download / torrent file). Playback on a
    /// COMPLETED file needs no allocation at all.
    pub task_id: Option<String>,
    pub engine: EngineKind,
    pub state: PlaybackSchedulingState,
    pub buffered_ahead_secs: Option<f64>,
    /// §38: forward buffer outranks backward; the backward reserve is
    /// a future refinement (config slot recorded in M7(下) space).
    pub position_ms: Option<u64>,
}

/// §6 DemandKind — mapped from the M4 demand reasons + session state.
/// The M4 runtime registry (core/streaming/demand.rs) stays the
/// per-chunk bookkeeping layer; the policy sees aggregated intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DemandKind {
    PlaybackStartup,
    PlaybackContinue,
    PlaybackSeek,
    PlaybackRebuffer,
    InteractiveDownload,
    BackgroundDownload,
    ExplicitUserPriority,
    Recovery,
    Verification,
}

/// §50: the complete immutable scheduler input.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SchedulerSnapshot {
    pub now_ms: i64,
    pub network: NetworkSnapshot,
    pub limits: GlobalLimits,
    pub tasks: Vec<TaskView>,
    pub playback: Vec<PlaybackView>,
}

/// §16: WHY. Every decision carries a decomposable reason — never a
/// bare score.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    PlaybackCritical,
    PlaybackLow,
    PlaybackStartup,
    PlaybackHealthy,
    PlaybackPaused,
    PlaybackEnded,
    SeekElevation,
    UserPriority,
    Aging,
    FairShare,
    BackgroundSpare,
    BackgroundFloor,
    NoCapacity,
    TaskNotRunning,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DecisionReason {
    pub code: ReasonCode,
    /// Human-readable decomposition: base/urgency/aging/user parts.
    pub detail: String,
}

/// §15: one scheduling decision — an INTENT for an engine adapter,
/// not a socket command (§3 control plane vs data plane).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AllocationDecision {
    pub task_id: String,
    pub engine: EngineKind,
    pub priority_class: PriorityClass,
    /// §22 decomposed effective weight (base × user × aging), kept for
    /// explainability.
    pub effective_weight: f64,
    /// §25/§26: allocated bandwidth (bytes/sec). 0 = starved this tick.
    pub allocated_bps: u64,
    /// §26 minimum/target/max split for playback tasks.
    pub min_bps: u64,
    pub target_bps: u64,
    /// Playback session driving this decision, if any.
    pub session_id: Option<String>,
    pub reason: DecisionReason,
}
