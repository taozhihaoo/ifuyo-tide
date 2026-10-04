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
