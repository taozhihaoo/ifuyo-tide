//! Scheduler policy core (M7(上) §20/§21/§22/§27–§34/§55/§57–§59/
//! §91–§96).
//!
//! PURE and DETERMINISTIC (§51): `compute(snapshot, config)` →
//! decisions. No clocks, no globals, no engine calls. The algorithm is
//! the four-phase allocation of §55 with a hysteresis playback state
//! machine, weighted fair sharing, and a capped aging bonus. Each of
//! the spec's §91–§96 examples is a unit test in this file.

use super::config::{PriorityClass, SchedulerConfig};
use super::model::{
    AllocationDecision, DecisionReason, PlaybackSchedulingState, PlaybackView, ReasonCode,
    SchedulerSnapshot, TaskRunState, TaskView,
};

/// One schedulable entry: a task (possibly carrying a playback
/// session), classified.
struct Entry {
    task: TaskView,
    class: PriorityClass,
    state: Option<PlaybackSchedulingState>,
    session_id: Option<String>,
    reason: ReasonCode,
    detail: String,
    min_bps: u64,
    target_bps: u64,
}

/// What one playback session contributes to a task's class.
#[derive(Clone)]
pub struct PlaybackNeed {
    pub class: PriorityClass,
    pub state: PlaybackSchedulingState,
    pub reason: ReasonCode,
    pub detail: String,
}

/// §27–§34: classify one session from its REAL buffer seconds, with
/// the watermark hysteresis (§31: leaving CRITICAL requires reaching
/// `recovery_secs`, not 1 second).
pub fn classify_playback(
    session: &PlaybackView,
    previous: Option<PlaybackSchedulingState>,
    config: &SchedulerConfig,
) -> PlaybackNeed {
    match session.state {
        PlaybackSchedulingState::Paused => {
            return PlaybackNeed {
                class: PriorityClass::Maintenance, // §60: paused → demand removed/downgraded
                state: PlaybackSchedulingState::Paused,
                reason: ReasonCode::PlaybackPaused,
                detail: "playback paused — no allocation needed".into(),
            };
        }
        PlaybackSchedulingState::Ended | PlaybackSchedulingState::Error => {
            return PlaybackNeed {
                class: PriorityClass::Maintenance,
                state: session.state,
                reason: ReasonCode::PlaybackEnded,
                detail: "playback ended — demand removed".into(),
            };
        }
        PlaybackSchedulingState::Seeking => {
            return PlaybackNeed {
                class: PriorityClass::Emergency, // §32: seek elevation
                state: PlaybackSchedulingState::Seeking,
                reason: ReasonCode::SeekElevation,
                detail: "seek — new position elevated, old range invalidated".into(),
            };
        }
        _ => {}
    }

    let buffer = session.buffered_ahead_secs;
    let Some(buffer) = buffer else {
        // Unknown buffer (client not reporting yet): assume healthy —
        // an unreporting client must NOT trigger fake emergencies
        // (§77 no fake data, in both directions).
        return PlaybackNeed {
            class: PriorityClass::Playback,
            state: PlaybackSchedulingState::Playing,
            reason: ReasonCode::PlaybackHealthy,
            detail: "buffer unknown — assumed healthy".into(),
        };
    };

    let wm = &config.playback;
    // Hysteresis: a session already in CRITICAL stays there until the
    // RECOVERY watermark is reached (§31).
    let in_critical = match previous {
        Some(PlaybackSchedulingState::CriticalBuffer) => buffer < wm.recovery_secs as f64,
        _ => buffer < wm.critical_secs as f64,
    };
    let (state, class, reason) = if in_critical {
        (
            PlaybackSchedulingState::CriticalBuffer,
            PriorityClass::Emergency,
            ReasonCode::PlaybackCritical,
        )
    } else if buffer < wm.low_secs as f64 {
        (
            PlaybackSchedulingState::LowBuffer,
            PriorityClass::Emergency,
            ReasonCode::PlaybackLow,
        )
    } else if buffer < wm.target_secs as f64 || session.state == PlaybackSchedulingState::Startup {
        let state = if buffer < wm.startup_target_secs as f64 {
            PlaybackSchedulingState::Startup
        } else {
            PlaybackSchedulingState::Playing
        };
        (state, PriorityClass::Playback, ReasonCode::PlaybackStartup)
    } else {
        (
            PlaybackSchedulingState::Playing,
            PriorityClass::Playback,
            ReasonCode::PlaybackHealthy,
        )
    };
    PlaybackNeed {
        class,
        state,
        reason,
        detail: format!("buffer {buffer:.1}s"),
    }
}

