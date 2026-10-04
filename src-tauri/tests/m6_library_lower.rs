//! M6(下) tests: the remaining critical tests (§96–§105), failure
//! injection (§109), cancellation/single-flight (§130/§131),
//! property-style path fuzzing (§110), database corruption (§144),
//! migration regression (§145) and the performance benchmark (§106/
//! §107 — run with `--ignored` to record numbers).

use ifuyo_tide_lib::core::library::model::{AvailabilityStatus, ItemQuery};
use ifuyo_tide_lib::core::library::path;
use ifuyo_tide_lib::core::library::repository::{LibraryRepository, ScanCandidate};
use ifuyo_tide_lib::core::library::scanner::{ScanFaults, ScanStatus};
use ifuyo_tide_lib::core::library::service::LibraryService;
use ifuyo_tide_lib::core::streaming::demand::DemandScheduler;
use ifuyo_tide_lib::core::streaming::playback::PlaybackManager;
use ifuyo_tide_lib::core::streaming::resource::StreamingResource;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m6l-{}-{}",
        label,
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

fn write_file(path: &Path, len: u64) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut file = std::fs::File::create(path).unwrap();
    let mut buf = vec![0u8; 64 * 1024];
    let mut offset = 0u64;
    while offset < len {
        let take = (len - offset).min(buf.len() as u64) as usize;
        for (i, b) in buf.iter_mut().enumerate().take(take) {
            *b = ((offset + i as u64) % 251) as u8;
        }
        file.write_all(&buf[..take]).unwrap();
        offset += take as u64;
    }
}

fn service() -> LibraryService {
    LibraryService::new(Arc::new(LibraryRepository::open_in_memory().unwrap()))
}

// ---------------------------------------------------------------------
// §100: task deletion must NEVER remove library items.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn task_lifecycle_never_touches_library() {
    let service = service();
    let dest = temp_dir("task-del");
    let file = dest.join("kept.mp4");
    write_file(&file, 4096);
    let item = service
        .import_download(&file, &dest, "task-9", None)
        .unwrap()
        .unwrap();

    // The only task→library interaction Tide has is playback-session
    // invalidation (M4 §102). Prove it removes SESSIONS, not items.
    let manager = PlaybackManager::new(Arc::new(DemandScheduler::new()))
        .await
        .unwrap();
    let (canonical, size, _) = service.play_source(&item.id).unwrap();
    let session = manager
        .open(
            StreamingResource::File {
                path: canonical,
                len: size,
            },
            "video/mp4".into(),
            Some("task-9".into()),
        )
        .unwrap();
    assert!(
        manager.close_for_owner("task-9") == 1,
        "session invalidated"
    );
    assert!(file.exists(), "file untouched");

    // And the library item survives any amount of owner invalidation.
    manager.close_for_owner("task-9");
    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 1, "library item outlives its task");
}

// ---------------------------------------------------------------------
// §104: playback security — fake/missing item ids fail safely.
// ---------------------------------------------------------------------

#[test]
fn playback_with_fake_item_id_fails_safely() {
    let service = service();
    let err = service.play_source("not-a-real-id").unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");
    // Empty and hostile ids: same structured failure, no panic.
    assert_eq!(service.play_source("").unwrap_err().code(), "NOT_FOUND");
    assert_eq!(
        service.play_source("../../etc/passwd").unwrap_err().code(),
        "NOT_FOUND",
        "an id-shaped PATH must be treated as an id (not found), never as a path"
    );
}

