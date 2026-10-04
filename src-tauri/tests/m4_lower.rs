//! M4(下) integration tests: the critical acceptance chain (§126–§134),
//! full security matrix (§100/§101/§125), cancellation/deadlock (§116/
//! §137), Windows concurrent read/write (§118), fuzz/abuse (§136) and
//! the recorded performance baseline (§119/§169/§196).

mod common;

use common::{pattern_byte, TestServer};
use ifuyo_tide_lib::core::download::file_sink::OverwritePolicy;
use ifuyo_tide_lib::core::download::url::validate_download_url;
use ifuyo_tide_lib::core::download::{DownloadEngine, DownloadRequest};
use ifuyo_tide_lib::core::recovery::recover_all;
use ifuyo_tide_lib::core::resource::ByteRange;
use ifuyo_tide_lib::core::streaming::demand::{DemandReason, DemandScheduler};
use ifuyo_tide_lib::core::streaming::playback::PlaybackManager;
use ifuyo_tide_lib::core::streaming::resource::{OriginSource, StreamingResource};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MIB: u64 = 1024 * 1024;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m4l-{}-{}",
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

fn physical_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

// ---------------------------------------------------------------------
// HTTP download harness (production engine over the M1 fixture server)
// ---------------------------------------------------------------------

fn make_request(server: &TestServer, path: &str, dir: &Path) -> DownloadRequest {
    let url = validate_download_url(&server.url(path), true).expect("valid test url");
    DownloadRequest::new(
        url,
        dir.to_path_buf(),
        None,
        OverwritePolicy::Overwrite,
        None,
    )
    .expect("request")
}

fn engine() -> DownloadEngine {
    DownloadEngine::new(Arc::new(
        ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap(),
    ))
}

async fn wait_frontier(shared: &ifuyo_tide_lib::core::download::DownloadShared, min: u64) {
    let started = Instant::now();
    loop {
        if shared.status().downloaded >= min {
            return;
        }
        if started.elapsed() > Duration::from_secs(30) {
            panic!("download made no progress in 30s");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ---------------------------------------------------------------------
// gateway harness (same wire-level client as the M4(上) suite)
// ---------------------------------------------------------------------

async fn manager() -> Arc<PlaybackManager> {
    Arc::new(
        PlaybackManager::new(Arc::new(DemandScheduler::new()))
            .await
            .expect("playback manager"),
    )
}

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn http_request(
    port: u16,
    target: &str,
    host: Option<&str>,
    method: &str,
    range: Option<&str>,
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
    let request =
        format!("{method} {target} HTTP/1.1\r\n{host_line}{range_line}Connection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    loop {
        let mut chunk = [0u8; 16 * 1024];
        let n = tokio::time::timeout(Duration::from_secs(90), stream.read(&mut chunk))
            .await
            .expect("read idle timeout")
            .expect("read");
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..n]);
    }
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> RawResponse {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(raw.len());
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
        body: raw[(header_end + 4).min(raw.len())..].to_vec(),
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
            assert_eq!(
                *b,
                pattern_byte(start + i as u64),
                "byte {} mismatch",
                start + i as u64
            );
        }
    }
}

// ---------------------------------------------------------------------
// Critical Acceptance #1 (§126): HTTP progressive playback — the
// player reaches the frontier, the gateway WAITS, the download
// extends, playback continues. No origin: pure local-frontier path.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn critical_1_http_progressive_playback_waits_at_frontier() {
    let server = common::spawn().await;
    let dir = temp_dir("c1");
    let engine = engine();
    let shared = engine
        .start(make_request(&server, "/slow/medium.bin", &dir), None)
        .await
        .expect("start download");
    wait_frontier(&shared, 64 * 1024).await;

    let part_path = shared.status().part_path.expect("part path");
    let part = StreamingResource::Part {
        path: part_path.clone(),
        total_len: Some(MIB),
        len_rx: Some(shared.subscribe_durable_len()),
        origin: None,
    };
    let playback = manager().await;
    let session = playback
        .open(part, "video/mp4".into(), None)
        .expect("open part session");

    // Ask for a range that does NOT exist yet: [384KiB, 448KiB).
    let range = format!("bytes={}-{}", 384 * 1024, 448 * 1024 - 1);
    let started = Instant::now();
    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some(&range),
    )
    .await;
    let elapsed = started.elapsed();

    assert_eq!(r.status, 206);
    assert_eq!(r.body.len(), 64 * 1024, "read must WAIT for the frontier");
    r.assert_body_matches_pattern(384 * 1024);
    assert!(
        elapsed < Duration::from_secs(30),
        "frontier wait exceeded: {elapsed:?}"
    );
}

