//! Scheduler runtime (M7(上) §17/§18/§46/§49/§63).
//!
//! Event-driven trigger + a low-frequency reconciliation tick (§18).
//! Each tick: collect snapshot → policy::compute → DIFF against the
//! last applied allocation (§17: engines only hear about CHANGES) →
//! apply through the engines. Runtime state (last rates, last playback
//! states, last-alloc stamps for aging) is in-memory only (§46/§47) —
//! restart rebuilds it.
//!
//! Allocated ≠ achieved (§63/§64): the scheduler never assumes its
//! rate will be met; telemetry flows back through the snapshot and the
//! next tick recalculates.

use super::config::{SchedulerConfig, UserPriority};
use super::model::{
    AllocationDecision, DecisionReason, EngineKind, PlaybackSchedulingState, PlaybackView,
    ReasonCode, SchedulerSnapshot, TaskRunState, TaskView,
};
use super::policy;
use crate::core::download::DownloadEngine;
use crate::core::torrent::TorrentEngine;
use crate::persistence::repositories::TorrentRepository;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// The app-side snapshot provider: download engine + torrent
/// engine/repo + playback manager + user-priority overrides.
pub struct AppSnapshotProvider {
    pub downloads: DownloadEngine,
    pub torrents: TorrentEngine,
    pub torrent_repo: Arc<TorrentRepository>,
    pub playback: Arc<crate::core::streaming::playback::PlaybackManager>,
    /// §23 user priority overrides (runtime registry; persisted via
    /// the settings repository).
    pub user_priorities: Arc<Mutex<HashMap<String, UserPriority>>>,
    /// §24 global bandwidth cap (bytes/sec; 0 = unlimited).
    pub global_bandwidth_bps: Arc<AtomicU64>,
}

impl super::runtime::SnapshotProvider for AppSnapshotProvider {
    fn collect(&self) -> SchedulerSnapshot {
        let priorities = self
            .user_priorities
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let mut tasks: Vec<TaskView> = Vec::new();
        for shared in self.downloads.list_sync() {
            let status = shared.status();
            let state = match status.lifecycle {
                crate::core::download::DownloadLifecycle::Running
                | crate::core::download::DownloadLifecycle::Created => TaskRunState::Running,
                crate::core::download::DownloadLifecycle::Paused => TaskRunState::Paused,
                crate::core::download::DownloadLifecycle::Completed => TaskRunState::Completed,
                crate::core::download::DownloadLifecycle::Failed => TaskRunState::Failed,
                // Ready/Cancelled: not actively downloading either way.
                _ => TaskRunState::Paused,
            };
            tasks.push(TaskView {
                task_id: shared.request.download_id.clone(),
                engine: EngineKind::Http,
                state,
                user_priority: priorities.get(&shared.request.download_id).copied(),
                smoothed_bps: shared.telemetry_bps() as f64,
                waited_secs: 0,
            });
        }
        for record in self.torrent_repo.list_tasks().unwrap_or_default() {
            if record.state != "running" {
                continue;
            }
            let Some(hash) = &record.info_hash else {
                continue;
            };
            let Some(handle) = self.torrents.handle_for(hash) else {
                continue;
            };
            let stats = self.torrents.stats(&handle);
            let smoothed = stats
                .live
                .as_ref()
                .map(|l| l.download_speed.as_bytes())
                .unwrap_or(0);
            tasks.push(TaskView {
                task_id: record.id.clone(),
                engine: EngineKind::Torrent,
                state: if stats.finished {
                    TaskRunState::Completed
                } else {
                    TaskRunState::Running
                },
                user_priority: priorities.get(&record.id).copied(),
                smoothed_bps: smoothed as f64,
                waited_secs: 0,
            });
        }

        // Playback sessions → views. Only sessions backed by a
        // DOWNLOAD task (in-progress HTTP / torrent) carry an
        // allocation; local/finished files need none.
        let playback = self
            .playback
            .active_sessions_internal()
            .into_iter()
            .filter_map(|(info, owner)| {
                let (engine, task_id) = match (info.source_kind, &owner) {
                    ("part", Some(owner)) => (EngineKind::Http, Some(owner.clone())),
                    ("torrent", Some(owner)) => (EngineKind::Torrent, Some(owner.clone())),
                    _ => return None,
                };
                Some(PlaybackView {
                    session_id: info.session_id,
                    task_id,
                    engine,
                    state: PlaybackSchedulingState::Playing,
                    buffered_ahead_secs: info.buffered_ahead_secs,
                    position_ms: (info.last_position_ms > 0).then_some(info.last_position_ms),
                })
            })
            .collect();

        let cap = self.global_bandwidth_bps.load(Ordering::Relaxed);
        SchedulerSnapshot {
            now_ms: crate::core::time::now_unix_ms(),
            network: Default::default(),
            limits: super::model::GlobalLimits {
                bandwidth_bps: (cap > 0).then_some(cap),
            },
            tasks,
            playback,
        }
    }
}

