//! Torrent completion monitor (M6(上) §27 enabling fix).
//!
//! AUDIT FINDING (M6 §1/§2): before M6 the backend had NO completion
//! detection for torrents — the DB state stayed ready/downloading
//! forever and recovery could not classify finished torrents. This
//! monitor closes that gap: every few seconds it reconciles task rows
//! against live engine stats, marks finished tasks completed, and
//! imports their selected files into the library (mode A, §27 — one
//! item per file).

use crate::core::library::LibraryService;
use crate::core::torrent::TorrentEngine;
use crate::persistence::repositories::TorrentRepository;
use std::sync::Arc;
use std::time::Duration;

pub fn spawn(engine: TorrentEngine, repo: Arc<TorrentRepository>, library: Arc<LibraryService>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if let Err(err) = tick(&engine, &repo, &library).await {
                crate::diagnostics::warn("torrent_monitor.error", &[("code", err.code())]);
            }
        }
    });
}

async fn tick(
    engine: &TorrentEngine,
    repo: &Arc<TorrentRepository>,
    library: &Arc<LibraryService>,
) -> Result<(), crate::error::TideError> {
    for record in repo.list_tasks()? {
        let Some(info_hash) = record.info_hash.clone() else {
            continue; // metadata still pending
        };
        if matches!(record.state.as_str(), "completed" | "cancelled" | "removed") {
            continue;
        }
        let Some(handle) = engine.handle_for(&info_hash) else {
            continue; // not in the engine (paused-but-recovered etc.)
        };
        let stats = engine.stats(&handle);
        if !stats.finished {
            // Keep the row's progress roughly current for the UI.
            if stats.progress_bytes != record.downloaded_bytes {
                repo.update_task(
                    &record.id,
                    "running",
                    "downloading",
                    Some(&info_hash),
                    record.total_bytes,
                    stats.progress_bytes,
                    record.uploaded_bytes,
                    record.selected_count,
                    None,
                    None,
                    None,
                )?;
            }
            continue;
        }

        // Finished: persist the terminal state (previously never done!)
        // and import selected files into the library.
        repo.update_task(
            &record.id,
            "completed",
            "completed",
            Some(&info_hash),
            record.total_bytes,
            record.total_bytes.unwrap_or(stats.progress_bytes),
            record.uploaded_bytes,
            record.selected_count,
            None,
            None,
            Some(crate::core::time::now_unix_ms()),
        )?;
        crate::diagnostics::info("torrent_monitor.completed", &[]);

        let total_bytes = record.total_bytes;
        for file in repo.list_files(&record.id)? {
            if !file.selected {
                continue;
            }
            let absolute = record.storage_root.join(&file.relative_path);
            if let Ok(item) =
                library.import_torrent_file(&record.storage_root, &absolute, &record.id)
            {
                let _ = item;
            }
        }
        let _ = total_bytes;
    }
    Ok(())
}
