//! M7(上) scheduler integration: the runtime's decisions actually
//! reach the data plane (per-task HTTP rate via the token bucket), the
//! §17 decision diff only re-applies CHANGES, and the torrent session
//! rate flows through librqbit's public API.

mod common;

use common::pattern_sha256;
use ifuyo_tide_lib::core::download::file_sink::OverwritePolicy;
use ifuyo_tide_lib::core::download::url::validate_download_url;
use ifuyo_tide_lib::core::download::{DownloadEngine, DownloadRequest};
use ifuyo_tide_lib::core::scheduler::model::{
    EngineKind, GlobalLimits, NetworkSnapshot, SchedulerSnapshot, TaskRunState, TaskView,
};
use ifuyo_tide_lib::core::scheduler::runtime::SchedulerRuntime;
use ifuyo_tide_lib::core::scheduler::SchedulerConfig;
use ifuyo_tide_lib::core::torrent::{TorrentEngine, TorrentEngineConfig};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m7-{}-{}",
        label,
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

fn engine() -> DownloadEngine {
    DownloadEngine::new(Arc::new(
        ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap(),
    ))
}

async fn torrent_engine(label: &str) -> TorrentEngine {
    let config = TorrentEngineConfig {
        metadata_dir: temp_dir(&format!("meta-{label}")),
        dht: false,
        listen_loopback_only: true,
        initial_peers: None,
    };
    TorrentEngine::new(&config).await.expect("torrent engine")
}

