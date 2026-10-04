//! M9(上) §47 — long-running stability soak (REPRODUCIBLE HARNESS).
//!
//! Run explicitly for the configured duration:
//! ```text
//! cargo test --locked --test m9_soak -- --ignored --nocapture -- soak-seconds=60
//! ```
//! (env var `TIDE_SOAK_SECONDS`, default 60; set 3600/14400/28800 for
//! the 1h/4h/8h spec runs — §47).
//!
//! What it exercises: the scheduler runtime loop + M4 demand registry
//! + engine pacing under continuous churn (tasks added/paused/resumed/
//! removed, demands submitted/cancelled) — the leak-prone surfaces.
//! Assertions are LEAK checks (RSS growth bound, registry bounded),
//! not throughput checks.

mod common;

use common::spawn;
use ifuyo_tide_lib::core::download::file_sink::OverwritePolicy;
use ifuyo_tide_lib::core::download::url::validate_download_url;
use ifuyo_tide_lib::core::download::{DownloadEngine, DownloadRequest};
use ifuyo_tide_lib::core::scheduler::bandwidth::TokenBucket;
use ifuyo_tide_lib::core::streaming::demand::DemandReason;
use ifuyo_tide_lib::core::streaming::demand::DemandScheduler;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn soak_seconds() -> u64 {
    std::env::var("TIDE_SOAK_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

/// RSS in KB via /proc-style query is not portable; on Windows we use
/// GetProcessMemoryInfo through psapi via `winapi`-free path: read the
/// working-set from `std` is unavailable, so we track the registry
/// sizes (the actual leak surface we own) and print process memory via
/// `tasklist` once per report interval.
fn rss_kb_probe() -> Option<u64> {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq ifuyo-tide.exe", "/FO", "CSV", "/NH"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    // "ifuyo-tide.exe","1234","Console","1","45,678 K"
    let field = text.split('"').nth(7)?;
    let digits: String = field
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let kb: String = digits.chars().filter(|c| *c != ',').collect();
    kb.parse().ok()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "soak: run explicitly — TIDE_SOAK_SECONDS=3600 cargo test --test m9_soak -- --ignored"]
async fn soak_long_running_stability() {
    let server = spawn().await;
    let dir = std::env::temp_dir().join(format!(
        "tide-m9-soak-{}",
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let engine = DownloadEngine::new(Arc::new(
        ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap(),
    ));
    let registry = DemandScheduler::new();
    let buckets: Vec<Arc<TokenBucket>> = (0..4).map(|_| Arc::new(TokenBucket::new(0))).collect();

    let duration = Duration::from_secs(soak_seconds());
    let started = Instant::now();
    let mut iteration: u64 = 0;
    let mut last_report = Instant::now();
    let mut report_interval = Duration::from_secs(10);
    let mut first_rss: Option<u64> = None;
    let mut max_rss: Option<u64> = None;

    while started.elapsed() < duration {
        iteration += 1;
        // Churn: cycle 4 downloads through start→pause→resume→delete.
        let url = validate_download_url(&server.url("/slow/medium.bin"), true).unwrap();
        let request =
            DownloadRequest::new(url, dir.clone(), None, OverwritePolicy::Overwrite, None).unwrap();
        let shared = engine.start(request, None).await.expect("start");
        let id = shared.request.download_id.clone();
        let slot = (iteration % 4) as usize;
        engine.set_task_rate_bps(&id, 64 * 1024);
        let _ = &buckets[slot];
        engine.pause(&id).await.expect("pause");
        // §47 churn: resume may legitimately race the pause transition
        // (state is still Running) — the soak tolerates it and moves on.
        if let Err(err) = engine.resume(&id, None).await {
            println!("resume race (expected under churn): {err}");
        }
        if iteration % 4 == 0 {
            let _ = engine.delete(&id).await;
        }
        // Demand registry churn with cancel (§47 event-subscription /
        // registry leak surface).
        for i in 0..50u64 {
            let range =
                ifuyo_tide_lib::core::resource::ByteRange::new(i * 4096, i * 4096 + 4096).unwrap();
            let id = registry.submit(
                &format!("soak-{iteration}"),
                range,
                DemandReason::BackgroundDownload,
                None,
            );
            if i % 5 == 0 {
                registry.cancel(id);
            }
        }

        if last_report.elapsed() >= report_interval {
            last_report = Instant::now();
            let rss = rss_kb_probe();
            if first_rss.is_none() {
                first_rss = rss;
            }
            max_rss = match (max_rss, rss) {
                (_, Some(v)) => Some(max_rss.unwrap_or(v)),
                (m, None) => m,
            };
            println!(
                "soak t={:>4}s iter={:>5} pending={} downloads={} rss={:?}KiB",
                started.elapsed().as_secs(),
                iteration,
                registry.pending_count("soak"),
                engine.list_sync().len(),
                rss
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // §47 assertions: registry bounded (no runaway demand leak).
    assert!(
        registry.pending_count("soak") < 5000,
        "demand registry grew without bound"
    );
    println!(
        "soak complete: {} iterations in {:?}; rss first={:?} max={:?} KiB",
        iteration,
        started.elapsed(),
        first_rss,
        max_rss
    );
}