// ---------------------------------------------------------------------
// Critical Acceptance #2 (§127): seek into the NOT-yet-downloaded
// region — served through the origin read-through (§127/§145).
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn critical_2_http_seek_beyond_frontier_reads_through_origin() {
    let server = common::spawn().await;
    let dir = temp_dir("c2");
    let engine = engine();
    let shared = engine
        .start(make_request(&server, "/slow/medium.bin", &dir), None)
        .await
        .expect("start download");
    wait_frontier(&shared, 64 * 1024).await;
    let before = server.seen_ranges().len();

    let part_path = shared.status().part_path.expect("part path");
    let part = StreamingResource::Part {
        path: part_path,
        total_len: Some(MIB),
        // No live frontier signal: exercises the pure read-through path.
        len_rx: None,
        origin: Some(OriginSource {
            url: server.url("/medium.bin"),
        }),
    };
    let playback = manager().await;
    let session = playback
        .open(part, "video/mp4".into(), None)
        .expect("open part session");

    // Seek far ahead, past the frontier.
    let range = format!("bytes={}-{}", 768 * 1024, 832 * 1024 - 1);
    let started = Instant::now();
    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some(&range),
    )
    .await;
    let elapsed = started.elapsed();

    assert_eq!(r.status, 206);
    assert_eq!(
        r.body.len(),
        64 * 1024,
        "seek beyond frontier must be served"
    );
    r.assert_body_matches_pattern(768 * 1024);
    // The origin answered a real Range request for the missing tail.
    assert!(
        server.seen_ranges().len() > before,
        "origin must have been read through"
    );
    assert!(
        server.seen_ranges().iter().any(|r| r.contains("786432")),
        "origin range must start at the frontier offset"
    );
    println!("read-through seek latency: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(10));
}

// ---------------------------------------------------------------------
// Critical Acceptance #7 (§132): restart — partial download, app
// "restart" (fresh engine over the same repository + startup
// recovery), availability reconstructed, new playback session works.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn critical_7_restart_recovers_resource_and_playback() {
    let server = common::spawn().await;
    let dir = temp_dir("c7");
    let db = temp_dir("c7-db").join("downloads.db");
    let repo = Arc::new(
        ifuyo_tide_lib::persistence::repositories::DownloadRepository::open(&db).expect("repo"),
    );
    let engine_a = DownloadEngine::new(repo.clone());
    let shared = engine_a
        .start(make_request(&server, "/slow/medium.bin", &dir), None)
        .await
        .expect("start download");
    wait_frontier(&shared, 256 * 1024).await;
    engine_a
        .pause(&shared.request.download_id)
        .await
        .expect("pause");
    // Wait for the paused state to be persisted.
    let started = Instant::now();
    loop {
        if shared.status().lifecycle == ifuyo_tide_lib::core::download::DownloadLifecycle::Paused {
            break;
        }
        if started.elapsed() > Duration::from_secs(10) {
            panic!("pause never settled");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let task_id = shared.request.download_id.clone();
    let part_path = shared.status().part_path.expect("part path");
    let paused_len = physical_len(&part_path);
    assert!(paused_len >= 256 * 1024);
    drop(engine_a);

    // "Relaunch": fresh engine + startup recovery over the same store.
    let repo_b = Arc::new(
        ifuyo_tide_lib::persistence::repositories::DownloadRepository::open(&db).expect("repo"),
    );
    let engine_b = DownloadEngine::new(repo_b.clone());
    let outcomes = recover_all(&*repo_b, &engine_b).expect("recovery");
    assert!(!outcomes.is_empty(), "the paused task must be recovered");

    // Availability reconstructed from the PHYSICAL part size (M2
    // invariant) — a new session serves the recovered bytes.
    let record = repo_b.get(&task_id).expect("get").expect("exists");
    let len_rx = engine_b
        .get_sync(&task_id)
        .map(|s| s.subscribe_durable_len());
    let part = StreamingResource::Part {
        path: record.part_path.clone(),
        total_len: record.total_bytes,
        len_rx,
        origin: Some(OriginSource {
            url: server.url("/medium.bin"),
        }),
    };
    assert_eq!(
        part.available_len(),
        paused_len,
        "availability = physical part size"
    );

    let playback = manager().await;
    let session = playback
        .open(part, "video/mp4".into(), None)
        .expect("open session after restart");
    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-262143"),
    )
    .await;
    assert_eq!(r.status, 206);
    assert_eq!(r.body.len(), 256 * 1024);
    r.assert_body_matches_pattern(0);
}

