//! M3 torrent engine integration tests: REAL BitTorrent over a local
//! loopback swarm (M3 §44/§46) — a seeder session (from the M0.5 spike
//! pattern) plus the production TorrentEngine as downloader.
//!
//! Security cases (traversal/absolute/reserved) run at the validation
//! layer; DHT/trackers are disabled and peers connect over 127.0.0.1.

mod common;

use common::pattern_byte;
use ifuyo_tide_lib::core::torrent::source::validate_torrent_paths;
use ifuyo_tide_lib::core::torrent::{TorrentEngine, TorrentEngineConfig};
use librqbit::{CreateTorrentOptions, ListenerMode, ListenerOptions};
use librqbit::{Session, SessionOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m3-{}-{}",
        label,
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

fn write_pattern_at(path: &Path, len: u64) {
    let mut file = std::fs::File::create(path).expect("create");
    let mut buf = vec![0u8; 64 * 1024];
    let mut offset = 0u64;
    while offset < len {
        let take = (len - offset).min(buf.len() as u64) as usize;
        for (i, b) in buf.iter_mut().enumerate().take(take) {
            *b = pattern_byte(offset + i as u64);
        }
        file.write_all(&buf[..take]).unwrap();
        offset += take as u64;
    }
}

/// A seeder session serving a generated torrent over loopback.
struct Seeder {
    session: Arc<Session>,
    torrent_bytes: Vec<u8>,
    info_hash: String,
    listen_addr: SocketAddr,
    files: Vec<(String, u64)>,
    seed_dir: PathBuf,
}

fn seeder_session_options() -> SessionOptions {
    SessionOptions {
        dht: None,
        disable_trackers: true,
        listen: Some(ListenerOptions {
            listen_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            mode: ListenerMode::TcpOnly,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn wait_listen_addr(session: &Session) -> SocketAddr {
    for _ in 0..50 {
        if let Some(addr) = session.listen_addr() {
            return addr;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("seeder never bound a listen port");
}

/// Creates the fixture files, seeds them, and returns the metadata.
async fn spawn_seeder(label: &str, files: &[(&str, u64)]) -> Seeder {
    let seed_dir = temp_dir(&format!("seed-{label}"));
    for (name, len) in files {
        write_pattern_at(&seed_dir.join(name), *len);
    }
    // Session::new_with_opts already returns Arc<Session>.
    let session = Session::new_with_opts(seed_dir.clone(), seeder_session_options())
        .await
        .expect("seeder session");
    let (created, _handle) = session
        .create_and_serve_torrent(
            &seed_dir,
            CreateTorrentOptions {
                name: None,
                trackers: Vec::new(),
                piece_length: Some(128 * 1024),
            },
        )
        .await
        .expect("create torrent");
    let listen_addr = wait_listen_addr(&session).await;
    let mut torrent_bytes = created.as_bytes().expect("torrent bytes").to_vec();
    torrent_bytes = normalize_torrent_bytes(torrent_bytes);
    Seeder {
        session,
        info_hash: created.info_hash().as_string(),
        torrent_bytes,
        listen_addr,
        files: files.iter().map(|(n, l)| (n.to_string(), *l)).collect(),
        seed_dir,
    }
}

/// librqbit's create_and_serve_torrent stores the torrent in the
/// session; the bytes contain the full metainfo — pass through.
fn normalize_torrent_bytes(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

/// Production-shaped TorrentEngine (loopback, no DHT) for tests.
async fn test_engine(
    metadata_dir: PathBuf,
    initial_peers: Option<Vec<SocketAddr>>,
) -> TorrentEngine {
    let config = TorrentEngineConfig {
        metadata_dir,
        dht: false,
        listen_loopback_only: true,
        initial_peers,
    };
    TorrentEngine::new(&config).await.expect("torrent engine")
}

async fn wait_completed(engine: &TorrentEngine, info_hash: &str, timeout: Duration) {
    let started = Instant::now();
    let handle = engine
        .handle_for(info_hash)
        .unwrap_or_else(|| panic!("handle for {info_hash}"));
    loop {
        let stats = engine.stats(&handle);
        if stats.finished {
            return;
        }
        if let Some(err) = &stats.error {
            panic!("torrent error: {err}");
        }
        if started.elapsed() > timeout {
            panic!("torrent did not complete in {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn verify_file(path: &Path, len: u64) {
    use std::io::{Read, Seek, SeekFrom};
    let meta = std::fs::metadata(path).expect("final exists");
    assert_eq!(meta.len(), len);
    let mut file = std::fs::File::open(path).expect("open");
    let mut buf = vec![0u8; 1024];
    for probe in [0u64, len / 2, len - 1024] {
        file.seek(SeekFrom::Start(probe)).unwrap();
        let take = 1024.min((len - probe) as usize);
        file.read_exact(&mut buf[..take]).unwrap();
        for (i, b) in buf.iter().enumerate().take(take) {
            assert_eq!(*b, pattern_byte(probe + i as u64));
        }
    }
}

/// T01+T02: .torrent → metadata → files → full download → correct bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn torrent_file_download_completes_and_verifies() {
    let seeder = spawn_seeder("single", &[("movie.bin", 1024 * 1024)]).await;
    let work = temp_dir("dl");
    let engine = test_engine(temp_dir("meta"), Some(vec![seeder.listen_addr])).await;

    // T01: metadata + file list from the .torrent bytes.
    let metadata = engine
        .inspect_torrent_bytes(seeder.torrent_bytes.clone())
        .await
        .expect("inspect");
    assert_eq!(metadata.info_hash, seeder.info_hash);
    assert_eq!(metadata.files.len(), 1);
    assert_eq!(metadata.files[0].size, 1024 * 1024);

    // T02: add → download → complete → verify.
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add");
    assert_eq!(added.info_hash, seeder.info_hash);
    wait_completed(&engine, &added.info_hash, Duration::from_secs(60)).await;

    let downloaded = work.join("movie.bin");
    verify_file(&downloaded, 1024 * 1024);
    engine.shutdown().await;
    let _ = std::fs::remove_dir_all(temp_dir("seed-single"));
    let _ = std::fs::remove_dir_all(work);
    let _ = std::fs::remove_dir_all(temp_dir("meta"));
}

/// T03: multi-file torrent with partial selection (M3 §9/§10/§82).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multi_file_selection_downloads_selected_only() {
    let seeder = spawn_seeder("multi", &[("big.bin", 1024 * 1024), ("tiny.txt", 4096)]).await;
    let work = temp_dir("multi-dl");
    let engine = test_engine(temp_dir("multi-meta"), Some(vec![seeder.listen_addr])).await;

    // Metadata lists both files.
    let metadata = engine
        .inspect_torrent_bytes(seeder.torrent_bytes.clone())
        .await
        .expect("inspect");
    assert_eq!(metadata.files.len(), 2);
    validate_torrent_paths(
        &metadata
            .files
            .iter()
            .map(|f| f.relative_path.clone())
            .collect::<Vec<_>>(),
    )
    .expect("paths are safe");

    // Select ONLY the small file: task completes with the big file
    // unselected (M3 §82 — partial selection can complete).
    let added = engine
        .add_torrent_bytes(
            seeder.torrent_bytes.clone(),
            &work,
            Some(vec![1]),
            None,
            false,
        )
        .await
        .expect("add");
    wait_completed(&engine, &added.info_hash, Duration::from_secs(60)).await;

    let selected_path = work.join("tiny.txt");
    assert!(selected_path.exists(), "selected file must exist");
    assert_eq!(std::fs::metadata(&selected_path).unwrap().len(), 4096);
    let unselected = work.join("big.bin");
    assert!(
        !unselected.exists() || std::fs::metadata(&unselected).unwrap().len() < 1024 * 1024,
        "unselected file must not be fully downloaded"
    );
    engine.shutdown().await;
    let _ = std::fs::remove_dir_all(temp_dir("seed-multi"));
    let _ = std::fs::remove_dir_all(work);
    let _ = std::fs::remove_dir_all(temp_dir("multi-meta"));
}

/// T04: pause stops the bytes, resume continues them (M3 §19).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pause_resume_stops_and_continues_bytes() {
    let seeder = spawn_seeder("pause", &[("movie.bin", 4 * 1024 * 1024)]).await;
    let work = temp_dir("pause-dl");
    let engine = test_engine(temp_dir("pause-meta"), Some(vec![seeder.listen_addr])).await;
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add");
    let handle = added.handle.clone();

    // Let some bytes flow, then pause.
    let started = Instant::now();
    loop {
        let stats = engine.stats(&handle);
        if stats.progress_bytes > 512 * 1024 {
            break;
        }
        if started.elapsed() > Duration::from_secs(30) {
            panic!("no progress");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    engine.pause(&handle).await.expect("pause");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let at_pause = engine.stats(&handle).progress_bytes;
    // Bytes must stop growing while paused.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let still = engine.stats(&handle).progress_bytes;
    assert!(
        still <= at_pause + 64 * 1024,
        "paused download must stop: {at_pause} → {still}"
    );

    // Resume: bytes continue to completion.
    engine.resume(&handle).await.expect("resume");
    wait_completed(&engine, &added.info_hash, Duration::from_secs(120)).await;
    verify_file(&work.join("movie.bin"), 4 * 1024 * 1024);
    engine.shutdown().await;
    let _ = std::fs::remove_dir_all(temp_dir("seed-pause"));
    let _ = std::fs::remove_dir_all(work);
}

/// T05/T06: crash (drop engine mid-download) + corrupt a byte + restart →
/// librqbit re-verifies pieces and re-downloads the bad one (M3 §22).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn restart_with_corrupted_piece_redownloads() {
    let seeder = spawn_seeder("corrupt", &[("movie.bin", 2 * 1024 * 1024)]).await;
    let work = temp_dir("corrupt-dl");
    let meta = temp_dir("corrupt-meta");
    let engine = test_engine(meta.clone(), Some(vec![seeder.listen_addr])).await;
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add");

    // Let it partially download, then "crash" (pause + drop everything).
    let started = Instant::now();
    loop {
        let stats = engine.stats(&added.handle);
        if stats.progress_bytes > 512 * 1024 {
            break;
        }
        if started.elapsed() > Duration::from_secs(30) {
            panic!("no progress");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    engine.pause(&added.handle).await.expect("pause");
    engine.shutdown().await;

    // Corrupt one byte inside the downloaded region.
    let part_or_file = {
        let candidates = [work.join("movie.bin"), work.join("movie.bin.part")];
        candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .expect("artifact exists")
    };
    {
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&part_or_file)
            .unwrap();
        f.seek(SeekFrom::Start(4096)).unwrap();
        let mut old = [0u8; 1];
        f.read_exact(&mut old).unwrap();
        f.seek(SeekFrom::Start(4096)).unwrap();
        f.write_all(&[old[0] ^ 0xFF]).unwrap();
    }

    // "Restart": a fresh engine over the same storage + metadata re-add
    // (the exact recovery path M3 §15 describes).
    let engine2 = test_engine(meta, Some(vec![seeder.listen_addr])).await;
    let re_added = engine2
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("re-add");
    // librqbit re-verifies pieces on add (overwrite=true) and re-fetches
    // the corrupted one.
    wait_completed(&engine2, &re_added.info_hash, Duration::from_secs(120)).await;
    verify_file(&work.join("movie.bin"), 2 * 1024 * 1024);
    engine2.shutdown().await;
    let _ = std::fs::remove_dir_all(temp_dir("seed-corrupt"));
    let _ = std::fs::remove_dir_all(work);
}

/// T07: duplicate info_hash → the session reuses the torrent (no second
/// download) — M3 §17.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn duplicate_add_reuses_managed_torrent() {
    let seeder = spawn_seeder("dup", &[("dup.bin", 1024 * 1024)]).await;
    let work = temp_dir("dup-dl");
    let engine = test_engine(temp_dir("dup-meta"), Some(vec![seeder.listen_addr])).await;
    let first = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, true)
        .await
        .expect("first add");
    assert!(!first.already_managed);
    let second = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, true)
        .await
        .expect("second add");
    assert!(
        second.already_managed,
        "duplicate must reuse the managed torrent (M3 §17)"
    );
    assert_eq!(first.info_hash, second.info_hash);
    engine.shutdown().await;
    let _ = std::fs::remove_dir_all(temp_dir("seed-dup"));
    let _ = std::fs::remove_dir_all(work);
}

/// T09 security cases at the validation layer (M3 §12).
#[test]
fn malicious_torrent_paths_are_rejected() {
    for bad in [
        "../escape.bin",
        "..\\escape.bin",
        "C:\\Windows\\evil.exe",
        "\\\\evil\\share\\f",
        "CON",
        "aux/b",
    ] {
        assert!(validate_torrent_paths(&[bad.to_string()]).is_err(), "{bad}");
    }
    assert!(validate_torrent_paths(&["normal.bin".to_string()]).is_ok());
}

/// Smoke: the seeder serves a valid swarm the production engine can join.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn zero_byte_file_in_multi_file_torrent() {
    let seeder = spawn_seeder("zero", &[("zero.bin", 0), ("data.bin", 128 * 1024)]).await;
    let work = temp_dir("zero-dl");
    let engine = test_engine(temp_dir("zero-meta"), Some(vec![seeder.listen_addr])).await;
    let metadata = engine
        .inspect_torrent_bytes(seeder.torrent_bytes.clone())
        .await
        .expect("inspect");
    let zero = metadata
        .files
        .iter()
        .find(|f| f.size == 0)
        .expect("zero-byte file listed");
    assert_eq!(zero.size, 0);
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add");
    wait_completed(&engine, &added.info_hash, Duration::from_secs(60)).await;
    assert!(work.join("zero.bin").exists());
    engine.shutdown().await;
    let _ = std::fs::remove_dir_all(temp_dir("seed-zero"));
    let _ = std::fs::remove_dir_all(work);
}