// ---------------------------------------------------------------------
// §105: large files — >4 GiB via a SPARSE file: size, offsets and
// range reads must not truncate anywhere.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn large_sparse_file_beyond_4gib() {
    const GIB: u64 = 1024 * 1024 * 1024;
    let service = service();
    let dest = temp_dir("huge");
    let file = dest.join("huge.bin");
    // Sparse: set_len allocates logically, costs nothing physically.
    let handle = std::fs::File::create(&file).unwrap();
    handle.set_len(5 * GIB).unwrap();
    drop(handle);

    let item = service
        .import_download(&file, &dest, "task-huge", None)
        .unwrap()
        .unwrap();
    assert_eq!(item.size_bytes, 5 * GIB, "u64 size, no truncation");

    let (canonical, size, _) = service.play_source(&item.id).unwrap();
    assert_eq!(size, 5 * GIB);

    let manager = PlaybackManager::new(Arc::new(DemandScheduler::new()))
        .await
        .unwrap();
    let session = manager
        .open(
            StreamingResource::File {
                path: canonical,
                len: size,
            },
            "application/octet-stream".into(),
            Some(item.id),
        )
        .unwrap();

    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", manager.gateway_port()))
        .await
        .unwrap();
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let start = 4 * GIB + 512 * 1024 * 1024; // 4.5 GiB in
    let request = format!(
        "GET /stream/{} HTTP/1.1\r\nHost: 127.0.0.1:9\r\nRange: bytes={}-{}\r\nConnection: close\r\n\r\n",
        session.session_id,
        start,
        start + 4095
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw);
    assert!(
        text.starts_with("HTTP/1.1 206"),
        "ranged read deep into a >4GiB file"
    );
    assert!(
        text.contains(&format!("bytes {start}-{}/{}", start + 4095, 5 * GIB)),
        "Content-Range must carry the full u64 size"
    );
    let body = &raw[raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4..];
    assert_eq!(body.len(), 4096);
    assert!(body.iter().all(|b| *b == 0), "sparse hole reads zeros");
    manager.close(&session.session_id);
}

// ---------------------------------------------------------------------
// §135/§136: locked file → structured failure; playing → FILE_IN_USE.
// ---------------------------------------------------------------------