// ---------------------------------------------------------------------
// Critical Acceptance #9 (§134): byte preservation — multiple random
// ranges through Resource.read → Gateway equal the original, byte for
// byte, including odd (non-piece-aligned) offsets and EOF edges.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn critical_9_byte_preservation_across_random_ranges() {
    let len: u64 = 8 * MIB;
    let dir = temp_dir("c9");
    let path = dir.join("original.bin");
    write_pattern_at(&path, len);

    let playback = manager().await;
    let session = playback
        .open(
            StreamingResource::File {
                path: path.clone(),
                len,
            },
            "application/octet-stream".into(),
            None,
        )
        .expect("open");
    let port = playback.gateway_port();
    let target = format!("/stream/{}", session.session_id);

    let ranges: [(u64, u64); 7] = [
        (0, MIB),
        (MIB, 2 * MIB),
        (7 * MIB, 8 * MIB - 1),
        (7 * MIB, 8 * MIB),
        (MIB + 13, MIB + 13 + 4999),     // non-aligned odd offset
        (8 * MIB - MIB, 8 * MIB),        // last MiB
        (4_000_003, 4_000_003 + 65_537), // arbitrary interior window
    ];
    for (start, end) in ranges {
        let r = http_request(
            port,
            &target,
            Some("127.0.0.1:9"),
            "GET",
            Some(&format!("bytes={start}-{}", end - 1)),
        )
        .await;
        assert_eq!(r.status, 206, "range {start}..{end}");
        assert_eq!(r.body.len() as u64, end - start);
        assert_eq!(
            r.header("content-range"),
            Some(format!("bytes {start}-{}/{len}", end - 1).as_str()),
            "content-range consistency (§109)"
        );
        r.assert_body_matches_pattern(start);
    }
    playback.close(&session.session_id);
}

// ---------------------------------------------------------------------
// §102: task deletion invalidates its playback sessions.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn task_delete_invalidates_its_sessions() {
    let dir = temp_dir("owner");
    let path = dir.join("f.bin");
    write_pattern_at(&path, 64 * 1024);
    let playback = manager().await;
    let session = playback
        .open(
            StreamingResource::File {
                path,
                len: 64 * 1024,
            },
            "application/octet-stream".into(),
            Some("task-1".into()),
        )
        .expect("open");
    let port = playback.gateway_port();
    let target = format!("/stream/{}", session.session_id);

    let r = http_request(port, &target, Some("127.0.0.1:9"), "GET", Some("bytes=0-9")).await;
    assert_eq!(r.status, 206);

    // Another owner's session is untouched.
    let other_dir = temp_dir("owner2");
    let other_path = other_dir.join("g.bin");
    write_pattern_at(&other_path, 4096);
    let other = playback
        .open(
            StreamingResource::File {
                path: other_path,
                len: 4096,
            },
            "application/octet-stream".into(),
            Some("task-2".into()),
        )
        .expect("open other");

    assert_eq!(
        playback.close_for_owner("task-1"),
        1,
        "one session invalidated"
    );
    assert_eq!(playback.close_for_owner("task-1"), 0, "idempotent");
    assert!(playback.info(&session.session_id).is_none(), "session gone");

    let r = http_request(port, &target, Some("127.0.0.1:9"), "GET", Some("bytes=0-9")).await;
    assert_eq!(r.status, 404, "invalidated session must stop serving");

    let r = http_request(
        port,
        &format!("/stream/{}", other.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-9"),
    )
    .await;
    assert_eq!(r.status, 206, "other owners' sessions are untouched");
}

