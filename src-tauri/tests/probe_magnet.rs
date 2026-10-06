//! 1.1.x DIAGNOSTIC probe (#[ignore]): resolves one real magnet through
//! the production engine configuration (DHT on, public listener) and
//! prints what the DHT actually achieves — nodes, metadata, progress —
//! so "all magnets fail in Tide" can be attributed to the network or
//! to the client. Run:
//!   RUST_LOG=librqbit=debug,info cargo test --locked --test probe_magnet -- --ignored --nocapture
#![cfg(all())]

use ifuyo_tide_lib::core::torrent::{TorrentEngine, TorrentEngineConfig};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MAGNET: &str = "magnet:?xt=urn:btih:74B238EC745F9EF3E586D30B12E42680B48276A9";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "diagnostic: real-network DHT probe"]
async fn probe_real_magnet_dht() {
    let config = TorrentEngineConfig {
        metadata_dir: std::env::temp_dir().join("tide-probe-meta"),
        dht: true,
        listen_loopback_only: false,
        initial_peers: None,
    };
    let engine = TorrentEngine::new(&config).await.expect("engine");
    let work = std::env::temp_dir().join("tide-probe-dl");
    std::fs::create_dir_all(&work).unwrap();

    let started = Instant::now();
    let added = engine
        .add_magnet(MAGNET.to_string(), &work, None, None, false)
        .await
        .expect("add magnet");
    println!("PROBE: add returned in {:?}", started.elapsed());
    println!("PROBE: info_hash = {}", added.info_hash);

    let deadline = Duration::from_secs(150);
    let mut resolved_at: Option<Duration> = None;
    while started.elapsed() < deadline {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let resolved = engine.metadata_resolved(&added.handle);
        if resolved && resolved_at.is_none() {
            resolved_at = Some(started.elapsed());
            println!("PROBE: METADATA RESOLVED after {:?}", started.elapsed());
            let meta = engine.metadata_of(&added.handle).expect("metadata");
            println!(
                "PROBE: name={:?} files={} total={}",
                meta.name,
                meta.files.len(),
                meta.total_size
            );
        }
        let stats = engine.stats(&added.handle);
        println!(
            "PROBE t={:>4}s resolved={:?} total={:?} progress={} finished={}",
            started.elapsed().as_secs(),
            resolved_at.is_some(),
            stats.total_bytes,
            stats.progress_bytes,
            stats.finished,
        );
        if stats.finished {
            break;
        }
    }
    if resolved_at.is_none() {
        println!(
            "PROBE: metadata NEVER resolved in {:?} — DHT could not find the swarm \
             (network/UDP limitation or a cold swarm).",
            deadline
        );
    }
    engine.shutdown().await;
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::remove_dir_all(&config.metadata_dir);
}