/// §21: capped aging bonus — Background can grow by at most
/// `aging_max_bonus`, never enough to outrank playback (§21).
pub fn aging_bonus(waited_secs: u64, config: &SchedulerConfig) -> f64 {
    if waited_secs == 0 || config.aging_secs == 0 {
        return 0.0;
    }
    let raw = waited_secs as f64 / config.aging_secs as f64 * config.aging_max_bonus;
    raw.min(config.aging_max_bonus)
}

/// §55: the four-phase allocation. Deterministic: same snapshot +
/// config → same decisions (ties broken by task_id ordering). Owns its
/// data — no borrow juggling; every phase reads owned entries.
pub fn compute(
    snapshot: &SchedulerSnapshot,
    previous_states: &std::collections::HashMap<String, PlaybackSchedulingState>,
    config: &SchedulerConfig,
) -> Vec<AllocationDecision> {
    // ---- Phase A: classify everything ---------------------------------
    let mut entries: Vec<Entry> = Vec::new();
    let mut playback_task_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for session in &snapshot.playback {
        let previous = previous_states.get(&session.session_id).copied();
        let need = classify_playback(session, previous, config);
        let task_id = session.task_id.clone();
        if let Some(task_id) = &task_id {
            playback_task_ids.insert(task_id.clone());
        }
        if task_id.is_none() {
            // Playback of a COMPLETED local/finished file: no network
            // allocation decision at all (nothing to download).
            continue;
        }
        entries.push(Entry {
            task: TaskView {
                task_id: task_id.unwrap_or_default(),
                engine: session.engine,
                state: TaskRunState::Running,
                user_priority: None,
                smoothed_bps: 0.0,
                waited_secs: 0,
            },
            class: need.class,
            state: Some(need.state),
            session_id: Some(session.session_id.clone()),
            reason: need.reason,
            detail: need.detail,
            min_bps: if need.class == PriorityClass::Emergency {
                config.emergency_min_bps
            } else {
                0
            },
            target_bps: if matches!(
                need.class,
                PriorityClass::Emergency | PriorityClass::Playback
            ) {
                config.playback_target_bps
            } else {
                0
            },
        });
    }

    for task in &snapshot.tasks {
        // §54: the task already carries a playback classification —
        // merge the task's own telemetry/user priority into it. (This
        // check MUST precede the not-running branch: a task whose
        // session is paused would otherwise produce a duplicate
        // Maintenance entry alongside the playback entry.)
        if playback_task_ids.contains(&task.task_id) {
            if let Some(entry) = entries.iter_mut().find(|e| e.task.task_id == task.task_id) {
                entry.task = task.clone();
                // A paused/ended session on a stopped task ends the
                // demand entirely (§146).
                if task.state != TaskRunState::Running {
                    entry.class = PriorityClass::Maintenance;
                    entry.min_bps = 0;
                    entry.target_bps = 0;
                    entry.reason = ReasonCode::TaskNotRunning;
                }
            }
            continue;
        }
        if task.state != TaskRunState::Running {
            entries.push(Entry {
                task: task.clone(),
                class: PriorityClass::Maintenance,
                state: None,
                session_id: None,
                reason: ReasonCode::TaskNotRunning,
                detail: format!("task state {:?}", task.state),
                min_bps: 0,
                target_bps: 0,
            });
            continue;
        }
        let (class, reason, detail) = match task.user_priority {
            Some(user) => {
                let base = user.base_class();
                (
                    base,
                    ReasonCode::UserPriority,
                    format!(
                        "user {:?} → base {:?} ×{:.1}",
                        user,
                        base,
                        user.weight_multiplier()
                    ),
                )
            }
            None => (
                PriorityClass::Normal,
                ReasonCode::FairShare,
                "default share".to_string(),
            ),
        };
        entries.push(Entry {
            task: task.clone(),
            class,
            state: None,
            session_id: None,
            reason,
            detail,
            min_bps: 0,
            target_bps: 0,
        });
    }

    let cap = snapshot.limits.bandwidth_bps.unwrap_or(u64::MAX);
    let mut used_bps: u64 = 0;
    let mut allocated: Vec<AllocationDecision> = Vec::new();

    // Stable id order everywhere → deterministic output.
    entries.sort_by(|a, b| a.task.task_id.cmp(&b.task.task_id));

    let weights: Vec<f64> = entries
        .iter()
        .map(|e| {
            let user_mult = e
                .task
                .user_priority
                .map(|u| u.weight_multiplier())
                .unwrap_or(1.0);
            let aging = if e.session_id.is_none()
                && (e.class == PriorityClass::Normal || e.class == PriorityClass::Background)
            {
                aging_bonus(e.task.waited_secs, config)
            } else {
                0.0
            };
            (config.weight_of(e.class) * user_mult + aging).max(0.001)
        })
        .collect();

    // ---- Phase B: mandatory Emergency minimums (within the cap) ------
    for i in 0..entries.len() {
        if entries[i].class == PriorityClass::Emergency && entries[i].min_bps > 0 && used_bps < cap
        {
            let grant = entries[i].min_bps.min(cap - used_bps);
            used_bps += grant;
            let reason = entries[i].reason.clone();
            let detail = entries[i].detail.clone();
            entries[i].min_bps = 0; // consumed
            push_decision(&entries, i, &mut allocated, grant, &reason, &detail);
        }
    }

    // ---- Phase C: top Emergency/Playback entries up to their TARGET
    // before background sees the leftover (§26/§29/§92). The grant is
    // the REMAINING distance to target — a Phase-B minimum counts
    // toward it.
    for i in 0..entries.len() {
        if entries[i].target_bps == 0 || entries[i].task.state != TaskRunState::Running {
            continue;
        }
        let already: u64 = allocated
            .iter()
            .filter(|d| d.task_id == entries[i].task.task_id)
            .map(|d| d.allocated_bps)
            .sum();
        if already >= entries[i].target_bps || used_bps >= cap {
            continue;
        }
        let want = (entries[i].target_bps - already).min(cap - used_bps);
        used_bps += want;
        let reason = entries[i].reason.clone();
        let detail = format!("{}, target allocation", entries[i].detail);
        push_decision(&entries, i, &mut allocated, want, &reason, &detail);
    }

    let leftover = cap.saturating_sub(used_bps);
    // §95/§59: when playback is fully healthy (no LOW/CRITICAL/STARTUP/
    // SEEKING session), background keeps a guaranteed floor of the
    // leftover — playback must not starve everything forever.
    let playback_stressed = entries.iter().any(|e| {
        matches!(
            e.state,
            Some(PlaybackSchedulingState::CriticalBuffer)
                | Some(PlaybackSchedulingState::LowBuffer)
                | Some(PlaybackSchedulingState::Startup)
                | Some(PlaybackSchedulingState::Seeking)
        ) || e.class == PriorityClass::Emergency
    });
    let floor = if playback_stressed {
        0.0
    } else {
        config.background_floor_share
    };

    let live: Vec<usize> = (0..entries.len())
        .filter(|i| {
            entries[*i].task.state == TaskRunState::Running
                && !allocated
                    .iter()
                    .any(|d| d.task_id == entries[*i].task.task_id && d.allocated_bps > 0)
        })
        .collect();
    let total_left_weight: f64 = live.iter().map(|i| weights[*i]).sum();

    for i in live {
        let share = if total_left_weight > 0.0 {
            leftover as f64 * (weights[i] / total_left_weight)
        } else {
            leftover as f64 / entries.len().max(1) as f64
        };
        let mut grant = share as u64;
        let mut reason = entries[i].reason.clone();
        let mut detail = entries[i].detail.clone();
        if entries[i].class == PriorityClass::Background {
            let floor_bps = (leftover as f64 * floor) as u64;
            if grant < floor_bps && !playback_stressed {
                grant = floor_bps.min(leftover.saturating_sub(used_bps));
                reason = ReasonCode::BackgroundFloor;
                detail = "background floor (playback healthy)".into();
            } else if grant == 0 {
                reason = ReasonCode::BackgroundSpare;
                detail = "playback stressed — background at minimum".into();
            }
        }
        if entries[i].class == PriorityClass::Maintenance {
            grant = 0;
            reason = ReasonCode::TaskNotRunning;
        }
        used_bps = used_bps.saturating_add(grant);
        push_decision(&entries, i, &mut allocated, grant, &reason, &detail);
    }

    // §166 "no duplicate demand": one decision per task — merge any
    // duplicates by summing (Phase B + Phase C grants for the same
    // task are intentionally recorded separately per phase, but the
    // OUTPUT contract is a single decision per task).
    let mut merged: Vec<AllocationDecision> = Vec::new();
    allocated.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    for d in allocated {
        match merged.last_mut() {
            Some(last) if last.task_id == d.task_id => {
                last.allocated_bps += d.allocated_bps;
                last.min_bps = last.min_bps.max(d.min_bps);
                last.target_bps = last.target_bps.max(d.target_bps);
                if d.session_id.is_some() {
                    last.session_id = d.session_id.clone();
                }
            }
            _ => merged.push(d),
        }
    }
    merged
}