// ---------------------------------------------------------------------
// §116/§137: a read blocked on unavailable data exits PROMPTLY when
// the session is closed — no dead reader, no timeout-length hang.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_close_aborts_blocked_read_promptly() {
    // A `.part` whose frontier can never grow: live watch sender held
    // by the test, never fired; no origin to read through.
    let dir = temp_dir("blocked");
    let path = dir.join("stuck.part");
    std::fs::write(&path, vec![0u8; 1024]).unwrap();
    let (tx, rx) = tokio::sync::watch::channel(1024u64);
    let part = StreamingResource::Part {
        path,
        total_len: Some(MIB),
        len_rx: Some(rx),
        origin: None,
    };
    let playback = manager().await;
    playback.set_read_timeout_ms(30_000);
    let session = playback.open(part, "video/mp4".into(), None).expect("open");
    let port = playback.gateway_port();
    let target = format!("/stream/{}", session.session_id);

    let client = tokio::spawn(async move {
        http_request(
            port,
            &target,
            Some("127.0.0.1:9"),
            "GET",
            Some("bytes=1024-65535"),
        )
        .await
    });
    tokio::time::sleep(Duration::from_millis(500)).await;

    let started = Instant::now();
    assert!(playback.close(&session.session_id), "close must succeed");
    let response = tokio::time::timeout(Duration::from_secs(15), client)
        .await
        .expect("client must NOT hang until the 30s read timeout")
        .expect("join");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "blocked reader must exit promptly, took {elapsed:?}"
    );
    assert_eq!(response.status, 206);
    assert!(
        response.body.len() < 64_512 - 1024,
        "body must be truncated"
    );
    let _ = tx; // sender dropped after close: no further growth by design
}

