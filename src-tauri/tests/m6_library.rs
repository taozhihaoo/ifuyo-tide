//! M6(上) library integration tests: scan idempotency, partial-scan
//! Missing safety, reparse/junction refusal, download/torrent import,
//! remove-vs-delete semantics, query paging, and library-item playback
//! through the M4 gateway (§80/§82/§83).

use ifuyo_tide_lib::core::library::model::{
    AvailabilityStatus, ItemQuery, MediaCategory, OriginType,
};
use ifuyo_tide_lib::core::library::path;
use ifuyo_tide_lib::core::library::repository::LibraryRepository;
use ifuyo_tide_lib::core::library::service::LibraryService;
use ifuyo_tide_lib::core::streaming::demand::DemandScheduler;
use ifuyo_tide_lib::core::streaming::playback::PlaybackManager;
use ifuyo_tide_lib::core::streaming::resource::StreamingResource;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-m6-{}-{}",
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
    // Absolute-offset pattern (byte k = k % 251) so ranged reads can
    // be verified at any offset.
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
// scan: idempotency, classification, artifact exclusion (§16/§21)
// ---------------------------------------------------------------------

#[test]
fn scan_is_idempotent_and_classifies() {
    let root_dir = temp_dir("scan");
    let service = service();
    let root = service.add_root(&root_dir).expect("add root");
    write_file(&root_dir.join("Movie.mp4"), 4096);
    write_file(&root_dir.join("Nested").join("Song.flac"), 2048);
    write_file(&root_dir.join("notes.txt"), 128);
    write_file(&root_dir.join("Movie.mp4.part"), 999);
    write_file(&root_dir.join("temp.tmp"), 100);

    let first = service.scan_root_blocking(&root.id, |_| {}).unwrap();
    assert_eq!(first.status, ifuyo_tide_lib::core::library::ScanStatus::Ok);
    // .part/.tmp excluded (§16): Movie.mp4, Nested/Song.flac, notes.txt
    assert_eq!(first.files_seen, 3);
    assert_eq!(first.added, 3);
    assert_eq!(first.missing, 0);

    // §21: scan again — nothing added, nothing missing, no dups.
    let second = service.scan_root_blocking(&root.id, |_| {}).unwrap();
    assert_eq!(second.added, 0, "idempotent scan must not re-add");
    assert_eq!(second.missing, 0);

    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 3);
    let video: Vec<_> = page
        .items
        .iter()
        .filter(|i| i.media_category == MediaCategory::Video)
        .collect();
    assert_eq!(video.len(), 1);
    assert_eq!(video[0].display_name, "Movie.mp4");
    let _ = root;
}

// ---------------------------------------------------------------------
// missing + restore (§22/§23): no row deletion, state flips
// ---------------------------------------------------------------------

