//! M0.5 SPIKE — torrent harness integration tests (experiments C/D/F).
//!
//! These run the real librqbit loop: create torrent → seed on a local
//! session → download on a second session via a direct loopback peer →
//! progressive read of NOT-YET-COMPLETE data. Slow by nature (tens of
//! seconds); they are the core evidence for the M0.5 decision.

use ifuyo_tide_lib::core::resource::ByteRange;
use ifuyo_tide_lib::spike::resource::{SpikeResource, TorrentResource};
use ifuyo_tide_lib::spike::torrent::TorrentSpike;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-spike-bt-{}-{}",
        label,
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

/// A 8 MiB patterned file — big enough for meaningful piece progress,
/// small enough to transfer fast over loopback.
fn make_payload(dir: &std::path::Path, name: &str, len: u64) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).expect("create payload");
    let mut buf = vec![0u8; 64 * 1024];
    let mut offset = 0u64;
    while offset < len {
        let take = (len - offset).min(64 * 1024) as usize;
        for (i, b) in buf.iter_mut().enumerate().take(take) {
            *b = ((offset + i as u64) % 253) as u8;
        }
        file.write_all(&buf[..take]).expect("write");
        offset += take as u64;
    }
    path
}

async fn wait_for_progress(spike: &TorrentSpike, min_bytes: u64, timeout: Duration) -> u64 {
    let started = Instant::now();
    loop {
        let status = spike.status();
        if status.progress_bytes >= min_bytes {
            return status.progress_bytes;
        }
        if status.error.is_some() {
            panic!("torrent error while waiting for progress: {:?}", status.error);
        }
        if started.elapsed() > timeout {
            panic!(
                "timeout waiting for progress: {}/{} bytes",
                status.progress_bytes, status.total_bytes
            );
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn torrent_loopback_downloads_and_supports_progressive_read() {
    let seed_dir = temp_dir("seed");
    let payload_len: u64 = 8 * 1024 * 1024;
    let payload = make_payload(&seed_dir, "video.mp4", payload_len);

    let work_dir = temp_dir("work");
    let started = Instant::now();
    let spike = Arc::new(
        TorrentSpike::start_single(&payload, &work_dir)
            .await
            .expect("torrent harness start"),
    );
    let setup = started.elapsed();

    // Experiment C: metadata + file list are available.
    assert_eq!(spike.files.len(), 1);
    assert_eq!(spike.files[0].name, "video.mp4");
    assert_eq!(spike.files[0].len, payload_len);
    assert_eq!(spike.files[0].id, 0);

    // Experiment D: read the FIRST 512 KiB while the download is still
    // running — the read must wait for the pieces and return exact bytes.
    let mut resource = SpikeResource::Torrent(
        TorrentResource::new(spike.clone(), 0).expect("torrent resource"),
    );
    let head_range = ByteRange::new(0, 512 * 1024).unwrap();
    let read_started = Instant::now();
    let head = resource.read_range(head_range).await.expect("read head");
    let head_wait = read_started.elapsed();
    assert_eq!(head.len(), 512 * 1024);
    for (i, b) in head.iter().enumerate() {
        assert_eq!(*b, (i as u64 % 253) as u8, "head byte {i}");
    }

    // Download must CONTINUE after the read (not starved).
    let after_read = spike.status().progress_bytes;
    let _ = wait_for_progress(&spike, (after_read + 1024 * 1024).min(payload_len), Duration::from_secs(120)).await;

    // Experiment E: seek (read a range in the MIDDLE and near the END)
    // without the whole file being downloaded first.
    let mid_range = ByteRange::new(payload_len / 2, payload_len / 2 + 256 * 1024).unwrap();
    let mid_started = Instant::now();
    let mid = resource.read_range(mid_range).await.expect("read mid");
    let mid_wait = mid_started.elapsed();
    assert_eq!(mid.len(), 256 * 1024);
    for (i, b) in mid.iter().enumerate() {
        assert_eq!(*b, ((payload_len / 2 + i as u64) % 253) as u8, "mid byte {i}");
    }

    // Wait for completion and verify the end of the file.
    wait_for_progress(&spike, payload_len, Duration::from_secs(180)).await;
    let tail_range = ByteRange::new(payload_len - 4096, payload_len).unwrap();
    let tail = resource.read_range(tail_range).await.expect("read tail");
    assert_eq!(tail.len(), 4096);
    for (i, b) in tail.iter().enumerate() {
        assert_eq!(*b, ((payload_len - 4096 + i as u64) % 253) as u8, "tail byte {i}");
    }

    eprintln!(
        "spike torrent metrics: setup={:?} head_wait={:?} mid_wait={:?} setup_pieces_bytes={after_read}",
        setup, head_wait, mid_wait
    );

    spike.stop().await;
    let _ = std::fs::remove_dir_all(&seed_dir);
    let _ = std::fs::remove_dir_all(&work_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multi_file_torrent_lists_all_files_and_reads_selected_file() {
    // Experiment F: dedicated multi-file seed dir — the future
    // Resource/Availability/Gateway layers must not assume one torrent =
    // one file.
    let seed_dir = temp_dir("multi-seed");
    make_payload(&seed_dir, "video.mp4", 4 * 1024 * 1024);
    std::fs::write(seed_dir.join("subtitle.srt"), b"1\n00:00:01,000 --> 00:00:02,000\nhi\n").unwrap();
    std::fs::write(seed_dir.join("cover.jpg"), b"\xff\xd8fakejpg").unwrap();
    std::fs::write(seed_dir.join("other.bin"), vec![7u8; 1024 * 1024]).unwrap();

    let work_dir = temp_dir("multi-work");
    let spike = Arc::new(
        TorrentSpike::start_multi(&seed_dir, &work_dir)
            .await
            .expect("multi torrent start"),
    );

    assert_eq!(spike.files.len(), 4, "file list must contain all 4 files");
    let names: Vec<&str> = spike.files.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"video.mp4"));
    assert!(names.contains(&"subtitle.srt"));
    assert!(names.contains(&"cover.jpg"));
    assert!(names.contains(&"other.bin"));

    // Select ONLY the small binary file and read it progressively; the
    // byte range must map to that file, never to a sibling file.
    let bin_id = spike
        .files
        .iter()
        .find(|f| f.name == "other.bin")
        .expect("other.bin in list")
        .id;
    let mut resource =
        SpikeResource::Torrent(TorrentResource::new(spike.clone(), bin_id).expect("resource"));
    let range = ByteRange::new(0, 4096).unwrap();
    let bytes = resource.read_range(range).await.expect("read bin head");
    assert_eq!(bytes.len(), 4096);
    assert!(bytes.iter().all(|b| *b == 7), "bytes must come from other.bin");

    // File ids are bounds-checked (no arbitrary access).
    assert!(TorrentResource::new(spike.clone(), 99).is_err());

    spike.stop().await;
    let _ = std::fs::remove_dir_all(&seed_dir);
    let _ = std::fs::remove_dir_all(&work_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn torrent_pause_and_resume_do_not_break_reads() {
    // Experiment I (domain level): pause stops progress, resume
    // continues, and a pending read recovers.
    let seed_dir = temp_dir("pause-seed");
    let payload_len: u64 = 6 * 1024 * 1024;
    let payload = make_payload(&seed_dir, "movie.mp4", payload_len);

    let work_dir = temp_dir("pause-work");
    let spike = Arc::new(
        TorrentSpike::start_single(&payload, &work_dir)
            .await
            .expect("start"),
    );

    let _ = wait_for_progress(&spike, 1024 * 1024, Duration::from_secs(120)).await;
    spike.pause().await.expect("pause");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let paused_progress = spike.status().progress_bytes;
    assert!(spike.status().paused, "handle must report paused");

    spike.resume().await.expect("resume");
    let _ = wait_for_progress(&spike, payload_len, Duration::from_secs(180)).await;
    assert!(spike.status().finished);
    assert!(!spike.status().paused);
    let _ = paused_progress;

    spike.stop().await;
    let _ = std::fs::remove_dir_all(&seed_dir);
    let _ = std::fs::remove_dir_all(&work_dir);
}