// ---------------------------------------------------------------------
// §100/§101/§125: the FULL gateway security matrix — arbitrary file
// read must be impossible; tokens are unguessable and revocable.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn security_matrix_full() {
    let dir = temp_dir("sec");
    let path = dir.join("f.bin");
    write_pattern_at(&path, 64 * 1024);
    let playback = manager().await;
    let session = playback
        .open(
            StreamingResource::File {
                path,
                len: 64 * 1024,
            },
            "application/octet-stream".into(),
            None,
        )
        .expect("open");
    let port = playback.gateway_port();
    let token = session.session_id.clone();

    // §100: arbitrary file read attempts — all rejected, empty bodies.
    for bad in [
        "/stream/../../secret",
        "/stream/..%5c..%5csecret",
        "/stream/%2e%2e/%2e%2e/secret",
        "/stream/%252e%252e/%252e%252e/secret",
        "/stream/C:%5cWindows%5cwin.ini",
        "/stream/C:/x",
        "/stream/%5c%5cserver%5cshare%5cx",
        "/stream/%5c%5f%3f%5cC:%5cx",
        "/stream/file%3a%2f%2f%2fC%3a/x",
        "/stream/....//....//secret",
        "/stream/..%2f..%2fsecret",
        "/stream/x?next=../../secret",
        "/stream/x#../../secret",
    ] {
        let r = http_request(port, bad, Some("127.0.0.1:9"), "GET", None).await;
        assert!(
            r.status == 403 || r.status == 404,
            "{bad} must be rejected, got {}",
            r.status
        );
        assert!(r.body.is_empty(), "{bad} must not leak bytes");
    }

    // §101: token lifecycle — expired and closed tokens are dead.
    let scheduler = Arc::new(DemandScheduler::new());
    let gw = ifuyo_tide_lib::core::streaming::StreamingGateway::start(scheduler)
        .await
        .expect("gateway");
    let other_path = dir.join("g.bin");
    write_pattern_at(&other_path, 1024);
    let temp = gw
        .create_session(
            StreamingResource::File {
                path: other_path,
                len: 1024,
            },
            Some(Duration::from_millis(80)),
        )
        .expect("create ttl session");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let r = http_request(
        gw.port,
        &format!("/stream/{temp}"),
        Some("127.0.0.1:9"),
        "GET",
        None,
    )
    .await;
    assert_eq!(r.status, 404, "expired token must be access denied");

    // Token from "another session" (guessable/sequential shapes).
    // The mutation must be guaranteed-different from the real token —
    // a char replace can be a no-op depending on the random UUID.
    let mut flipped = token.clone();
    let first = flipped.remove(0);
    flipped.insert(0, if first == '0' { '1' } else { '0' });
    for guess in [format!("{token}x"), flipped, "0".repeat(36), String::new()] {
        let r = http_request(
            port,
            &format!("/stream/{guess}"),
            Some("127.0.0.1:9"),
            "GET",
            None,
        )
        .await;
        assert_eq!(r.status, 404, "guessed token must be denied");
    }

    // §125: wrong HTTP methods are rejected with Allow.
    for method in ["POST", "PUT", "DELETE", "OPTIONS", "CONNECT", "TRACE"] {
        let r = http_request(
            port,
            &format!("/stream/{token}"),
            Some("127.0.0.1:9"),
            method,
            None,
        )
        .await;
        assert_eq!(r.status, 405, "{method} must be 405");
        assert_eq!(r.header("allow"), Some("GET, HEAD"));
    }

    // §125: the resource-gone case → 503 BEFORE headers.
    let missing = StreamingResource::Part {
        path: dir.join("never-existed.part"),
        total_len: Some(1024),
        len_rx: None,
        origin: None,
    };
    let gone = gw.create_session(missing, None).expect("create");
    let r = http_request(
        gw.port,
        &format!("/stream/{gone}"),
        Some("127.0.0.1:9"),
        "GET",
        None,
    )
    .await;
    assert_eq!(r.status, 503, "missing resource must be a controlled 503");
    let _ = playback.close(&token);
}

// ---------------------------------------------------------------------
// §136: fuzz/abuse battery — malformed input never panics, never
// overflows, never deadlocks, never serves arbitrary bytes.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fuzz_abuse_battery_never_panics() {
    let dir = temp_dir("fuzz");
    let path = dir.join("f.bin");
    write_pattern_at(&path, 64 * 1024);
    let playback = manager().await;
    let session = playback
        .open(
            StreamingResource::File {
                path,
                len: 64 * 1024,
            },
            "application/octet-stream".into(),
            None,
        )
        .expect("open");
    let port = playback.gateway_port();
    let token = session.session_id.clone();
    let target = format!("/stream/{token}");

    let hostile_ranges = [
        "bytes=0-18446744073709551615",
        "bytes=18446744073709551615-",
        "bytes=18446744073709551616-",
        "bytes=-18446744073709551615",
        "bytes=0-1-2",
        "bytes=--",
        "bytes=",
        "bytes=0-",
        "bytes= ",
        " bytes=0-5",
        "bytes=0 - 5",
        "bytes=+0-+5",
        "bytes=0x0-0x10",
        "bytes=99999999999999999999999999-",
        "bytes=5-0",
        "bytes=0-0",
        "units=0-1",
        "bytes=0-1, 5-6, 9-10",
        "\u{0}\u{1}\u{2}",
        "bytes=0-5 Range: evil",
    ];
    for range in hostile_ranges {
        let r = http_request(port, &target, Some("127.0.0.1:9"), "GET", Some(range)).await;
        assert!(
            r.status == 200 || r.status == 206 || r.status == 400 || r.status == 416,
            "range {range:?} must resolve to a controlled status, got {}",
            r.status
        );
    }

    // Hostile targets while the valid session is live.
    for bad_target in [
        format!("/{token}"),
        format!("/stream/{token}?x=1"),
        format!("/stream/{token}%00"),
        format!("/stream/{}%20", token),
        format!("/STREAM/{token}"),
        "/stream/%FF%FE".to_string(),
    ] {
        let r = http_request(port, &bad_target, Some("127.0.0.1:9"), "GET", None).await;
        assert!(
            r.status == 404 || r.status == 400 || r.status == 403,
            "target {bad_target:?} must be rejected, got {}",
            r.status
        );
    }

    // The server must still be healthy: a valid request succeeds.
    let r = http_request(
        port,
        &target,
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-99"),
    )
    .await;
    assert_eq!(r.status, 206);
    assert_eq!(r.body.len(), 100);
    r.assert_body_matches_pattern(0);
}

