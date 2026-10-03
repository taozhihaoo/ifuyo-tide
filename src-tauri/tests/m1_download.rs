//! M1 integration tests: the download engine against the local test
//! server (M1 §38/§39/§54). Every case runs REAL HTTP over loopback.

mod common;

use common::{pattern_byte, pattern_sha256, spawn, TestServer};
use ifuyo_tide_lib::core::download::file_sink::OverwritePolicy;
use ifuyo_tide_lib::core::download::url::{validate_download_url, DownloadFailure};
use ifuyo_tide_lib::core::download::DownloadEngine;
use ifuyo_tide_lib::core::download::DownloadLifecycle;
use ifuyo_tide_lib::core::download::DownloadRequest;
use ifuyo_tide_lib::error::TideError;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m1-{}-{}",
        label,
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

async fn wait_terminal(
    engine: &DownloadEngine,
    id: &str,
    timeout: Duration,
) -> ifuyo_tide_lib::core::download::DownloadStatus {
    let started = Instant::now();
    loop {
        let shared = engine.get(id).await.expect("entry exists");
        let status = shared.status();
        if matches!(
            status.lifecycle,
            DownloadLifecycle::Completed | DownloadLifecycle::Failed | DownloadLifecycle::Cancelled
        ) {
            return status;
        }
        if started.elapsed() > timeout {
            panic!(
                "download did not reach a terminal state in {timeout:?}: {:?}",
                status.lifecycle
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_progress_at_least(
    engine: &DownloadEngine,
    id: &str,
    min_bytes: u64,
    timeout: Duration,
) {
    let started = Instant::now();
    loop {
        let shared = engine.get(id).await.expect("entry exists");
        if shared.status().downloaded >= min_bytes {
            return;
        }
        if started.elapsed() > timeout {
            panic!("download made no progress in {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn make_request(
    server: &TestServer,
    path: &str,
    dir: &PathBuf,
    checksum: Option<String>,
) -> DownloadRequest {
    let url = validate_download_url(&server.url(path), true).expect("valid test url");
    DownloadRequest::new(
        url,
        dir.clone(),
        None,
        OverwritePolicy::Overwrite,
        checksum.map(|hex| {
            (
                ifuyo_tide_lib::core::download::checksum::ChecksumAlgorithm::Sha256,
                hex,
            )
        }),
    )
    .expect("request")
}

fn verify_file(path: &PathBuf, len: u64) {
    let meta = std::fs::metadata(path).expect("final file exists");
    assert_eq!(meta.len(), len, "final file size");
    // Spot-verify the pattern at several offsets (full verify for small).
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).expect("open");
    let mut buf = vec![0u8; 1024];
    for probe in [0u64, len / 2, len - 1024] {
        file.seek(SeekFrom::Start(probe)).unwrap();
        let take = 1024.min((len - probe) as usize);
        file.read_exact(&mut buf[..take]).unwrap();
        for (i, b) in buf.iter().enumerate().take(take) {
            assert_eq!(
                *b,
                pattern_byte(probe + i as u64),
                "byte at {}",
                probe + i as u64
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn full_download_with_checksum_and_atomic_finalize() {
    let server = spawn().await;
    let dir = temp_dir("full");
    let len: u64 = 1024 * 1024;
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/slow/medium.bin", &dir, Some(pattern_sha256(len)));
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    assert_eq!(status.downloaded, len);
    let final_path = status.final_path.expect("final path recorded");
    verify_file(&final_path, len);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pause_resume_uses_range_and_completes() {
    let server = spawn().await;
    let dir = temp_dir("pause-resume");
    let len: u64 = 1024 * 1024;
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    // SHA-256 across a pause/resume boundary: the digest must match the
    // full pattern, proving resumed bytes are byte-exact (M1 §51/§30).
    let request = make_request(&server, "/slow/medium.bin", &dir, Some(pattern_sha256(len)));
    let shared = engine.start(request, None).await.expect("start");
    let id = shared.request.download_id.clone();

    // Let it make some progress, then pause.
    wait_progress_at_least(&engine, &id, 256 * 1024, Duration::from_secs(15)).await;
    engine.pause(&id).await.expect("pause");
    // Wait for Paused state.
    loop {
        if matches!(shared.status().lifecycle, DownloadLifecycle::Paused) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let paused_status = shared.status();
    let partial = paused_status.downloaded;
    assert!(partial > 0, "partial bytes before pause");
    let part_path = paused_status.part_path.expect("part path recorded");
    assert!(part_path.exists(), ".part file is kept on pause");

    let ranges_before = server.seen_ranges().len();

    engine.resume(&id, None).await.expect("resume");
    let status = wait_terminal(&engine, &id, Duration::from_secs(30)).await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );

    // The resume attempt must have carried a Range request (M1 §51/§79).
    let ranges_after = server.seen_ranges();
    assert!(
        ranges_after.len() > ranges_before,
        "server must observe a Range request on resume: {ranges_after:?}"
    );
    assert!(
        ranges_after.iter().any(|r| r.starts_with("bytes=")),
        "range header format"
    );
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_discards_part_and_never_finalizes() {
    let server = spawn().await;
    let dir = temp_dir("cancel");
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/slow/medium.bin", &dir, None);
    let shared = engine.start(request, None).await.expect("start");
    let id = shared.request.download_id.clone();
    wait_progress_at_least(&engine, &id, 128 * 1024, Duration::from_secs(15)).await;
    let part = shared.status().part_path.expect("part path");
    engine.cancel(&id).await.expect("cancel");
    let status = wait_terminal(&engine, &id, Duration::from_secs(15)).await;
    assert_eq!(status.lifecycle, DownloadLifecycle::Cancelled);
    assert!(!part.exists(), "cancel deletes the .part (M1 §21)");
    assert!(status.final_path.is_none(), "no final file on cancel");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unstable_connection_retries_and_resumes_from_partial() {
    let server = spawn().await;
    let dir = temp_dir("retry");
    let len: u64 = 512 * 1024;
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(
        &server,
        "/unstable/file.bin",
        &dir,
        Some(pattern_sha256(len)),
    );
    let shared = engine.start(request, None).await.expect("start");
    let id = shared.request.download_id.clone();
    let status = wait_terminal(&engine, &id, Duration::from_secs(30)).await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    assert!(
        status.attempts >= 1,
        "the reset connection must have triggered a retry (attempts={})",
        status.attempts
    );
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn redirect_chain_downloads_final_resource() {
    let server = spawn().await;
    let dir = temp_dir("redirect");
    let len: u64 = 1024 * 1024;
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/redirect/0", &dir, Some(pattern_sha256(len)));
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chunked_download_completes_without_total() {
    let server = spawn().await;
    let dir = temp_dir("chunked");
    let len: u64 = 256 * 1024;
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(
        &server,
        "/chunked/test.bin",
        &dir,
        Some(pattern_sha256(len)),
    );
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http_404_fails_without_retry() {
    let server = spawn().await;
    let dir = temp_dir("404");
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/status/404", &dir, None);
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(15),
    )
    .await;
    assert_eq!(status.lifecycle, DownloadLifecycle::Failed);
    assert_eq!(
        status.error.as_ref().map(|(code, _)| code.as_str()),
        Some("DOWNLOAD_HTTP_STATUS")
    );
    assert_eq!(status.attempts, 0, "404 must not be retried (M1 §19)");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn checksum_mismatch_fails_and_keeps_part() {
    let server = spawn().await;
    let dir = temp_dir("checksum");
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/small.bin", &dir, Some("0".repeat(64)));
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(status.lifecycle, DownloadLifecycle::Failed);
    assert_eq!(
        status.error.as_ref().map(|(code, _)| code.as_str()),
        Some("DOWNLOAD_CHECKSUM_MISMATCH")
    );
    let part = status.part_path.expect("part kept for diagnosis (M1 §30)");
    assert!(part.exists());
    let _ = std::fs::remove_dir_all(dir);
}

/// The single most important safety case (M1 §79): a part file plus a
/// server that ignores Range must NEVER be appended into a corrupt file.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn range_ignored_server_restarts_instead_of_appending() {
    let server = spawn().await;
    let dir = temp_dir("norange");
    let len: u64 = 512 * 1024;
    // Simulate a partial from an earlier session: first half of the
    // pattern (the representation HASN'T changed — pattern is stable).
    let part = dir.join("norange.bin.part");
    let mut buf = Vec::with_capacity((len / 2) as usize);
    for i in 0..len / 2 {
        buf.push(pattern_byte(i));
    }
    std::fs::write(&part, &buf).expect("pre-seed part");

    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(
        &server,
        "/norange/file.bin",
        &dir,
        Some(pattern_sha256(len)),
    );
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    // Correctness proof: the final file matches the FULL pattern — if the
    // engine had appended the 200 body to the part, the checksum and the
    // spot verification would both fail.
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

/// Case C (M1 §39): validators recorded, remote changed → full restart,
/// never a stale-representation append.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn remote_changed_restarts_from_zero() {
    let server = spawn().await;
    let dir = temp_dir("changed");
    let len: u64 = 256 * 1024;
    // First GET bumps the server's ETag to v0 and we keep a "partial"
    // claiming v0; the next request bumps to v1 → If-Range(v0) mismatches
    // → the server returns 200 and the engine must restart from zero.
    let _ = std::fs::write(dir.join("changing.bin.part"), vec![0u8; 64 * 1024]);

    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/changing/file.bin", &dir, None);
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    // The engine restarted (validators mismatch) and wrote the complete
    // new representation over the stale part.
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

/// Case G (M1 §39): local already complete + 416 → finalize.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_local_with_416_finalizes() {
    let server = spawn().await;
    let dir = temp_dir("416");
    let len: u64 = 2 * 1024 * 1024;
    // A complete patterned part: resume asks bytes=len- → 416 with
    // Content-Range total == local size → finalize, no re-download.
    let part = dir.join("range.bin.part");
    let mut buf = Vec::with_capacity(len as usize);
    for i in 0..len {
        buf.push(pattern_byte(i));
    }
    std::fs::write(&part, &buf).expect("seed complete part");

    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/range/file.bin", &dir, Some(pattern_sha256(len)));
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(
        &engine,
        &shared.request.download_id,
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    verify_file(&status.final_path.expect("final"), len);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ssrf_baseline_blocks_private_literal_urls() {
    // Engine-level: the request builder rejects private literals before
    // any network I/O.
    let dir = temp_dir("ssrf");
    let err = validate_download_url("http://127.0.0.1/file", false).unwrap_err();
    match err {
        TideError::Download(DownloadFailure::PrivateAddressBlocked { host }) => {
            assert_eq!(host, "127.0.0.1");
        }
        other => panic!("expected PrivateAddressBlocked, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn true_zero_byte_download_completes() {
    // M1 §38: a REAL zero-byte representation (Content-Length: 0), not a
    // small-file stand-in. The engine must complete, finalize atomically
    // and produce an empty final file.
    let server = spawn().await;
    let dir = temp_dir("zero");
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/empty.bin", &dir, Some(pattern_sha256(0)));
    let shared = engine.start(request, None).await.expect("start");
    let status = wait_terminal(&engine, &shared.request.download_id, Duration::from_secs(30)).await;
    assert_eq!(status.lifecycle, DownloadLifecycle::Completed, "{:?}", status.error);
    assert_eq!(status.downloaded, 0);
    let final_path = status.final_path.expect("final");
    let meta = std::fs::metadata(&final_path).expect("final exists");
    assert_eq!(meta.len(), 0, "final file must be exactly 0 bytes");
    assert!(!status.part_path.as_ref().is_some_and(|p| p.exists()), "part consumed by finalize");
    let _ = std::fs::remove_dir_all(dir);
}

/// Large-file proof (M1 §29): u64 offsets beyond 4 GiB, real transfer on
/// Windows. Sparse-content generator keeps the test time bounded while
/// the DOWNLOADED file is a real 4 GB+ file on disk.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "long-running large-file proof; run explicitly with --ignored"]
async fn huge_file_beyond_4gib_downloads_and_verifies() {
    let server = spawn().await;
    let dir = temp_dir("huge");
    let len: u64 = 4 * 1024 * 1024 * 1024 + 512; // > 4 GiB
    let engine = DownloadEngine::new(std::sync::Arc::new(ifuyo_tide_lib::persistence::repositories::DownloadRepository::open_in_memory().unwrap()));
    let request = make_request(&server, "/huge.bin", &dir, Some(pattern_sha256(len)));
    let shared = engine.start(request, None).await.expect("start");
    let id = shared.request.download_id.clone();
    let status = wait_terminal(&engine, &id, Duration::from_secs(600)).await;
    assert_eq!(
        status.lifecycle,
        DownloadLifecycle::Completed,
        "{:?}",
        status.error
    );
    let final_path = status.final_path.expect("final");
    let meta = std::fs::metadata(&final_path).expect("final exists");
    assert_eq!(meta.len(), len);
    // Spot checks deep past the 4 GiB line.
    verify_file(&final_path, len);
    let _ = std::fs::remove_dir_all(dir);
}