#[test]
fn scan_marks_missing_and_restores() {
    let root_dir = temp_dir("missing");
    let service = service();
    let root = service.add_root(&root_dir).unwrap();
    let gone = root_dir.join("Gone.mp4");
    let stays = root_dir.join("Stays.mp4");
    write_file(&gone, 1024);
    write_file(&stays, 1024);
    service.scan_root_blocking(&root.id, |_| {}).unwrap();

    std::fs::remove_file(&gone).unwrap();
    let outcome = service.scan_root_blocking(&root.id, |_| {}).unwrap();
    assert_eq!(outcome.missing, 1);

    let page = service
        .list_items(&ItemQuery {
            availability: Some(AvailabilityStatus::Missing),
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].display_name, "Gone.mp4");

    // Restore → Available again.
    write_file(&gone, 1024);
    service.scan_root_blocking(&root.id, |_| {}).unwrap();
    let page = service
        .list_items(&ItemQuery {
            availability: Some(AvailabilityStatus::Available),
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 2);
}

// ---------------------------------------------------------------------
// §68/§69: uncovered prefixes must not produce Missing verdicts
// ---------------------------------------------------------------------

#[test]
fn uncovered_directory_is_never_marked_missing() {
    // ONE shared repository: the service and the raw apply_scan calls
    // must see the same database (the root row must exist where the
    // items are written — FK enforced).
    let repo = Arc::new(LibraryRepository::open_in_memory().unwrap());
    let root_dir = temp_dir("uncovered");
    let root = LibraryService::new(repo.clone())
        .add_root(&root_dir)
        .unwrap();
    let covered = root_dir.join("covered.mp4");
    let hidden_dir = root_dir.join("private");
    write_file(&covered, 1024);
    write_file(&hidden_dir.join("hidden.mp4"), 1024);
    let canonical_covered = path::canonicalize(&covered).unwrap();
    let canonical_hidden = path::canonicalize(&hidden_dir.join("hidden.mp4")).unwrap();

    // First import both items via the repo (simulating a full scan).
    repo.apply_scan(
        &root.id,
        &[
            candidate(&canonical_covered, &root_dir),
            candidate(&canonical_hidden, &root_dir),
        ],
        &[],
    )
    .unwrap();

    // A scan that could NOT read `private/`: hidden.mp4 keeps its
    // availability; covered.mp4 (not seen) goes Missing.
    let (added, missing) = repo
        .apply_scan(
            &root.id,
            &[candidate(&canonical_covered, &root_dir)],
            &[hidden_dir.clone()],
        )
        .unwrap();
    assert_eq!(added, 0);
    // §69: covered.mp4 was seen (not missing); hidden.mp4 is under an
    // UNCOVERED prefix — it must NOT be declared missing either.
    assert_eq!(missing, 0);
    let item = repo
        .get_item_by_path(&canonical_hidden)
        .expect("hidden item exists")
        .expect("hidden item row");
    assert_eq!(item.availability, AvailabilityStatus::Available);
    let covered_item = repo
        .get_item_by_path(&canonical_covered)
        .expect("covered item exists")
        .expect("covered item row");
    assert_eq!(
        covered_item.availability,
        AvailabilityStatus::Available,
        "covered.mp4 was seen by this scan — it stays available"
    );

    // A FULL scan (no uncovered prefixes) may now declare it Missing.
    let (_, missing) = repo
        .apply_scan(&root.id, &[candidate(&canonical_covered, &root_dir)], &[])
        .unwrap();
    assert_eq!(missing, 1, "hidden.mp4 becomes missing under full coverage");
}

fn candidate(path: &Path, root: &Path) -> ifuyo_tide_lib::core::library::repository::ScanCandidate {
    ifuyo_tide_lib::core::library::repository::ScanCandidate {
        canonical_path: path.to_path_buf(),
        relative_path: path::relative_path(root, path),
        display_name: path.file_name().unwrap().to_string_lossy().to_string(),
        extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        media_category: MediaCategory::Video,
        mime_type: Some("video/mp4".into()),
        size_bytes: 1024,
        modified_at: 0,
    }
}

// ---------------------------------------------------------------------
// §12: junctions/reparse points are never traversed
// ---------------------------------------------------------------------

#[test]
fn junctions_are_not_traversed() {
    let root_dir = temp_dir("junction-root");
    let outside = temp_dir("junction-outside");
    write_file(&outside.join("secret.mp4"), 4096);

    let link = root_dir.join("link");
    // NOTE: single slashes — std::process::Command does NOT go through
    // MSYS argument conversion (the // doubling is only needed for
    // commands invoked from a Git Bash shell).
    let output = std::process::Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            &link.to_string_lossy(),
            &outside.to_string_lossy(),
        ])
        .output();
    let Ok(output) = output else { return };
    if !output.status.success() {
        return; // environment without junction support: skip
    }

    let service = service();
    let root = service.add_root(&root_dir).unwrap();
    let outcome = service.scan_root_blocking(&root.id, |_| {}).unwrap();
    // The junction directory itself counts as an error/uncovered — its
    // contents (secret.mp4) must NOT be indexed.
    assert!(outcome.errors >= 1, "junction must be recorded");
    let page = service
        .list_items(&ItemQuery {
            search: Some("secret".into()),
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 0, "junction contents must never be indexed");
}

// ---------------------------------------------------------------------
// §13: root validation — no drive roots, no system dirs
// ---------------------------------------------------------------------