// ---------------------------------------------------------------------
// §118: Windows-specific — reading the .part through the gateway WHILE
// the engine appends to it must never yield torn or wrong bytes.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_read_while_writing_never_tears() {
    let server = common::spawn().await;
    let dir = temp_dir("rw");
    let engine = engine();
    let shared = engine
        .start(make_request(&server, "/slow/medium.bin", &dir), None)
        .await
        .expect("start download");
    wait_frontier(&shared, 64 * 1024).await;

    let part_path = shared.status().part_path.expect("part path");
    let part = StreamingResource::Part {
        path: part_path.clone(),
        total_len: Some(MIB),
        len_rx: Some(shared.subscribe_durable_len()),
        origin: None,
    };
    let playback = manager().await;
    let session = playback.open(part, "video/mp4".into(), None).expect("open");
    let port = playback.gateway_port();
    let target = format!("/stream/{}", session.session_id);

    // Read behind the frontier repeatedly WHILE the download continues.
    for i in 0..12u64 {
        let start = i * 32 * 1024;
        let frontier = physical_len(&part_path);
        if start + 32 * 1024 > frontier {
            wait_frontier(&shared, start + 32 * 1024).await;
        }
        let r = http_request(
            port,
            &target,
            Some("127.0.0.1:9"),
            "GET",
            Some(&format!("bytes={}-{}", start, start + 32 * 1024 - 1)),
        )
        .await;
        assert_eq!(r.status, 206, "read {i} at {start}");
        assert_eq!(r.body.len(), 32 * 1024);
        r.assert_body_matches_pattern(start);
    }
}

// ---------------------------------------------------------------------
// §119/§169/§196: performance baseline — REAL measured numbers, not
// adjectives. Printed for the report; sane floors asserted.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn performance_baseline_recorded() {
    // Gateway throughput over a 64 MiB local file (single full GET).
    let len: u64 = 64 * MIB;
    let dir = temp_dir("perf");
    let path = dir.join("big.bin");
    write_pattern_at(&path, len);
    let playback = manager().await;
    let session = playback
        .open(
            StreamingResource::File { path, len },
            "application/octet-stream".into(),
            None,
        )
        .expect("open");
    let started = Instant::now();
    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", session.session_id),
        Some("127.0.0.1:9"),
        "GET",
        None,
    )
    .await;
    let elapsed = started.elapsed();
    assert_eq!(r.status, 200);
    assert_eq!(r.body.len() as u64, len);
    let mib_per_s = (len as f64 / MIB as f64) / elapsed.as_secs_f64();
    println!("gateway throughput: {mib_per_s:.1} MiB/s over {len} bytes in {elapsed:?}");
    assert!(
        mib_per_s > 100.0,
        "gateway throughput floor: {mib_per_s:.1} MiB/s"
    );

    // Session create+close latency (1000 cycles).
    let started = Instant::now();
    let cycles = 500u32;
    for _ in 0..cycles {
        let s = playback
            .open(
                StreamingResource::File {
                    path: dir.join("big.bin"),
                    len,
                },
                "application/octet-stream".into(),
                None,
            )
            .expect("open");
        playback.close(&s.session_id);
    }
    let per_cycle = started.elapsed() / cycles;
    println!("session create+close: {} µs/cycle", per_cycle.as_micros());
    assert!(
        per_cycle < Duration::from_millis(10),
        "session churn too slow"
    );

    // Scheduler overhead: submit+complete pairs.
    let scheduler = Arc::new(DemandScheduler::new());
    let started = Instant::now();
    let ops = 100_000u32;
    for i in 0..ops {
        let range = ByteRange::new(i as u64 * 10, i as u64 * 10 + 8).unwrap();
        let id = scheduler.submit("bench", range, DemandReason::PlaybackBuffer, None);
        scheduler.complete(id);
    }
    let per_op = started.elapsed() / ops;
    println!("scheduler submit+complete: {} ns/op", per_op.as_nanos());
    assert!(per_op < Duration::from_micros(50), "scheduler too slow");

    // Read-through seek latency (origin hit, no frontier wait).
    let server = common::spawn().await;
    let part = StreamingResource::Part {
        path: dir.join("tiny.part"),
        total_len: Some(MIB),
        len_rx: None,
        origin: Some(OriginSource {
            url: server.url("/medium.bin"),
        }),
    };
    std::fs::write(dir.join("tiny.part"), [0u8; 64]).unwrap();
    let seek = playback
        .open(part, "video/mp4".into(), None)
        .expect("open seek session");
    let started = Instant::now();
    let r = http_request(
        playback.gateway_port(),
        &format!("/stream/{}", seek.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=524288-589823"),
    )
    .await;
    let seek_elapsed = started.elapsed();
    assert_eq!(r.status, 206);
    r.assert_body_matches_pattern(524288);
    println!("read-through seek (origin round trip): {seek_elapsed:?}");
    assert!(seek_elapsed < Duration::from_secs(5));
}

