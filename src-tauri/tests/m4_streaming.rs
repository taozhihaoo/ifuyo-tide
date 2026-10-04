//! M4(上) streaming integration tests: the ONE gateway serves File,
//! HTTP and Torrent resources through the same session/URL model
//! (§72/§74), the security matrix holds (§56/§57), range semantics are
//! byte-preserving (§92), a stopped session stops serving (§53), and
//! torrent reads WAIT for pieces with a bounded timeout (§71/§97).

mod common;

use common::pattern_byte;
use ifuyo_tide_lib::core::streaming::demand::{DemandReason, DemandScheduler};
use ifuyo_tide_lib::core::streaming::playback::PlaybackManager;
use ifuyo_tide_lib::core::streaming::resource::StreamingResource;
use ifuyo_tide_lib::core::torrent::{TorrentEngine, TorrentEngineConfig};
use librqbit::{CreateTorrentOptions, ListenerMode, ListenerOptions};
use librqbit::{Session, SessionOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const LEN: u64 = 1024 * 1024;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m4-{}-{}",
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

// ---------------------------------------------------------------------
// torrent harness (same shape as the M3 suite: real loopback seeder)
// ---------------------------------------------------------------------

#[allow(dead_code)]
struct Seeder {
    _session: Arc<Session>,
    torrent_bytes: Vec<u8>,
    info_hash: String,
    listen_addr: SocketAddr,
    file_name: String,
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

async fn spawn_seeder(label: &str, file_name: &str, len: u64) -> Seeder {
    let seed_dir = temp_dir(&format!("seed-{label}"));
    write_pattern_at(&seed_dir.join(file_name), len);
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
    let mut listen_addr = None;
    for _ in 0..50 {
        if let Some(addr) = session.listen_addr() {
            listen_addr = Some(addr);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Seeder {
        info_hash: created.info_hash().as_string(),
        torrent_bytes: created.as_bytes().expect("torrent bytes").to_vec(),
        listen_addr: listen_addr.expect("seeder listen addr"),
        file_name: file_name.to_string(),
        _session: session,
    }
}

async fn test_engine(label: &str, initial_peers: Option<Vec<SocketAddr>>) -> TorrentEngine {
    let config = TorrentEngineConfig {
        metadata_dir: temp_dir(&format!("meta-{label}")),
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
        if started.elapsed() > timeout {
            panic!("torrent did not complete in {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

// ---------------------------------------------------------------------
// gateway harness
// ---------------------------------------------------------------------

async fn manager() -> Arc<PlaybackManager> {
    Arc::new(
        PlaybackManager::new(Arc::new(DemandScheduler::new()))
            .await
            .expect("playback manager"),
    )
}

fn file_resource(len: u64) -> (PathBuf, StreamingResource) {
    let dir = temp_dir("file");
    let path = dir.join("content.bin");
    write_pattern_at(&path, len);
    (
        dir,
        StreamingResource::File {
            path: path.clone(),
            len,
        },
    )
}

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// Hand-rolled one-shot client (Connection: close), full control over
/// Host/Range so the security matrix is exercised at the wire level.
/// `idle_secs` bounds each read — long enough for a read that is
/// legitimately WAITING on torrent pieces.
async fn http_request(
    port: u16,
    target: &str,
    host: Option<&str>,
    method: &str,
    range: Option<&str>,
) -> RawResponse {
    http_request_bounded(port, target, host, method, range, 10).await
}

async fn http_request_bounded(
    port: u16,
    target: &str,
    host: Option<&str>,
    method: &str,
    range: Option<&str>,
    idle_secs: u64,
) -> RawResponse {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    let host_line = match host {
        Some(h) => format!("Host: {h}\r\n"),
        None => String::new(),
    };
    let range_line = match range {
        Some(r) => format!("Range: {r}\r\n"),
        None => String::new(),
    };
    let request = format!(
        "{method} {target} HTTP/1.1\r\n{host_line}{range_line}Connection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    // Bounded read: a hung gateway must fail the test, not the suite.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let mut chunk = [0u8; 16 * 1024];
        let n = tokio::time::timeout(Duration::from_secs(idle_secs), stream.read(&mut chunk))
            .await
            .expect("read idle timeout")
            .expect("read");
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..n]);
        if Instant::now() > deadline {
            panic!("gateway response exceeded 120s");
        }
    }
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> RawResponse {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header terminator");
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or_default();
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    RawResponse {
        status,
        headers,
        body: raw[header_end + 4..].to_vec(),
    }
}

impl RawResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn assert_body_matches_pattern(&self, start: u64) {
        for (i, b) in self.body.iter().enumerate() {
            assert_eq!(*b, pattern_byte(start + i as u64), "byte {} mismatch", start + i as u64);
        }
    }
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

/// §72/§74: File, HTTP and Torrent resources play through the SAME
/// gateway, the SAME session model and byte-identical range semantics.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unified_gateway_serves_file_http_and_torrent() {
    // One origin server for the HTTP resource.
    let origin = common::spawn().await;
    // One file resource.
    let (_file_dir, file_res) = file_resource(LEN);
    // One torrent resource (downloaded to completion first).
    let seeder = spawn_seeder("unified", "movie.bin", LEN).await;
    let engine = test_engine("unified", Some(vec![seeder.listen_addr])).await;
    let work = temp_dir("unified-dl");
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add torrent");
    wait_completed(&engine, &added.info_hash, Duration::from_secs(60)).await;
    let torrent_res = StreamingResource::Torrent {
        engine: engine.clone(),
        info_hash: added.info_hash.clone(),
        file_index: 0,
        len: LEN,
    };

    // ONE manager = ONE gateway for all three sessions.
    let playback = manager().await;
    let port = playback.gateway_port();
    let http_res = StreamingResource::Http {
        url: origin.url("/medium.bin"),
        len: LEN,
        accept_ranges: true,
    };
    let sessions = [
        playback.open(file_res, "application/octet-stream".into(), None).expect("open file"),
        playback.open(http_res, "application/octet-stream".into(), None).expect("open http"),
        playback.open(torrent_res, "application/octet-stream".into(), None).expect("open torrent"),
    ];
    assert_eq!(playback.session_count(), 3);
    assert!(sessions.iter().all(|s| s.url.contains(&format!(":{port}/"))));

    for session in &sessions {
        // Closed range → 206 with exact patterned bytes.
        let response = http_request(
            port,
            &format!("/stream/{}", session.session_id),
            Some("127.0.0.1:9"),
            "GET",
            Some("bytes=4096-12287"),
        )
        .await;
        assert_eq!(response.status, 206, "{} session", session.source_kind);
        assert_eq!(response.body.len(), 8192);
        response.assert_body_matches_pattern(4096);
        assert_eq!(
            response.header("content-range"),
            Some(format!("bytes 4096-12287/{LEN}").as_str())
        );

        // Suffix range → the last 4096 bytes.
        let response = http_request(
            port,
            &format!("/stream/{}", session.session_id),
            Some("127.0.0.1:9"),
            "GET",
            Some("bytes=-4096"),
        )
        .await;
        assert_eq!(response.status, 206, "{} session", session.source_kind);
        assert_eq!(response.body.len(), 4096);
        response.assert_body_matches_pattern(LEN - 4096);
    }

    for session in &sessions {
        assert!(playback.close(&session.session_id));
    }
    assert_eq!(playback.session_count(), 0);
    engine.shutdown().await;
}

/// §56/§57: the security matrix — no host, hostile host, wrong/empty/
/// traversal tokens, and malformed/unsatisfiable ranges must never
/// yield file bytes.
#[tokio::test]
async fn gateway_security_matrix() {
    let playback = manager().await;
    let (_dir, resource) = file_resource(64 * 1024);
    let session = playback
        .open(resource, "application/octet-stream".into(), None)
        .expect("open");
    let port = playback.gateway_port();

    // Missing Host header → rejected.
    let r = http_request(port, &format!("/stream/{}", session.session_id), None, "GET", None).await;
    assert_eq!(r.status, 403);
    // Hostile Host header → rejected.
    let r = http_request(
        port,
        &format!("/stream/{}", session.session_id),
        Some("evil.example.com"),
        "GET",
        None,
    )
    .await;
    assert_eq!(r.status, 403);

    for bad_target in [
        "/stream/not-a-real-token".to_string(),
        "/stream/".to_string(),
        format!("/stream/{}/../etc", session.session_id),
        "/stream/..%2e%2fwin.ini".to_string(),
        "/stream/with/slash".to_string(),
        "/".to_string(),
    ] {
        let r = http_request(port, &bad_target, Some("127.0.0.1:9"), "GET", None).await;
        assert_eq!(r.status, 404, "target {bad_target} must not serve");
        assert!(r.body.is_empty());
    }

    // Malformed range → 400.
    let r = http_request(
        port,
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=zz-yy"),
    )
    .await;
    assert_eq!(r.status, 400);

    // Unsatisfiable range → 416 with bytes */len.
    let r = http_request(
        port,
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=999999999-"),
    )
    .await;
    assert_eq!(r.status, 416);
    assert_eq!(r.header("content-range"), Some("bytes */65536"));

    // Multi-range is not served as multipart — full 200 body instead.
    let r = http_request(
        port,
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-1,5-6"),
    )
    .await;
    assert_eq!(r.status, 200);
    assert_eq!(r.body.len() as u64, 64 * 1024);
}

/// §61/§92: full GET → 200 byte-exact; HEAD carries the range length
/// with NO body; every served body is byte-preserving.
#[tokio::test]
async fn range_head_and_full_semantics() {
    let playback = manager().await;
    let (_dir, resource) = file_resource(LEN);
    let session = playback.open(resource, "application/octet-stream".into(), None).expect("open");
    let target = format!("/stream/{}", session.session_id);

    // Full GET.
    let r = http_request(playback.gateway_port(), &target, Some("127.0.0.1:9"), "GET", None).await;
    assert_eq!(r.status, 200);
    assert_eq!(r.body.len() as u64, LEN);
    r.assert_body_matches_pattern(0);

    // HEAD with a range: headers only, range length.
    let r = http_request(
        playback.gateway_port(),
        &target,
        Some("127.0.0.1:9"),
        "HEAD",
        Some("bytes=1000-2999"),
    )
    .await;
    assert_eq!(r.status, 206);
    assert_eq!(r.header("content-length"), Some("2000"));
    assert!(r.body.is_empty(), "HEAD must not carry a body");

    // Open-ended range to EOF.
    let r = http_request(
        playback.gateway_port(),
        &target,
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=1048500-"),
    )
    .await;
    assert_eq!(r.status, 206);
    assert_eq!(r.body.len(), (LEN - 1048500) as usize);
    r.assert_body_matches_pattern(1048500);
}

/// §53/§80: stopping a session revokes gateway access AND cancels its
/// queued demands.
#[tokio::test]
async fn stop_revokes_gateway_and_cancels_demands() {
    let scheduler = Arc::new(DemandScheduler::new());
    let playback = Arc::new(
        PlaybackManager::new(scheduler.clone()).await.expect("manager"),
    );
    let (_dir, resource) = file_resource(64 * 1024);
    let session = playback.open(resource, "application/octet-stream".into(), None).expect("open");
    let port = playback.gateway_port();
    let target = format!("/stream/{}", session.session_id);

    // Queued (never served) demands exist for the session.
    scheduler.submit(&session.session_id, ifuyo_tide_lib::core::resource::ByteRange::new(0, 1024).unwrap(), DemandReason::PlaybackBuffer, None);
    assert_eq!(scheduler.pending_count(session.session_id.as_str()), 1);

    let r = http_request(port, &target, Some("127.0.0.1:9"), "GET", Some("bytes=0-99")).await;
    assert_eq!(r.status, 206);

    assert!(playback.close(&session.session_id));
    let r = http_request(port, &target, Some("127.0.0.1:9"), "GET", Some("bytes=0-99")).await;
    assert_eq!(r.status, 404, "stopped session must stop serving");
    assert_eq!(scheduler.pending_count(session.session_id.as_str()), 0);
}

/// §97: a torrent whose pieces can never arrive (no peers, no DHT)
/// must fail the read after a BOUNDED wait — the connection closes
/// early, the suite never hangs.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn torrent_read_without_peers_is_bounded() {
    let seeder = spawn_seeder("nopeers", "movie.bin", LEN).await;
    // Engine WITHOUT initial peers: metadata exists (we pass the
    // .torrent bytes) but no piece can ever arrive.
    let engine = test_engine("nopeers", None).await;
    let work = temp_dir("nopeers-dl");
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add torrent");

    let playback = manager().await;
    playback.set_read_timeout_ms(1_500);
    let resource = StreamingResource::Torrent {
        engine: engine.clone(),
        info_hash: added.info_hash.clone(),
        file_index: 0,
        len: LEN,
    };
    let session = playback.open(resource, "application/octet-stream".into(), None).expect("open");

    let started = Instant::now();
    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-65535"),
    )
    .await;
    let elapsed = started.elapsed();
    assert_eq!(r.status, 206);
    assert!(
        (r.body.len() as u64) < 65536,
        "expected a truncated body, got {} bytes",
        r.body.len()
    );
    assert!(elapsed < Duration::from_secs(25), "bounded wait exceeded: {elapsed:?}");
    engine.shutdown().await;
}

/// §71: the full demand chain must actually run — a read for missing
/// pieces WAITS, the swarm delivers, availability updates, and the
/// gateway serves the correct 206 bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn torrent_read_waits_for_pieces_then_serves() {
    let seeder = spawn_seeder("wait", "movie.bin", LEN).await;
    let engine = test_engine("wait", Some(vec![seeder.listen_addr])).await;
    let work = temp_dir("wait-dl");
    let added = engine
        .add_torrent_bytes(seeder.torrent_bytes.clone(), &work, None, None, false)
        .await
        .expect("add torrent");

    // Ask for the LAST 64KiB immediately: the swarm has barely started,
    // so this read blocks until the pieces arrive (§71 wake-up path).
    let playback = manager().await;
    let resource = StreamingResource::Torrent {
        engine: engine.clone(),
        info_hash: added.info_hash.clone(),
        file_index: 0,
        len: LEN,
    };
    let session = playback.open(resource, "application/octet-stream".into(), None).expect("open");
    let range = format!("bytes={}-{}", LEN - 65536, LEN - 1);

    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some(&range),
    )
    .await;
    assert_eq!(r.status, 206);
    assert_eq!(r.body.len(), 65536, "read must WAIT for pieces, not truncate");
    r.assert_body_matches_pattern(LEN - 65536);
    engine.shutdown().await;
}

/// §80/§82-C: seek cancels the stale queued demands; new demands for
/// the new position are the ones served.
#[tokio::test]
async fn seek_cancels_stale_demands() {
    let scheduler = Arc::new(DemandScheduler::new());
    let playback = Arc::new(
        PlaybackManager::new(scheduler.clone()).await.expect("manager"),
    );
    let (_dir, resource) = file_resource(256 * 1024);
    let session = playback.open(resource, "application/octet-stream".into(), None).expect("open");
    let id = session.session_id.as_str();

    let make_range = |start: u64| {
        ifuyo_tide_lib::core::resource::ByteRange::new(start, start + 4096).unwrap()
    };
    // Old position demands queued…
    scheduler.submit(id, make_range(0), DemandReason::PlaybackBuffer, None);
    scheduler.submit(id, make_range(4096), DemandReason::PlaybackBuffer, None);
    assert_eq!(scheduler.pending_count(id), 2);

    // …user seeks: stale demands are dropped (§80)…
    playback.notify_seek(id);
    assert_eq!(scheduler.pending_count(id), 0);

    // …and the new position gets priority when re-queued.
    let seek = scheduler.submit(id, make_range(131072), DemandReason::PlaybackSeek, None);
    let buffer = scheduler.submit(id, make_range(0), DemandReason::PlaybackBuffer, None);
    let next = scheduler.next_for(id).expect("next demand");
    assert_eq!(next.id, seek, "seek demand beats buffer demand");
    scheduler.complete(seek);
    let next = scheduler.next_for(id).expect("next demand");
    assert_eq!(next.id, buffer);
    playback.close(id);
}