#[test]
fn root_validation_rejects_system_locations() {
    let service = service();
    assert!(service.add_root(Path::new("C:\\")).is_err());
    assert!(service
        .add_root(Path::new("Q:\\definitely-not-a-drive"))
        .is_err());
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    assert!(service.add_root(Path::new(&system_root)).is_err());
}

// ---------------------------------------------------------------------
// §26: download import; §20: checksum reused, never recomputed
// ---------------------------------------------------------------------

#[test]
fn download_import_is_idempotent_and_keeps_origin() {
    let service = service();
    let dest = temp_dir("downloads");
    let final_path = dest.join("Ubuntu.iso");
    write_file(&final_path, 8192);

    let first = service
        .import_download(&final_path, &dest, "task-1", Some("abc123"))
        .unwrap()
        .expect("imported");
    assert_eq!(first.origin, OriginType::Download);
    assert_eq!(first.task_id.as_deref(), Some("task-1"));
    assert_eq!(first.checksum.as_deref(), Some("abc123"));
    // The destination dir became a library root (§14 seed).
    assert_eq!(service.list_roots().unwrap().len(), 1);

    // Import again (repeat completion event) — still one item.
    let again = service
        .import_download(&final_path, &dest, "task-1", Some("abc123"))
        .unwrap();
    assert!(again.is_some());
    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 1, "import must be idempotent (§21)");
}

// ---------------------------------------------------------------------
// §24/§25: remove (index only) vs delete (file + row), with validation
// ---------------------------------------------------------------------

#[test]
fn remove_keeps_file_delete_removes_it() {
    let service = service();
    let dest = temp_dir("remove-delete");
    let file = dest.join("a.mp4");
    write_file(&file, 2048);
    let item = service
        .import_download(&file, &dest, "task-1", None)
        .unwrap()
        .unwrap();

    // Remove from library: file survives.
    assert!(service.remove_item(&item.id).unwrap());
    assert!(file.exists());
    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 0);

    // Re-import, then DELETE from disk: file AND row are gone.
    let item = service
        .import_download(&file, &dest, "task-1", None)
        .unwrap()
        .unwrap();
    service.delete_item_from_disk(&item.id).unwrap();
    assert!(!file.exists(), "delete from disk removes the file");
    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 0, "index row removed with the file");
}

#[test]
fn delete_rejects_paths_outside_the_root() {
    let repo = Arc::new(LibraryRepository::open_in_memory().unwrap());
    let service = LibraryService::new(repo.clone());
    let root_dir = temp_dir("containment");
    let root = service.add_root(&root_dir).unwrap();
    let inside = root_dir.join("real.mp4");
    write_file(&inside, 1024);
    service
        .import_download(&inside, &root_dir, "task-1", None)
        .unwrap()
        .unwrap();

    // Forge a row whose canonical_path points OUTSIDE the root (a
    // tampered/legacy row): play and delete must both refuse.
    let outside = temp_dir("outside");
    let outside_file = outside.join("secret.mp4");
    write_file(&outside_file, 1024);
    let forged = ifuyo_tide_lib::core::library::model::LibraryItem {
        id: ifuyo_tide_lib::core::ids::new_id(),
        root_id: root.id.clone(),
        canonical_path: path::canonicalize(&outside_file).unwrap(),
        relative_path: "../secret.mp4".into(),
        display_name: "secret.mp4".into(),
        extension: Some("mp4".into()),
        media_category: MediaCategory::Video,
        mime_type: Some("video/mp4".into()),
        size_bytes: 1024,
        modified_at: 0,
        availability: AvailabilityStatus::Available,
        origin: OriginType::Unknown,
        task_id: None,
        checksum: None,
        last_verified_at: 0,
        created_at: 0,
        updated_at: 0,
    };
    repo.insert_imported_item(&forged).unwrap();

    let err = service.play_source(&forged.id).unwrap_err();
    assert_eq!(err.code(), "SECURITY", "play must refuse out-of-root rows");
    let err = service.delete_item_from_disk(&forged.id).unwrap_err();
    assert_eq!(
        err.code(),
        "SECURITY",
        "delete must refuse out-of-root rows"
    );
    assert!(outside_file.exists(), "the outside file must be untouched");
}