// ---------------------------------------------------------------------
// M5 §64 Test 7: source switch — the old session is released (gateway
// revoked) and the new session works.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_switch_releases_old_session() {
    let dir = temp_dir("switch");
    let playback = manager().await;
    let port = playback.gateway_port();

    let make = |name: &str, len: u64| {
        let path = dir.join(name);
        write_pattern_at(&path, len);
        StreamingResource::File { path, len }
    };
    let a = playback
        .open(make("a.bin", 64 * 1024), "video/mp4".into(), None)
        .expect("open A");
    let b = playback
        .open(make("b.bin", 64 * 1024), "video/mp4".into(), None)
        .expect("open B");

    // A was serving…
    let r = http_request(
        port,
        &format!("/stream/{}", a.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-9"),
    )
    .await;
    assert_eq!(r.status, 206);

    // …switch: the app stops A before opening B; simulate exactly that.
    assert!(playback.close(&a.session_id));
    let r = http_request(
        port,
        &format!("/stream/{}", a.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-9"),
    )
    .await;
    assert_eq!(r.status, 404, "old session must be dead after a switch");

    let r = http_request(
        port,
        &format!("/stream/{}", b.session_id),
        Some("127.0.0.1:9"),
        "GET",
        Some("bytes=0-9"),
    )
    .await;
    assert_eq!(r.status, 206, "new session must work");
}

// ---------------------------------------------------------------------
// M5 §64 Test 9: frontend reload — the backend session survives and
// the projection (metadata + last position) is recoverable.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn frontend_reload_recovers_active_session_projection() {
    let dir = temp_dir("reload");
    let path = dir.join("f.bin");
    write_pattern_at(&path, 64 * 1024);
    let playback = manager().await;
    let session = playback
        .open(
            StreamingResource::File {
                path,
                len: 64 * 1024,
            },
            "video/mp4".into(),
            None,
        )
        .expect("open");

    // The player reports its position (throttled)…
    assert!(playback.report_position(&session.session_id, 12_345));

    // …the "frontend reloads": React state is gone, the backend still
    // knows the active session and its projection.
    let active = playback.active_session().expect("active session");
    assert_eq!(active.session_id, session.session_id);
    assert_eq!(active.last_position_ms, 12_345);

    // Availability still refreshes through the same session.
    let status = playback.status(&session.session_id).await.expect("status");
    assert_eq!(status.available_bytes, 64 * 1024);
    assert!(status.last_position_ms == 12_345);

    // Nothing playing → None (no stale session is fabricated).
    playback.close(&session.session_id);
    assert!(playback.active_session().is_none());
}