/// Where decisions go — implemented by the engine wiring itself.
pub trait SnapshotProvider: Send + Sync {
    fn collect(&self) -> SchedulerSnapshot;
}

/// The running scheduler: config + provider + engines, with the §17
/// decision diff, §21 aging stamps, and the §33/§19 hysteresis memory.
pub struct SchedulerRuntime {
    config: std::sync::RwLock<SchedulerConfig>,
    provider: Box<dyn SnapshotProvider>,
    downloads: DownloadEngine,
    torrents: TorrentEngine,
    /// §46 runtime state — in-memory only, rebuilt on restart.
    last_rates: Mutex<HashMap<String, u64>>,
    last_torrent_rate: Mutex<u64>,
    last_alloc_ms: Mutex<HashMap<String, i64>>,
    last_states: Mutex<HashMap<String, PlaybackSchedulingState>>,
    trigger: AtomicBool,
    pub tick_interval: Duration,
    /// §140: decision ring buffer — the LAST applied decisions, capped
    /// (never unbounded history).
    decision_ring: Mutex<std::collections::VecDeque<(i64, AllocationDecision)>>,
    /// §141 metrics: counters + last tick latency.
    metrics: Mutex<SchedulerMetricsState>,
}

/// §141: internal metrics — real counters, no telemetry upload.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerMetrics {
    pub tick_ms: u64,
    pub ticks: u64,
    pub active_tasks: u64,
    pub active_playback: u64,
    pub allocation_changes: u64,
    pub seek_events: u64,
    pub rebuffer_events: u64,
}

#[derive(Debug, Default)]
struct SchedulerMetricsState {
    tick_ms: u64,
    ticks: u64,
    allocation_changes: u64,
}

const DECISION_RING_CAP: usize = 256;