// ---------------------------------------------------------------------
// §25: the per-task token bucket REALLY paces the HTTP data plane.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn per_task_rate_actually_paces_the_download() {
    let server = common::spawn().await;
    let dir = temp_dir("pace");
    let engine = engine();
    let url = validate_download_url(&server.url("/medium.bin"), true).unwrap();
    let request = DownloadRequest::new(
        url,
        dir,
        None,
        OverwritePolicy::Overwrite,
        Some((
            ifuyo_tide_lib::core::download::checksum::ChecksumAlgorithm::Sha256,
            pattern_sha256(1024 * 1024),
        )),
    )
    .unwrap();
    let shared = engine.start(request, None).await.expect("start");
    let id = shared.request.download_id.clone();

    // 256 KiB/s over a 1 MiB file: at least ~3.5 s even with the burst.
    engine.set_task_rate_bps(&id, 256 * 1024);

    let started = Instant::now();
    let deadline = started + Duration::from_secs(60);
    loop {
        let status = shared.status();
        if status.lifecycle == ifuyo_tide_lib::core::download::DownloadLifecycle::Completed {
            break;
        }
        if status.lifecycle == ifuyo_tide_lib::core::download::DownloadLifecycle::Failed {
            panic!("download failed under rate limit");
        }
        if Instant::now() > deadline {
            panic!("rate-limited download did not finish in 60s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let elapsed = started.elapsed();
    println!("rate-limited 1 MiB @256 KiB/s took {elapsed:?}");
    assert!(
        elapsed >= Duration::from_secs(1),
        "the rate limiter must actually pace (unlimited ≈0.1s): finished in {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "the limiter must not stall forever: {elapsed:?}"
    );
}

// ---------------------------------------------------------------------
// §17/§55: the runtime applies decision CHANGES to the data plane and
// skips unchanged ones (the diff) — driven through the REAL policy.
// ---------------------------------------------------------------------

struct MutableProvider(Mutex<SchedulerSnapshot>);

/// Arc forwarder so the test can keep mutating the snapshot after the
/// runtime owns the provider.
struct ProviderHandle(Arc<MutableProvider>);

impl ifuyo_tide_lib::core::scheduler::SnapshotProvider for ProviderHandle {
    fn collect(&self) -> SchedulerSnapshot {
        self.0.collect()
    }
}

impl ifuyo_tide_lib::core::scheduler::SnapshotProvider for MutableProvider {
    fn collect(&self) -> SchedulerSnapshot {
        self.0.lock().unwrap().clone()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn runtime_tick_diffs_and_applies_to_the_data_plane() {
    let server = common::spawn().await;
    let dir = temp_dir("runtime");
    let engine = engine();
    let torrents = torrent_engine("rt").await;

    let url = validate_download_url(&server.url("/medium.bin"), true).unwrap();
    let request = DownloadRequest::new(url, dir, None, OverwritePolicy::Overwrite, None).unwrap();
    let shared = engine.start(request, None).await.expect("start");
    let id = shared.request.download_id.clone();
    // Idle under test: the pacing, not the transfer, is the subject.
    engine.pause(&id).await.expect("pause");

    // One running task, no playback: the policy gives a Normal task the
    // FULL cap as its fair share (deterministic).
    let make_snapshot = |cap_bps: u64| SchedulerSnapshot {
        now_ms: 1_000,
        network: NetworkSnapshot::default(),
        limits: GlobalLimits {
            bandwidth_bps: Some(cap_bps),
        },
        tasks: vec![TaskView {
            task_id: id.clone(),
            engine: EngineKind::Http,
            state: TaskRunState::Running,
            user_priority: None,
            smoothed_bps: 0.0,
            waited_secs: 0,
        }],
        playback: vec![],
    };

    let applied: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let applied = applied.clone();
        engine.set_on_rate_change(Arc::new(move |bps| {
            applied.lock().unwrap().push(bps);
        }));
    }

    let provider = Arc::new(MutableProvider(Mutex::new(make_snapshot(100_000))));
    let runtime = SchedulerRuntime::new(
        SchedulerConfig::default(),
        Box::new(ProviderHandle(provider.clone())),
        engine.clone(),
        torrents,
    );

    // Tick 1: cap 100k → the fair share (full cap for one task) applies.
    runtime.tick();
    assert_eq!(
        engine.get_sync(&id).unwrap().limiter().rate_bps(),
        100_000,
        "the policy decision must reach the data plane"
    );
    // Tick 2: UNCHANGED snapshot → the diff must not re-apply.
    runtime.tick();
    // Tick 3: cap raised → the new rate applies.
    {
        let mut snap = provider.0.lock().unwrap();
        *snap = make_snapshot(250_000);
    }
    runtime.tick();
    assert_eq!(engine.get_sync(&id).unwrap().limiter().rate_bps(), 250_000);

    let rates = applied.lock().unwrap();
    assert_eq!(
        rates.as_slice(),
        &[100_000, 250_000],
        "§17: the diff must skip unchanged allocations, apply changes once"
    );
}

// ---------------------------------------------------------------------
// §115–§121: headless deterministic simulation — multiple ticks with
// evolving buffers/caps, NO real network. The pure policy runs against
// synthesized snapshots; states carry across ticks (hysteresis).
// ---------------------------------------------------------------------
#[cfg(test)]
mod simulation {
    use ifuyo_tide_lib::core::scheduler::model::{
        AllocationDecision, EngineKind, GlobalLimits, NetworkSnapshot, PlaybackSchedulingState,
        PlaybackView, ReasonCode, SchedulerSnapshot, TaskRunState, TaskView,
    };
    use ifuyo_tide_lib::core::scheduler::policy;
    use ifuyo_tide_lib::core::scheduler::SchedulerConfig;

    const MIB: u64 = 1024 * 1024;

    fn task(id: &str) -> TaskView {
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

    fn snap(cap_bps: u64, playback: Vec<PlaybackView>, tasks: Vec<TaskView>) -> SchedulerSnapshot {
        SchedulerSnapshot {
            now_ms: 0,
            network: NetworkSnapshot::default(),
            limits: GlobalLimits {
                bandwidth_bps: Some(cap_bps),
            },
            tasks,
            playback,
        }
    }

    fn total(decisions: &[AllocationDecision], id: &str) -> u64 {
        decisions
            .iter()
            .filter(|d| d.task_id == id)
            .map(|d| d.allocated_bps)
            .sum()
    }

    fn alloc_of<'a>(decisions: &'a [AllocationDecision], id: &str) -> &'a AllocationDecision {
        decisions
            .iter()
            .find(|d| d.task_id == id)
            .unwrap_or_else(|| panic!("no decision for {id}"))
    }

    /// Scenario 1 (§116): 0-buffer playback gets the EMERGENCY
    /// allocation while background is dominated.
    #[test]
    fn sim1_zero_buffer_playback_gets_emergency() {
        let cfg = SchedulerConfig::default();
        let decisions = policy::compute(
            &snap(
                20 * MIB,
                vec![session("s", "movie", 0.0)],
                vec![task("movie"), task("a"), task("b")],
            ),
            &Default::default(),
            &cfg,
        );
        let movie = total(&decisions, "movie");
        assert!(movie >= cfg.emergency_min_bps, "emergency minimum granted");
        let bg = total(&decisions, "a") + total(&decisions, "b");
        assert!(
            movie > bg,
            "playback dominates under 0-buffer: {movie} vs {bg}"
        );
    }

    /// Scenario 2 (§117): a 120 s buffer caps playback at target and
    /// releases the rest of the capacity to background.
    #[test]
    fn sim2_healthy_buffer_releases_capacity() {
        let cfg = SchedulerConfig::default();
        let decisions = policy::compute(
            &snap(
                20 * MIB,
                vec![session("s", "movie", 120.0)],
                vec![task("movie"), task("bg")],
            ),
            &Default::default(),
            &cfg,
        );
        let movie = total(&decisions, "movie");
        let bg = total(&decisions, "bg");
        assert_eq!(movie, cfg.playback_target_bps, "capped at target");
        assert!(
            bg >= (20 * MIB - movie),
            "released capacity flows to background (movie={movie} bg={bg})"
        );
    }

    /// Scenario 3 (§118): a seek flips the session to SEEKING — the
    /// new range's allocation rises to emergency level and the reason
    /// records the elevation; there is no old-range demand left to
    /// weight (one session = one current position).
    #[test]
    fn sim3_seek_reroutes_allocation() {
        let cfg = SchedulerConfig::default();
        let low = policy::compute(
            &snap(
                20 * MIB,
                vec![session("s", "movie", 0.5)],
                vec![task("movie")],
            ),
            &Default::default(),
            &cfg,
        );
        let mut seeking = session("s", "movie", 0.5);
        seeking.state = PlaybackSchedulingState::Seeking;
        let elevated = policy::compute(
            &snap(20 * MIB, vec![seeking], vec![task("movie")]),
            &Default::default(),
            &cfg,
        );
        assert!(
            total(&elevated, "movie") >= total(&low, "movie"),
            "seek allocation must rise to at least the low-buffer level"
        );
        assert!(matches!(
            alloc_of(&elevated, "movie").reason.code,
            ReasonCode::SeekElevation
        ));
    }

    /// Scenario 4 (§119): three background tasks over many ticks —
    /// equal weights → equal shares; NO task is ever starved.
    #[test]
    fn sim4_background_fairness_over_ticks() {
        let cfg = SchedulerConfig::default();
        let ids = ["b0", "b1", "b2"];
        let mut totals = [0u64; 3];
        for tick in 0..12u64 {
            let tasks: Vec<TaskView> = ids.iter().map(|id| task(id)).collect();
            let decisions =
                policy::compute(&snap(10 * MIB, vec![], tasks), &Default::default(), &cfg);
            for (i, id) in ids.iter().enumerate() {
                let grant = total(&decisions, id);
                assert!(grant > 0, "tick {tick}: {id} starved");
                totals[i] += grant;
            }
        }
        assert_eq!(totals[0], totals[1], "fair shares");
        assert_eq!(totals[1], totals[2]);
    }

    /// Scenario 5 (§120): the cap moves 20→10→2→30 MiB/s — every tick
    /// respects the CURRENT cap; emergency playback still protected.
    #[test]
    fn sim5_adapts_to_bandwidth_changes_without_overflow() {
        let cfg = SchedulerConfig::default();
        for cap_mib in [20u64, 10, 2, 30] {
            let decisions = policy::compute(
                &snap(
                    cap_mib * MIB,
                    vec![session("s", "movie", 9.0)],
                    vec![task("movie"), task("bg")],
                ),
                &Default::default(),
                &cfg,
            );
            let sum: u64 = decisions.iter().map(|d| d.allocated_bps).sum();
            assert!(sum <= cap_mib * MIB, "cap {cap_mib} MiB: sum {sum}");
            let floor = cfg.emergency_min_bps.min(cap_mib * MIB);
            assert!(total(&decisions, "movie") >= floor);
        }
    }

    /// Scenario 6 (§121): availability [0,100)+[200,400) MiB, playhead
    /// at 150 MiB — the RangeSet identifies the hole [150,200) as the
    /// ONLY missing window inside the remaining playback path.
    #[test]
    fn sim6_availability_hole_identified() {
        use ifuyo_tide_lib::core::scheduler::RangeSet;
        let availability = RangeSet::from_ranges(vec![(0, 100 * MIB), (200 * MIB, 400 * MIB)]);
        let missing = availability.missing_from(150 * MIB, 400 * MIB);
        assert_eq!(missing.ranges(), &[(150 * MIB, 200 * MIB)]);
    }

    /// §166: invariants under randomized multi-entity snapshots.
    #[test]
    fn sim_random_invariants() {
        let cfg = SchedulerConfig::default();
        let mut state: u64 = 0xDEAD_BEEF;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for case in 0..80u32 {
            let cap = 1 + next() % (30 * MIB);
            let mut playback = Vec::new();
            let mut tasks = Vec::new();
            for i in 0..(next() % 3) as usize {
                let mut p = session(&format!("s{i}"), &format!("t{i}"), (next() % 50) as f64);
                if next() % 4 == 0 {
                    p.state = PlaybackSchedulingState::Paused;
                }
                playback.push(p);
                tasks.push(task(&format!("t{i}")));
            }
            for i in 0..(next() % 5) as usize {
                tasks.push(task(&format!("b{i}")));
            }
            let snapshot = snap(cap, playback.clone(), tasks.clone());
            let decisions = policy::compute(&snapshot, &Default::default(), &cfg);
            let sum: u64 = decisions.iter().map(|d| d.allocated_bps).sum();
            assert!(sum <= cap, "case {case}: sum {sum} > cap {cap}");
            let mut ids: Vec<&str> = decisions.iter().map(|d| d.task_id.as_str()).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(
                ids.len(),
                decisions.len(),
                "case {case}: duplicate decision"
            );
        }
    }
}

// ---------------------------------------------------------------------
// §12: the torrent session rate is set through the PUBLIC API.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn torrent_session_rate_smoke() {
    let torrents = torrent_engine("rate").await;
    // Smoke: the public setter accepts a cap and unlimited without
    // error (no swarm needed — the limiter object just stores it).
    torrents.set_download_rate_bps(Some(512 * 1024));
    torrents.set_download_rate_bps(None);
}