fn push_decision(
    entries: &[Entry],
    i: usize,
    allocated: &mut Vec<AllocationDecision>,
    allocated_bps: u64,
    reason: &ReasonCode,
    detail: &str,
) {
    let entry = &entries[i];
    let user_mult = entry
        .task
        .user_priority
        .map(|u| u.weight_multiplier())
        .unwrap_or(1.0);
    let base = match entry.class {
        PriorityClass::Emergency => 16.0,
        PriorityClass::Interactive => 6.0,
        PriorityClass::Playback => 4.0,
        PriorityClass::Normal => 2.0,
        PriorityClass::Background => 1.0,
        PriorityClass::Maintenance => 0.5,
    };
    allocated.push(AllocationDecision {
        task_id: entry.task.task_id.clone(),
        engine: entry.task.engine,
        priority_class: entry.class,
        effective_weight: base * user_mult,
        allocated_bps,
        min_bps: entry.min_bps,
        target_bps: entry.target_bps,
        session_id: entry.session_id.clone(),
        reason: DecisionReason {
            code: reason.clone(),
            detail: detail.to_string(),
        },
    });
}

// ---------------------------------------------------------------------
// §91–§96 examples as unit tests: each spec scenario asserts compute()
// output properties.
// ---------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::scheduler::config::UserPriority;
    use crate::core::scheduler::model::{EngineKind, GlobalLimits, NetworkSnapshot};

    fn config() -> SchedulerConfig {
        SchedulerConfig {
            global_bandwidth_bps: Some(20 * 1024 * 1024), // 20 MiB/s
            ..SchedulerConfig::default()
        }
    }

    fn snapshot(playback: Vec<PlaybackView>, tasks: Vec<TaskView>) -> SchedulerSnapshot {
        snapshot_with_cap(playback, tasks, 20 * 1024 * 1024)
    }

    fn snapshot_with_cap(
        playback: Vec<PlaybackView>,
        tasks: Vec<TaskView>,
        cap_bps: u64,
    ) -> SchedulerSnapshot {
        SchedulerSnapshot {
            now_ms: 1_000,
            network: NetworkSnapshot::default(),
            limits: GlobalLimits {
                bandwidth_bps: Some(cap_bps),
            },
            tasks,
            playback,
        }
    }

    fn running(id: &str) -> TaskView {
        TaskView {
            task_id: id.into(),
            engine: EngineKind::Http,
            state: TaskRunState::Running,
            user_priority: None,
            smoothed_bps: 0.0,
            waited_secs: 0,
        }
    }

    fn session(id: &str, task: &str, buffered: f64) -> PlaybackView {
        PlaybackView {
            session_id: id.into(),
            task_id: Some(task.into()),
            engine: EngineKind::Http,
            state: PlaybackSchedulingState::Playing,
            buffered_ahead_secs: Some(buffered),
            position_ms: None,
        }
    }

    /// A task's TOTAL allocation — it may hold several grants (Phase B
    /// minimum + Phase C target top-up are separate decisions).
    fn alloc_total(decisions: &[AllocationDecision], task: &str) -> u64 {
        decisions
            .iter()
            .filter(|d| d.task_id == task)
            .map(|d| d.allocated_bps)
            .sum()
    }

    fn alloc_of<'a>(decisions: &'a [AllocationDecision], task: &str) -> &'a AllocationDecision {
        decisions
            .iter()
            .find(|d| d.task_id == task)
            .unwrap_or_else(|| panic!("no decision for {task}"))
    }

    /// §91: healthy playback — moderate playback allocation, background
    /// receives the remaining capacity.
    #[test]
    fn example_91_healthy_playback_leaves_room() {
        let decisions = compute(
            &snapshot(vec![session("s1", "movie", 40.0)], vec![running("movie")]),
            &Default::default(),
            &config(),
        );
        let movie = alloc_of(&decisions, "movie");
        assert_eq!(movie.priority_class, PriorityClass::Playback);
        assert_eq!(
            alloc_total(&decisions, "movie"),
            6 * 1024 * 1024,
            "target allocation"
        );
        assert!(matches!(movie.reason.code, ReasonCode::PlaybackHealthy));
    }

    /// §92: buffer declining → playback urgency up, background down —
    /// BEFORE the buffer hits zero (§58).
    #[test]
    fn example_92_low_buffer_squeezes_background() {
        // §92's premise: the network is SLOW (download < consumption).
        // Model it with a 3 MiB/s global cap so allocation is scarce.
        let decisions = compute(
            &snapshot_with_cap(
                vec![session("s1", "movie", 8.0)],
                vec![running("movie"), running("bg1")],
                3 * 1024 * 1024,
            ),
            &Default::default(),
            &config(),
        );
        let movie = alloc_of(&decisions, "movie");
        assert_eq!(movie.priority_class, PriorityClass::Emergency);
        assert!(
            alloc_total(&decisions, "movie") >= 2 * 1024 * 1024,
            "emergency minimum"
        );
        let bg = alloc_of(&decisions, "bg1");
        assert!(
            alloc_total(&decisions, "bg1") < alloc_total(&decisions, "movie"),
            "background must yield to low-buffer playback"
        );
    }

    /// §93: seek — the new position is elevated with the reason saying
    /// so; old-range invalidation happened upstream (M4 notify_seek).
    #[test]
    fn example_93_seek_elevates_new_position() {
        let mut s = session("s1", "movie", 2.0);
        s.state = PlaybackSchedulingState::Seeking;
        let decisions = compute(
            &snapshot(vec![s], vec![running("movie")]),
            &Default::default(),
            &config(),
        );
        let movie = alloc_of(&decisions, "movie");
        assert_eq!(movie.priority_class, PriorityClass::Emergency);
        assert!(matches!(movie.reason.code, ReasonCode::SeekElevation));
    }

    /// §94: two downloads + one playback — allocation follows buffer
    /// health across states, within the global cap.
    #[test]
    fn example_94_two_downloads_one_playback() {
        let tasks = vec![running("movie"), running("dl-a"), running("dl-b")];
        let mk = |buffered: f64| {
            compute(
                &snapshot(vec![session("s1", "movie", buffered)], tasks.clone()),
                &Default::default(),
                &config(),
            )
        };
        let healthy = mk(40.0);
        assert_eq!(alloc_total(&healthy, "movie"), 6 * 1024 * 1024);
        let a_healthy = alloc_total(&healthy, "dl-a");
        assert!(a_healthy > 0, "healthy → background runs");

        let low = mk(8.0);
        assert!(alloc_of(&low, "movie").allocated_bps >= 2 * 1024 * 1024);
        let a_low = alloc_total(&low, "dl-a");
        assert!(a_low <= a_healthy, "background falls when buffer declines");

        let critical = mk(2.0);
        let movie_c = alloc_total(&critical, "movie");
        println!(
            "§94 movie healthy={} low={} critical={}",
            alloc_of(&healthy, "movie").allocated_bps,
            alloc_of(&low, "movie").allocated_bps,
            movie_c
        );
        assert!(movie_c >= alloc_total(&healthy, "movie"));
        let sum: u64 = critical.iter().map(|d| d.allocated_bps).sum();
        assert!(sum <= 20 * 1024 * 1024 + 1, "global cap respected");
    }

    /// §95: long-lived HEALTHY playback must not starve background
    /// forever — the floor kicks in.
    #[test]
    fn example_95_background_not_starved_forever() {
        let decisions = compute(
            &snapshot(
                vec![session("s1", "movie", 120.0)],
                vec![running("movie"), running("bg")],
            ),
            &Default::default(),
            &config(),
        );
        let bg = alloc_of(&decisions, "bg");
        assert!(
            alloc_total(&decisions, "bg") > 0,
            "healthy playback must release spare capacity"
        );
    }

    /// §96: multi-file torrent — the PLAYED file's task gets playback
    /// priority; the rest stays background.
    #[test]
    fn example_96_multi_file_torrent_prioritizes_played_file() {
        // Scarce capacity (4 MiB/s): the played file's emergency demand
        // is served before the rest of the torrent sees anything.
        let decisions = compute(
            &snapshot_with_cap(
                vec![session("s1", "torrent-movie", 5.0)],
                vec![running("torrent-movie"), running("torrent-rest")],
                4 * 1024 * 1024,
            ),
            &Default::default(),
            &config(),
        );
        let movie = alloc_of(&decisions, "torrent-movie");
        let rest = alloc_of(&decisions, "torrent-rest");
        assert!(
            alloc_total(&decisions, "torrent-movie") > alloc_total(&decisions, "torrent-rest"),
            "played file first"
        );
        assert_eq!(rest.priority_class, PriorityClass::Normal);
    }

    /// §21: aging lifts waiting background weight but stays capped.
    #[test]
    fn aging_is_capped() {
        let cfg = SchedulerConfig::default();
        assert_eq!(aging_bonus(0, &cfg), 0.0);
        let full = aging_bonus(600, &cfg);
        assert!(full <= cfg.aging_max_bonus + 1e-9, "capped");
        assert_eq!(full, cfg.aging_max_bonus);
    }

    /// §31: hysteresis — leaving CRITICAL requires the RECOVERY
    /// watermark, not 1 second.
    #[test]
    fn critical_hysteresis_uses_recovery_watermark() {
        let cfg = SchedulerConfig::default();
        let mut s = session("s1", "movie", 2.0);
        let need = classify_playback(&s, None, &cfg);
        assert_eq!(need.state, PlaybackSchedulingState::CriticalBuffer);
        // 5 s: above critical(4) but below recovery(10) → STAYS critical.
        s.buffered_ahead_secs = Some(5.0);
        let need = classify_playback(&s, Some(PlaybackSchedulingState::CriticalBuffer), &cfg);
        assert_eq!(need.state, PlaybackSchedulingState::CriticalBuffer);
        // 10 s: reaches recovery → leaves critical.
        s.buffered_ahead_secs = Some(10.0);
        let need = classify_playback(&s, Some(PlaybackSchedulingState::CriticalBuffer), &cfg);
        assert_ne!(need.state, PlaybackSchedulingState::CriticalBuffer);
    }

    /// §60: paused playback demands nothing — capacity released.
    #[test]
    fn paused_playback_releases_capacity() {
        let mut s = session("s1", "movie", 5.0);
        s.state = PlaybackSchedulingState::Paused;
        let decisions = compute(
            &snapshot(vec![s], vec![running("movie"), running("bg")]),
            &Default::default(),
            &config(),
        );
        let movie = alloc_of(&decisions, "movie");
        assert_eq!(movie.priority_class, PriorityClass::Maintenance);
        assert_eq!(alloc_total(&decisions, "movie"), 0);
        assert!(
            alloc_total(&decisions, "bg") > 0,
            "capacity released to background"
        );
    }

    /// §51: determinism — identical snapshot+config → identical output.
    #[test]
    fn compute_is_deterministic() {
        let snap = snapshot(
            vec![session("s1", "movie", 9.0)],
            vec![running("movie"), running("bg1"), running("bg2")],
        );
        let cfg = config();
        let a = compute(&snap, &Default::default(), &cfg);
        let b = compute(&snap, &Default::default(), &cfg);
        assert_eq!(a, b);
    }

    /// §108: same-class fairness — three Normal tasks share the
    /// leftover equally each tick; no task is starved to zero while
    /// capacity exists.
    #[test]
    fn same_class_tasks_fair_share() {
        let decisions = compute(
            &snapshot(vec![], vec![running("a"), running("b"), running("c")]),
            &Default::default(),
            &config(),
        );
        let a = alloc_total(&decisions, "a");
        let b = alloc_total(&decisions, "b");
        let c = alloc_total(&decisions, "c");
        assert!(a > 0 && b > 0 && c > 0, "no starvation with capacity");
        assert_eq!(a, b, "equal weights → equal shares");
        assert_eq!(b, c);
    }

    /// §114: property invariants over randomized snapshots —
    /// allocation ≥ 0 (by type), sum ≤ capacity, unique task ids,
    /// ended/paused/maintenance receive zero, determinism holds.
    #[test]
    fn property_invariants_hold_over_random_snapshots() {
        let mut state: u64 = 0x1234_5678_9ABC_DEF0;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for case in 0..100u32 {
            let cap_bps = 1 + next() % (50 * 1024 * 1024);
            let task_count = (next() % 12) as usize;
            let mut tasks = Vec::new();
            for i in 0..task_count {
                let mut t = running(&format!("t{i:03}"));
                t.state = match next() % 4 {
                    0 => TaskRunState::Running,
                    1 => TaskRunState::Paused,
                    2 => TaskRunState::Completed,
                    _ => TaskRunState::Running,
                };
                if next() % 5 == 0 {
                    t.user_priority = Some(UserPriority::High);
                }
                t.waited_secs = next() % 300;
                tasks.push(t);
            }
            let mut playback = Vec::new();
            for i in 0..(next() % 3) as usize {
                let mut p = session(&format!("s{i}"), &format!("t{i:03}"), (next() % 60) as f64);
                p.state = match next() % 3 {
                    0 => PlaybackSchedulingState::Playing,
                    1 => PlaybackSchedulingState::Paused,
                    _ => PlaybackSchedulingState::Seeking,
                };
                playback.push(p);
            }
            let snap = snapshot_with_cap(playback, tasks, cap_bps);
            let decisions = compute(&snap, &Default::default(), &config());

            // INVARIANT 1: sum(allocation) ≤ capacity.
            let sum: u64 = decisions.iter().map(|d| d.allocated_bps).sum();
            assert!(
                sum <= cap_bps,
                "case {case}: capacity overflow {sum} > {cap_bps}"
            );
            // INVARIANT 2: no orphan/duplicate decisions.
            let mut ids: Vec<&str> = decisions.iter().map(|d| d.task_id.as_str()).collect();
            ids.sort_unstable();
            let before = ids.len();
            ids.dedup();
            assert_eq!(ids.len(), before, "case {case}: duplicate decision");
            // INVARIANT 3: ended/paused/non-running receive zero.
            for d in &decisions {
                let view = snap
                    .tasks
                    .iter()
                    .find(|t| t.task_id == d.task_id)
                    .map(|t| t.state)
                    .unwrap_or(TaskRunState::Running);
                let paused_view = snap
                    .playback
                    .iter()
                    .find(|p| Some(&p.session_id) == d.session_id.as_ref())
                    .map(|p| p.state == PlaybackSchedulingState::Paused)
                    .unwrap_or(false);
                if view != TaskRunState::Running || paused_view {
                    assert_eq!(
                        d.allocated_bps, 0,
                        "case {case}: non-running {} got bandwidth",
                        d.task_id
                    );
                }
            }
        }
    }

    /// §23/§16: user priority lifts a task; emergency playback still
    /// outranks user-High, and the reason decomposes.
    #[test]
    fn user_priority_decomposes_reason() {
        // Constrained capacity: emergency playback is served FIRST, the
        // user-High download waits (§23) — with the reason decomposed.
        let mut high = running("high");
        high.user_priority = Some(UserPriority::High);
        let decisions = compute(
            &snapshot_with_cap(
                vec![session("s1", "movie", 5.0)],
                vec![running("movie"), high],
                5 * 1024 * 1024,
            ),
            &Default::default(),
            &config(),
        );
        let movie = alloc_of(&decisions, "movie");
        let high_d = alloc_of(&decisions, "high");
        assert!(
            alloc_total(&decisions, "movie") > alloc_total(&decisions, "high"),
            "§23: emergency playback outranks user-High"
        );
        assert!(matches!(high_d.reason.code, ReasonCode::UserPriority));
        assert!(high_d.reason.detail.contains("user"), "decomposable reason");
    }
}