impl SchedulerRuntime {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: SchedulerConfig,
        provider: Box<dyn SnapshotProvider>,
        downloads: DownloadEngine,
        torrents: TorrentEngine,
    ) -> Arc<SchedulerRuntime> {
        Arc::new(SchedulerRuntime {
            config: std::sync::RwLock::new(config),
            provider,
            downloads,
            torrents,
            last_rates: Mutex::new(HashMap::new()),
            last_torrent_rate: Mutex::new(0),
            last_alloc_ms: Mutex::new(HashMap::new()),
            last_states: Mutex::new(HashMap::new()),
            trigger: AtomicBool::new(false),
            tick_interval: Duration::from_millis(500), // §18 low-frequency
            decision_ring: Mutex::new(std::collections::VecDeque::new()),
            metrics: Mutex::new(SchedulerMetricsState::default()),
        })
    }

    pub fn config(&self) -> SchedulerConfig {
        self.config
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set_config(&self, config: SchedulerConfig) {
        *self.config.write().unwrap_or_else(PoisonError::into_inner) = config;
        self.trigger();
    }

    /// §49: event-driven trigger — the next loop pass runs a tick
    /// immediately.
    pub fn trigger(&self) {
        self.trigger.store(true, Ordering::Relaxed);
    }

    /// §79: a fresh snapshot from the provider (for the IPC DTO).
    pub fn collect_snapshot(&self) -> SchedulerSnapshot {
        self.provider.collect()
    }

    /// §79/§76: the LATEST applied decisions (the last diff).
    pub fn last_applied(&self) -> Vec<AllocationDecision> {
        let rates = self
            .last_rates
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut out: Vec<AllocationDecision> = Vec::new();
        for (task_id, bps) in rates.iter() {
            out.push(AllocationDecision {
                task_id: task_id.clone(),
                engine: EngineKind::Http,
                priority_class: super::config::PriorityClass::Normal,
                effective_weight: 0.0,
                allocated_bps: *bps,
                min_bps: 0,
                target_bps: 0,
                session_id: None,
                reason: DecisionReason {
                    code: ReasonCode::FairShare,
                    detail: "last applied rate".into(),
                },
            });
        }
        let t = *self
            .last_torrent_rate
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if t > 0 {
            out.push(AllocationDecision {
                task_id: "*torrent-session*".into(),
                engine: EngineKind::Torrent,
                priority_class: super::config::PriorityClass::Normal,
                effective_weight: 0.0,
                allocated_bps: t,
                min_bps: 0,
                target_bps: 0,
                session_id: None,
                reason: DecisionReason {
                    code: ReasonCode::FairShare,
                    detail: "last applied torrent session rate".into(),
                },
            });
        }
        out.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        out
    }

    /// §17/§55: one reconciliation pass. Returns the decisions APPLIED
    /// (the diff) — engines only hear about CHANGES.
    pub fn tick(&self) -> Vec<AllocationDecision> {
        let tick_started = std::time::Instant::now();
        let config = self.config();
        let mut snapshot = self.provider.collect();
        let now = snapshot.now_ms;

        // §21 aging input: waited = now − last nonzero allocation.
        {
            let stamps = self
                .last_alloc_ms
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for task in &mut snapshot.tasks {
                task.waited_secs = stamps
                    .get(&task.task_id)
                    .map(|t| ((now - t).max(0) / 1000) as u64)
                    .unwrap_or(0);
            }
        }

        let previous_states = self
            .last_states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let decisions = policy::compute(&snapshot, &previous_states, &config);

        // §33 hysteresis memory: record the classified states.
        {
            let mut states = self
                .last_states
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for view in &snapshot.playback {
                let need =
                    policy::classify_playback(view, states.get(&view.session_id).copied(), &config);
                states.insert(view.session_id.clone(), need.state);
            }
            states.retain(|id, _| snapshot.playback.iter().any(|v| &v.session_id == id));
        }

        // §17 diff + apply.
        let mut applied: Vec<AllocationDecision> = Vec::new();
        let mut torrent_total: u64 = 0;
        {
            let last = self
                .last_rates
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for d in &decisions {
                if d.engine == EngineKind::Torrent {
                    torrent_total += d.allocated_bps;
                    continue;
                }
                if last.get(&d.task_id) != Some(&d.allocated_bps) {
                    self.downloads
                        .set_task_rate_bps(&d.task_id, d.allocated_bps);
                    applied.push(d.clone());
                }
            }
        }
        {
            let mut last_t = self
                .last_torrent_rate
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if *last_t != torrent_total {
                // §12: public librqbit API, session-level.
                self.torrents.set_download_rate_bps(if torrent_total == 0 {
                    None
                } else {
                    Some(torrent_total)
                });
                *last_t = torrent_total;
                for d in decisions.iter().filter(|d| d.engine == EngineKind::Torrent) {
                    applied.push(d.clone());
                }
            }
        }

        // §21 stamps + §62 garbage guard.
        {
            let mut stamps = self
                .last_alloc_ms
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let mut rates = self
                .last_rates
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for d in &decisions {
                if d.allocated_bps > 0 {
                    stamps.insert(d.task_id.clone(), now);
                }
            }
            // §144: tasks/playback that vanished lose their stamps AND
            // their last rate — no dangling references survive.
            stamps.retain(|id, _| {
                snapshot.tasks.iter().any(|t| &t.task_id == id)
                    || snapshot
                        .playback
                        .iter()
                        .any(|v| v.task_id.as_deref() == Some(id))
            });
            rates.retain(|id, _| {
                snapshot.tasks.iter().any(|t| &t.task_id == id)
                    || snapshot
                        .playback
                        .iter()
                        .any(|v| v.task_id.as_deref() == Some(id))
            });
        }

        // §140: decision ring (capped) + §141 metrics (real counters).
        {
            let mut ring = self
                .decision_ring
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for d in &applied {
                ring.push_back((now, d.clone()));
            }
            while ring.len() > DECISION_RING_CAP {
                ring.pop_front();
            }
            let mut metrics = self.metrics.lock().unwrap_or_else(PoisonError::into_inner);
            metrics.tick_ms = tick_started.elapsed().as_millis() as u64;
            metrics.ticks += 1;
            metrics.allocation_changes += applied.len() as u64;
        }

        applied
    }

    /// §140/§141: the applied-decision ring (oldest first) + live
    /// metrics. Real data only (§77).
    pub fn observability(&self) -> (Vec<(i64, AllocationDecision)>, SchedulerMetrics) {
        let ring = self
            .decision_ring
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let state = self.metrics.lock().unwrap_or_else(PoisonError::into_inner);
        let mut metrics = SchedulerMetrics {
            tick_ms: state.tick_ms,
            ticks: state.ticks,
            allocation_changes: state.allocation_changes,
            ..SchedulerMetrics::default()
        };
        let snapshot = self.provider.collect();
        metrics.active_tasks = snapshot
            .tasks
            .iter()
            .filter(|t| t.state == TaskRunState::Running)
            .count() as u64;
        metrics.active_playback = snapshot.playback.len() as u64;
        metrics.seek_events = snapshot
            .playback
            .iter()
            .filter(|p| p.state == PlaybackSchedulingState::Seeking)
            .count() as u64;
        metrics.rebuffer_events = snapshot
            .playback
            .iter()
            .filter(|p| {
                matches!(
                    p.state,
                    PlaybackSchedulingState::LowBuffer | PlaybackSchedulingState::CriticalBuffer
                )
            })
            .count() as u64;
        (ring.iter().cloned().collect(), metrics)
    }

    /// §18/§49: the loop — low-frequency tick, immediate on trigger.
    /// Lives for the app's lifetime; process exit ends it (§132: the
    /// loop holds only Arcs, nothing to join on shutdown).
    pub fn spawn_loop(self: &Arc<Self>) {
        let runtime = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            loop {
                runtime.tick();
                // Event-driven fast path: a trigger during the tick
                // (state change) skips the sleep; otherwise wait out
                // the interval (§18 low-frequency).
                if runtime.trigger.swap(false, Ordering::Relaxed) {
                    continue;
                }
                tokio::time::sleep(runtime.tick_interval).await;
            }
        });
    }
}