// ---------------------------------------------------------------------
// §27: torrent multi-file import (mode A)
// ---------------------------------------------------------------------

#[test]
fn torrent_import_one_item_per_file() {
    let service = service();
    let storage = temp_dir("torrent-storage");
    let movie = storage.join("Movie.mp4");
    let poster = storage.join("Poster.jpg");
    write_file(&movie, 4096);
    write_file(&poster, 512);

    service
        .import_torrent_file(&storage, &movie, "torrent-1")
        .unwrap()
        .expect("movie imported");
    service
        .import_torrent_file(&storage, &poster, "torrent-1")
        .unwrap()
        .expect("poster imported");

    let page = service
        .list_items(&ItemQuery {
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 2, "mode A: one item per file");
    assert!(page
        .items
        .iter()
        .all(|i| i.task_id.as_deref() == Some("torrent-1")));
    assert!(page
        .items
        .iter()
        .any(|i| i.media_category == MediaCategory::Video));
    assert!(page
        .items
        .iter()
        .any(|i| i.media_category == MediaCategory::Image));
}

// ---------------------------------------------------------------------
// §41–§45: query paging/filter/sort in SQL
// ---------------------------------------------------------------------

#[test]
fn queries_page_filter_and_sort() {
    let service = service();
    let dest = temp_dir("query");
    for i in 0..30u32 {
        let name = format!("item-{i:02}.mp4");
        let file = dest.join(&name);
        write_file(&file, 1024 * (i as u64 + 1));
        service
            .import_download(&file, &dest, &format!("task-{i}"), None)
            .unwrap();
    }
    let page0 = service
        .list_items(&ItemQuery {
            sort: Some("name".into()),
            ascending: true,
            page: 0,
            page_size: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page0.total, 30);
    assert_eq!(page0.items.len(), 10);
    assert_eq!(page0.items[0].display_name, "item-00.mp4");

    let page2 = service
        .list_items(&ItemQuery {
            sort: Some("name".into()),
            ascending: true,
            page: 2,
            page_size: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page2.items[0].display_name, "item-20.mp4");

    let search = service
        .list_items(&ItemQuery {
            search: Some("item-2".into()),
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(search.total, 10, "LIKE search over display_name");

    let by_size = service
        .list_items(&ItemQuery {
            sort: Some("size".into()),
            ascending: false,
            page_size: 1,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_size.items[0].display_name, "item-29.mp4");

    let audio = service
        .list_items(&ItemQuery {
            category: Some(MediaCategory::Audio),
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(audio.total, 0, "category filter is backend-side");
}

// ---------------------------------------------------------------------
// §46/§80/§82/§83: library item → Resource → gateway → byte-exact 206
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn library_item_plays_through_the_gateway() {
    let service = service();
    let dest = temp_dir("play");
    let file = dest.join("video.mp4");
    write_file(&file, 256 * 1024);
    let item = service
        .import_download(&file, &dest, "task-1", None)
        .unwrap()
        .unwrap();

    // The playback command path: item id → validated canonical path.
    let (canonical, size, name) = service.play_source(&item.id).unwrap();
    assert_eq!(name, "video.mp4");
    assert_eq!(size, 256 * 1024);

    let playback = PlaybackManager::new(Arc::new(DemandScheduler::new()))
        .await
        .unwrap();
    let session = playback
        .open(
            StreamingResource::File {
                path: canonical,
                len: size,
            },
            "video/mp4".into(),
            Some(item.id.clone()),
        )
        .unwrap();

    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", playback.gateway_port()))
        .await
        .unwrap();
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let request = format!(
        "GET /stream/{} HTTP/1.1\r\nHost: 127.0.0.1:9\r\nRange: bytes=1000-1099\r\nConnection: close\r\n\r\n",
        session.session_id
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw);
    assert!(
        text.starts_with("HTTP/1.1 206"),
        "library playback must be ranged"
    );
    let body = &raw[raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4..];
    assert_eq!(body.len(), 100);
    for (i, b) in body.iter().enumerate() {
        let expected = ((1000usize + i) % 251) as u8;
        assert_eq!(*b, expected, "byte-exact at offset {}", 1000 + i);
    }
    playback.close(&session.session_id);
}