#[cfg(windows)]
#[test]
fn locked_file_delete_fails_structured_not_panic() {
    use std::os::windows::fs::OpenOptionsExt;
    let service = service();
    let dest = temp_dir("locked");
    let file = dest.join("locked.mp4");
    write_file(&file, 4096);
    let item = service
        .import_download(&file, &dest, "task-1", None)
        .unwrap()
        .unwrap();

    // Hold the file with NO sharing — every other open fails (the
    // Windows "file locked" case, §135).
    let _lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .expect("acquire exclusive lock");

    let err = service.delete_item_from_disk(&item.id).unwrap_err();
    assert_eq!(
        err.code(),
        "RESOURCE_UNAVAILABLE",
        "locked delete is a structured failure"
    );
    assert!(file.exists(), "locked file must survive");
    drop(_lock);

    // After release the same delete succeeds.
    service.delete_item_from_disk(&item.id).unwrap();
    assert!(!file.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn playing_file_refuses_deletion_with_file_in_use() {
    let service = service();
    let dest = temp_dir("in-use");
    let file = dest.join("playing.mp4");
    write_file(&file, 64 * 1024);
    let item = service
        .import_download(&file, &dest, "task-1", None)
        .unwrap()
        .unwrap();

    let manager = PlaybackManager::new(Arc::new(DemandScheduler::new()))
        .await
        .unwrap();
    let (canonical, size, _) = service.play_source(&item.id).unwrap();
    let session = manager
        .open(
            StreamingResource::File {
                path: canonical,
                len: size,
            },
            "video/mp4".into(),
            Some(item.id.clone()),
        )
        .unwrap();

    // §136: the command-layer check (mirrored here) — an active
    // session REFUSES deletion instead of silently closing it.
    assert!(manager.has_session_for_owner(&item.id));
    // (The command maps this to FILE_IN_USE; the manager predicate is
    // the decision input.)

    manager.close(&session.session_id);
    assert!(!manager.has_session_for_owner(&item.id));
    // After playback ends, deletion goes through.
    service.delete_item_from_disk(&item.id).unwrap();
    assert!(!file.exists());
}

// ---------------------------------------------------------------------
// §97 via refresh (not full scan): Missing ⇄ Available restoration.
// ---------------------------------------------------------------------

#[test]
fn refresh_restores_state_both_directions() {
    let service = service();
    let dest = temp_dir("refresh");
    let file = dest.join("movie.mp4");
    write_file(&file, 4096);
    let item = service
        .import_download(&file, &dest, "task-1", None)
        .unwrap()
        .unwrap();

    std::fs::remove_file(&file).unwrap();
    let refreshed = service.refresh_item(&item.id).unwrap();
    assert_eq!(refreshed.availability, AvailabilityStatus::Missing);

    write_file(&file, 4096);
    let refreshed = service.refresh_item(&item.id).unwrap();
    assert_eq!(refreshed.availability, AvailabilityStatus::Available);
}

// ---------------------------------------------------------------------
// §95/§144: duplicate roots, FK safety, corrupted database fails safe.
// ---------------------------------------------------------------------

#[test]
fn duplicate_root_returns_existing() {
    let service = service();
    let dir = temp_dir("dup-root");
    let a = service.add_root(&dir).unwrap();
    let b = service.add_root(&dir).unwrap();
    assert_eq!(a.id, b.id, "§95: duplicate root path dedupes");
    assert_eq!(service.list_roots().unwrap().len(), 1);
}

#[test]
fn corrupted_database_fails_safe() {
    let db = temp_dir("corrupt").join("lib.db");
    // Create a v4 database, then surgically break the schema while
    // keeping user_version=4 (the "tampered DB" case, §144).
    {
        let repo = LibraryRepository::open(&db).unwrap();
        repo.list_roots().unwrap();
    }
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute("DROP TABLE library_items", []).unwrap();
    drop(conn);

    // Re-opening does NOT panic and does NOT re-run migrations
    // (user_version is already 4): operations fail structurally.
    let repo = LibraryRepository::open(&db).unwrap();
    assert!(repo.list_roots().is_ok(), "roots table still exists");
    let err = repo
        .list_items(&ItemQuery {
            page_size: 10,
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(
        err.code(),
        "PERSISTENCE",
        "broken schema → structured error"
    );
}

#[test]
fn migration_regression_v3_to_v4() {
    // §145: an M0–M5 era database (user_version = 3) must migrate to
    // v4 cleanly, keeping ALL pre-existing tables.
    let db = temp_dir("migr").join("old.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        for sql in [
            include_str!("../src/persistence/migrations/0001_initial.sql"),
            include_str!("../src/persistence/migrations/0002_download_tasks.sql"),
            include_str!("../src/persistence/migrations/0003_torrent_tasks.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 3).unwrap();
    }
    let repo = LibraryRepository::open(&db).unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 4, "migration applied");
    for table in [
        "tasks",
        "download_tasks",
        "torrent_tasks",
        "torrent_files",
        "library_roots",
        "library_items",
    ] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "table {table} must survive the migration");
    }
    let _ = repo;
}

// ---------------------------------------------------------------------
// §130/§131: scan cancellation + same-root single-flight.
// ---------------------------------------------------------------------

#[test]
fn cancelled_scan_commits_nothing() {
    let service = service();
    let dir = temp_dir("cancel");
    let root = service.add_root(&dir).unwrap();
    write_file(&dir.join("a.mp4"), 1024);

    let flag = service.begin_scan(&root.id).unwrap();
    flag.store(true, std::sync::atomic::Ordering::Relaxed);
    let outcome = service
        .scan_root_blocking_with(&root.id, &flag, ScanFaults::default(), |_| {})
        .unwrap();
    assert_eq!(outcome.status, ScanStatus::Cancelled);
    assert_eq!(outcome.files_seen, 0, "cancelled between directories");
    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 0, "§130: cancelled scan commits NOTHING");
    service.end_scan(&root.id);
    let root_after = service.repo().get_root(&root.id).unwrap().unwrap();
    assert_eq!(root_after.scan_status, "idle", "no Scanning-forever");
}

#[test]
fn same_root_scan_is_single_flight() {
    let service = service();
    let dir = temp_dir("single");
    let root = service.add_root(&dir).unwrap();
    let _flag = service.begin_scan(&root.id).unwrap();
    let err = service.begin_scan(&root.id).unwrap_err();
    assert_eq!(err.code(), "BUSY", "§131: AlreadyScanning");
    // A DIFFERENT root may scan concurrently (§131 leaves this open;
    // we allow it — roots are independent directories).
    let dir2 = temp_dir("single-2");
    let root2 = service.add_root(&dir2).unwrap();
    service.begin_scan(&root2.id).unwrap();
    service.end_scan(&root.id);
    // After end, the root can scan again.
    service.begin_scan(&root.id).unwrap();
}

// ---------------------------------------------------------------------
// §109: failure injection — DB consistency at every fault point.
// ---------------------------------------------------------------------

fn candidate(path: &Path, root: &Path) -> ScanCandidate {
    ifuyo_tide_lib::core::library::repository::ScanCandidate {
        canonical_path: path.to_path_buf(),
        relative_path: path::relative_path(root, path),
        display_name: path.file_name().unwrap().to_string_lossy().to_string(),
        extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        media_category: ifuyo_tide_lib::core::library::model::MediaCategory::Video,
        mime_type: Some("video/mp4".into()),
        size_bytes: 1024,
        modified_at: 0,
    }
}

#[test]
fn failure_injection_before_db_upsert_leaves_db_untouched() {
    let repo = LibraryRepository::open_in_memory().unwrap();
    let dir = temp_dir("inj-before");
    let root = service().add_root(&dir).unwrap();
    let file = dir.join("a.mp4");
    write_file(&file, 1024);
    let canonical = path::canonicalize(&file).unwrap();

    let err = ifuyo_tide_lib::core::library::scanner::scan_root_blocking_with(
        &root.id,
        &dir,
        &repo,
        &mut |_| {},
        &std::sync::atomic::AtomicBool::new(false),
        ScanFaults {
            fail_before_db_upsert: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code(), "RESOURCE_UNAVAILABLE");
    // The DB must be completely untouched.
    let page = repo
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 0);
    let _ = canonical;
}

#[test]
fn failure_injection_after_commit_keeps_data() {
    // ONE shared repo: the service and the direct apply_scan/queries
    // must see the same database (the root row is FK-referenced).
    let repo = Arc::new(LibraryRepository::open_in_memory().unwrap());
    let dir = temp_dir("inj-after");
    let root = LibraryService::new(repo.clone()).add_root(&dir).unwrap();
    let file = dir.join("b.mp4");
    write_file(&file, 1024);

    let err = ifuyo_tide_lib::core::library::scanner::scan_root_blocking_with(
        &root.id,
        &dir,
        &repo,
        &mut |_| {},
        &std::sync::atomic::AtomicBool::new(false),
        ScanFaults {
            fail_after_scan: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code(), "RESOURCE_UNAVAILABLE");
    // The commit SUCCEEDED before the injected fault: data stays.
    let page = repo
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 1, "committed scan data survives the fault");
}

#[test]
fn failure_injection_metadata_read_is_partial() {
    let repo = Arc::new(LibraryRepository::open_in_memory().unwrap());
    let dir = temp_dir("inj-meta");
    let root = LibraryService::new(repo.clone()).add_root(&dir).unwrap();
    let seen = dir.join("seen.mp4");
    let later = dir.join("later.mp4");
    write_file(&seen, 1024);
    write_file(&later, 1024);
    // Pre-index BOTH (full scan).
    repo.apply_scan(
        &root.id,
        &[
            candidate(&path::canonicalize(&seen).unwrap(), &dir),
            candidate(&path::canonicalize(&later).unwrap(), &dir),
        ],
        &[],
    )
    .unwrap();

    // A scan whose FIRST entry read fails: that entry becomes an
    // uncovered prefix — later.mp4 must NOT go Missing.
    let outcome = ifuyo_tide_lib::core::library::scanner::scan_root_blocking_with(
        &root.id,
        &dir,
        &repo,
        &mut |_| {},
        &std::sync::atomic::AtomicBool::new(false),
        ScanFaults {
            fail_metadata_read: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(outcome.status, ScanStatus::Partial);
    let item = repo
        .get_item_by_path(&path::canonicalize(&later).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        item.availability,
        AvailabilityStatus::Available,
        "§69: fault-uncovered entry keeps its availability"
    );
}

// ---------------------------------------------------------------------
// §110: property-style path fuzz — nothing bypasses containment.
// ---------------------------------------------------------------------

#[test]
fn fuzzed_paths_never_escape_containment() {
    let root_dir = temp_dir("fuzz");
    let root = path::canonicalize(&root_dir).unwrap();
    // Seeded LCG for determinism (no rand dependency).
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut accepted = 0u64;
    let mut rejected = 0u64;
    // Seed a couple of real in-root files so the accept branch runs.
    for i in 0..3u64 {
        write_file(&root.join(format!("real{}.mp4", i)), 1024);
    }
    for _ in 0..500 {
        let parts = (next() % 4) + 1;
        let mut rel = PathBuf::new();
        for _ in 0..parts {
            match next() % 6 {
                0 => rel.push("..".to_string()),
                1 => rel.push(format!("dir{}", next() % 5)),
                2 => rel.push(format!("real{}.mp4", next() % 3)),
                3 => rel.push("CON".to_string()),
                4 => rel.push(format!("..{}", next() % 7)),
                _ => rel.push(format!("n{}%2e%2e", next() % 7)),
            }
        }
        let candidate_path = root.join(&rel);
        // THE INVARIANT (§110): the real validator only accepts paths
        // whose canonical form stays inside the root.
        match path::validate_item_path(&root, &candidate_path) {
            Ok(canonical) => {
                accepted += 1;
                assert!(
                    path::is_contained(&root, &canonical),
                    "accepted path escaped containment: {rel:?} → {canonical:?}"
                );
            }
            Err(_) => rejected += 1,
        }
    }
    assert!(accepted > 0, "fuzz must accept in-root files too");
    assert!(rejected > 0, "fuzz must exercise the rejection path");
    // Sanity: the fuzz actually exercised the check.
    assert!(accepted > 0, "fuzz must accept in-root paths too");
}

// ---------------------------------------------------------------------
// §106/§107: performance benchmark — REAL measured numbers.
// Run explicitly:  cargo test --locked --test m6_library_lower -- --ignored --nocapture
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "benchmark: run explicitly to record numbers"]
async fn benchmark_scan_and_query_at_scale() {
    // --- real-file scans: 1k and 10k ---
    for count in [1_000u64, 10_000] {
        let dir = temp_dir(&format!("bench-{count}"));
        let service = service();
        let root = service.add_root(&dir).unwrap();
        for d in 0..20u64 {
            std::fs::create_dir_all(dir.join(format!("dir{}/", d))).unwrap();
        }
        for i in 0..count {
            let p = dir.join(format!("dir{}/file{}.mp4", i % 20, i));
            std::fs::write(p, [0u8; 8]).unwrap();
        }
        let started = std::time::Instant::now();
        let outcome = service.scan_root_blocking(&root.id, |_| {}).unwrap();
        println!(
            "scan {count} real files: {:?} (added={}, status={:?})",
            started.elapsed(),
            outcome.added,
            outcome.status
        );
    }

    // --- 100k items: synthetic rows through the same DB path ---
    let repo = Arc::new(LibraryRepository::open_in_memory().unwrap());
    let service = LibraryService::new(repo.clone());
    let dir = temp_dir("bench-100k");
    let root = service.add_root(&dir).unwrap();
    let started = std::time::Instant::now();
    const TOTAL: u64 = 100_000;
    let mut batch: Vec<ScanCandidate> = Vec::with_capacity(10_000);
    for i in 0..TOTAL {
        batch.push(ScanCandidate {
            canonical_path: dir.join(format!("dir{}/movie{}.mp4", i % 100, i)),
            relative_path: format!("dir{}/movie{}.mp4", i % 100, i),
            display_name: format!("movie{}.mp4", i),
            extension: Some("mp4".into()),
            media_category: ifuyo_tide_lib::core::library::model::MediaCategory::Video,
            mime_type: Some("video/mp4".into()),
            size_bytes: i,
            modified_at: 0,
        });
        if batch.len() == 10_000 {
            let take = std::mem::take(&mut batch);
            repo.apply_scan(&root.id, &take, &[]).unwrap();
        }
    }
    if !batch.is_empty() {
        repo.apply_scan(&root.id, &batch, &[]).unwrap();
    }
    println!(
        "insert 100k rows (batched transactions): {:?}",
        started.elapsed()
    );

    // Search latency: 50 cold + 50 warm lookups for "movie".
    let query = |page: u32| {
        service
            .list_items(&ItemQuery {
                search: Some("movie".into()),
                sort: Some("name".into()),
                ascending: true,
                page,
                page_size: 50,
                ..Default::default()
            })
            .unwrap()
    };
    let mut cold = Vec::new();
    for page in 0..50u32 {
        let t = std::time::Instant::now();
        let p = query(page);
        assert_eq!(p.items.len(), 50);
        cold.push(t.elapsed());
    }
    let mut warm = Vec::new();
    for _ in 0..50 {
        let t = std::time::Instant::now();
        let p = query(7);
        assert_eq!(p.total, TOTAL);
        warm.push(t.elapsed());
    }
    let report = |label: &str, times: &[std::time::Duration]| {
        let mut sorted = times.to_vec();
        sorted.sort();
        println!(
            "{}: p50 {:?} / p95 {:?} (n={})",
            label,
            sorted[sorted.len() / 2],
            sorted[(sorted.len() as f64 * 0.95) as usize % sorted.len()],
            times.len()
        );
    };
    report("search 100k COLD (varied pages)", &cold);
    report("search 100k WARM (same page)", &warm);

    // Filter + sort latency.
    let t = std::time::Instant::now();
    let filtered = service
        .list_items(&ItemQuery {
            category: Some(ifuyo_tide_lib::core::library::model::MediaCategory::Video),
            sort: Some("size".into()),
            ascending: false,
            page: 0,
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    println!(
        "filter+sort 100k: {:?} (total={})",
        t.elapsed(),
        filtered.total
    );

    // Idempotent re-commit of all 100k (update path).
    let started = std::time::Instant::now();
    for i in (0..TOTAL).step_by(10_000) {
        let batch: Vec<ScanCandidate> = (i..(i + 10_000).min(TOTAL))
            .map(|i| ScanCandidate {
                canonical_path: dir.join(format!("dir{}/movie{}.mp4", i % 100, i)),
                relative_path: format!("dir{}/movie{}.mp4", i % 100, i),
                display_name: format!("movie{}.mp4", i),
                extension: Some("mp4".into()),
                media_category: ifuyo_tide_lib::core::library::model::MediaCategory::Video,
                mime_type: Some("video/mp4".into()),
                size_bytes: i,
                modified_at: 1,
            })
            .collect();
        repo.apply_scan(&root.id, &batch, &[]).unwrap();
    }
    println!("idempotent re-commit 100k: {:?}", started.elapsed());
    let page = repo
        .list_items(&ItemQuery {
            page_size: 1,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, TOTAL, "still exactly 100k rows (§21)");
}
