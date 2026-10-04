//! Scheduler IPC commands (M7(上) §78/§79/§87/§88).
//!
//! The frontend may express INTENT (priority, bandwidth limit) and may
//! READ a snapshot DTO — it can never inject paths, arbitrary task
//! ids are re-validated against the live registries (§88), and the
//! Rust scheduler is never exposed directly (§79).
//!
//! §72: user priorities and the bandwidth limit are PERSISTED (the
//! settings table, `sched:` namespace); scores/allocations are not.

use crate::app::state::AppState;
use crate::core::scheduler::model::SchedulerSnapshot;
use crate::core::scheduler::UserPriority;
use crate::error::TideError;
use serde::Serialize;
use tauri::State;

const KEY_GLOBAL_BANDWIDTH: &str = "sched:global_bandwidth_bps";
const KEY_PRIORITY_PREFIX: &str = "sched:prio:";

/// §79: a frontend-facing DTO — never the internal scheduler.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerSnapshotDto {
    pub global_bandwidth_bps: Option<u64>,
    pub tasks: Vec<SchedulerTaskDto>,
    pub playback: Vec<SchedulerPlaybackDto>,
    pub decisions: Vec<SchedulerDecisionDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerTaskDto {
    pub task_id: String,
    pub engine: String,
    pub state: String,
    pub user_priority: Option<String>,
    pub smoothed_bps: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerPlaybackDto {
    pub session_id: String,
    pub task_id: Option<String>,
    pub state: String,
    pub buffered_ahead_secs: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerDecisionDto {
    pub task_id: String,
    pub priority_class: String,
    pub allocated_bps: u64,
    pub reason_code: String,
    pub reason_detail: String,
}

/// §88: the priority must be one of the five known values ("normal"
/// CLEARS the override); unknown strings are rejected, never applied
/// blindly.
fn parse_priority(raw: &str) -> Result<Option<UserPriority>, TideError> {
    match raw {
        "highest" => Ok(Some(UserPriority::Highest)),
        "high" => Ok(Some(UserPriority::High)),
        "normal" | "" => Ok(None),
        "low" => Ok(Some(UserPriority::Low)),
        "background" => Ok(Some(UserPriority::Background)),
        other => Err(TideError::InvalidInput(format!(
            "unknown priority: {other}"
        ))),
    }
}

/// §23/§72: sets a task's user priority (persisted + applied at the
/// next tick). "normal" clears the override. The task must EXIST in
/// the current app state (§88) — the snapshot is the live registry.
#[tauri::command]
pub fn scheduler_set_task_priority(
    state: State<'_, AppState>,
    task_id: String,
    priority: String,
) -> Result<(), TideError> {
    let parsed = parse_priority(&priority)?;
    // §88 ownership/existence: the task must be known to the live
    // snapshot (download or torrent task).
    let snapshot = build_snapshot(&state)?;
    let exists = snapshot.tasks.iter().any(|t| t.task_id == task_id)
        || snapshot
            .playback
            .iter()
            .any(|p| p.task_id.as_deref() == Some(task_id.as_str()));
    if !exists {
        return Err(TideError::NotFound(format!("unknown task: {task_id}")));
    }

    let mut priorities = state
        .user_priorities
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match parsed {
        Some(p) => {
            priorities.insert(task_id.clone(), p);
        }
        None => {
            priorities.remove(&task_id);
        }
    }
    drop(priorities);

    // §72 persistence (best-effort write-through).
    state.with_conn(|conn| {
        let repo = crate::persistence::repositories::SettingsRepository;
        if let Some(p) = parsed {
            repo.set(conn, &format!("{KEY_PRIORITY_PREFIX}{task_id}"), p_str(p))?;
        } else {
            repo.delete(conn, &format!("{KEY_PRIORITY_PREFIX}{task_id}"))?;
        }
        Ok(())
    })?;
    state.scheduler.trigger();
    Ok(())
}

fn p_str(p: UserPriority) -> &'static str {
    match p {
        UserPriority::Highest => "highest",
        UserPriority::High => "high",
        UserPriority::Normal => "normal",
        UserPriority::Low => "low",
        UserPriority::Background => "background",
    }
}

/// §24: the global download bandwidth cap (bytes/sec; 0 = unlimited),
/// persisted and applied at the next tick.
#[tauri::command]
pub fn scheduler_set_global_bandwidth_limit(
    state: State<'_, AppState>,
    bps: u64,
) -> Result<(), TideError> {
    state
        .global_bandwidth_bps
        .store(bps, std::sync::atomic::Ordering::Relaxed);
    state.with_conn(|conn| {
        let repo = crate::persistence::repositories::SettingsRepository;
        repo.set(conn, KEY_GLOBAL_BANDWIDTH, &bps.to_string())
    })?;
    state.scheduler.trigger();
    Ok(())
}

/// §78/§79: the scheduler snapshot as a DTO (also carries the LATEST
/// decisions the runtime applied, for the debug panel §76).
#[tauri::command]
pub fn scheduler_get_snapshot(
    state: State<'_, AppState>,
) -> Result<SchedulerSnapshotDto, TideError> {
    let snapshot = build_snapshot(&state)?;
    let decisions = state.scheduler.last_applied();
    Ok(SchedulerSnapshotDto {
        global_bandwidth_bps: snapshot.limits.bandwidth_bps,
        tasks: snapshot
            .tasks
            .iter()
            .map(|t| SchedulerTaskDto {
                task_id: t.task_id.clone(),
                engine: format!("{:?}", t.engine).to_lowercase(),
                state: format!("{:?}", t.state).to_lowercase(),
                user_priority: t.user_priority.map(p_str).map(str::to_string),
                smoothed_bps: t.smoothed_bps as u64,
            })
            .collect(),
        playback: snapshot
            .playback
            .iter()
            .map(|p| SchedulerPlaybackDto {
                session_id: p.session_id.clone(),
                task_id: p.task_id.clone(),
                state: format!("{:?}", p.state).to_lowercase(),
                buffered_ahead_secs: p.buffered_ahead_secs,
            })
            .collect(),
        decisions: decisions
            .iter()
            .map(|d| SchedulerDecisionDto {
                task_id: d.task_id.clone(),
                priority_class: format!("{:?}", d.priority_class).to_lowercase(),
                allocated_bps: d.allocated_bps,
                reason_code: format!("{:?}", d.reason.code).to_lowercase(),
                reason_detail: d.reason.detail.clone(),
            })
            .collect(),
    })
}

fn build_snapshot(state: &State<'_, AppState>) -> Result<SchedulerSnapshot, TideError> {
    // The runtime's provider owns snapshot construction; reuse it via
    // a tick-free collect through the runtime's stored provider is not
    // exposed — rebuild the same view from the registries here.
    Ok(state.scheduler.collect_snapshot())
}

/// Loads persisted scheduler preferences at startup (§72): global
/// bandwidth + per-task user priorities into the runtime registries.
pub fn load_persisted(state: &AppState) {
    let _ = state.with_conn(|conn| {
        let repo = crate::persistence::repositories::SettingsRepository;
        if let Some((_, value)) = repo
            .get_by_prefix(conn, KEY_GLOBAL_BANDWIDTH)?
            .into_iter()
            .next()
        {
            if let Ok(bps) = value.parse::<u64>() {
                state
                    .global_bandwidth_bps
                    .store(bps, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let mut priorities = state
            .user_priorities
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (key, value) in repo.get_by_prefix(conn, KEY_PRIORITY_PREFIX)? {
            if let Some(task_id) = key.strip_prefix(KEY_PRIORITY_PREFIX) {
                if let Some(p) = parse_priority(&value).unwrap_or(None) {
                    priorities.insert(task_id.to_string(), p);
                }
            }
        }
        Ok(())
    });
}
