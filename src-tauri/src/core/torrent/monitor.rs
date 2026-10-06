//! Torrent completion monitor (M6(上) §27 enabling fix).
//!
//! AUDIT FINDING (M6 §1/§2): before M6 the backend had NO completion
//! detection for torrents — the DB state stayed ready/downloading
//! forever and recovery could not classify finished torrents. This
//! monitor closes that gap: every few seconds it reconciles task rows
//! against live engine stats, backfills magnet metadata once the DHT
//! delivers it, marks finished tasks completed, and imports their
//! selected files into the library (mode A, §27 — one item per file).
//!
//! M12(1.0.1) fixes found in real-use validation:
//! - a paused task keeps its paused STATE (progress writes used to
//!   stomp it back to running);
//! - the phase written is honest: metadata_pending until the metadata
//!   is actually resolved;
//! - magnet metadata (display name, total size, file list) is now
//!   persisted when it resolves — previously a magnet row froze at
//!   its creation-time shape forever (Files(0), info-hash title).

use crate::core::library::LibraryService;
use crate::core::torrent::TorrentEngine;
use crate::persistence::repositories::{TorrentFileRecord, TorrentRepository};
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

/// One reconciliation pass. Public so integration tests can drive a
/// single deterministic tick instead of racing the 5-second loop.
pub async fn tick(
    engine: &TorrentEngine,
    repo: &Arc<TorrentRepository>,
    library: &Arc<LibraryService>,
) -> Result<(), crate::error::TideError> {
    for mut record in repo.list_tasks()? {
        let Some(info_hash) = record.info_hash.clone() else {
            continue; // metadata still pending
        };
        if matches!(record.state.as_str(), "completed" | "cancelled" | "removed") {
            continue;
        }
        let Some(handle) = engine.handle_for(&info_hash) else {
            continue; // not in the engine (paused-but-recovered etc.)
        };
        let resolved = engine.metadata_resolved(&handle);

        // Whether the user ever made an explicit file selection
        // (selected_count is Some only after creation-with-metadata or
        // a set_torrent_file_selection call).
        let user_selected = record.selected_count.is_some();

        // Magnet backfill: once the DHT delivers the metadata, persist
        // the real name/total/file list (previously never done).
        if resolved && record.total_bytes.is_none() {
            match engine.metadata_of(&handle) {
                Ok(metadata) => {
                    repo.set_magnet_metadata_resolved(
                        &record.id,
                        &metadata.name,
                        metadata.total_size,
                        metadata.files.len() as u32,
                    )?;
                    let rows: Vec<TorrentFileRecord> = metadata
                        .files
                        .iter()
                        .map(|f| TorrentFileRecord {
                            file_index: f.index as u32,
                            info_hash: info_hash.clone(),
                            relative_path: f.relative_path.clone(),
                            size: f.size,
                            selected: true,
                        })
                        .collect();
                    repo.replace_files(&record.id, &info_hash, &rows)?;
                    // Refresh the loop-local snapshot: the progress write
                    // below uses these fields and would otherwise clobber
                    // the just-persisted values with stale ones.
                    record.display_name = metadata.name;
                    record.total_bytes = Some(metadata.total_size);
                    record.selected_count = Some(metadata.files.len() as u32);
                    record.phase = "downloading".into();
                    if !user_selected {
                        // M3 §23: the user never chose what to download —
                        // hold the torrent instead of grabbing every file
                        // by default. They pick files in the UI, then
                        // resume.
                        engine.pause(&handle).await?;
                        repo.update_task(
                            &record.id,
                            "paused",
                            "downloading",
                            Some(&info_hash),
                            record.total_bytes,
                            record.downloaded_bytes,
                            record.uploaded_bytes,
                            record.selected_count,
                            None,
                            None,
                            None,
                        )?;
                        record.state = "paused".into();
                        crate::diagnostics::info(
                            "torrent_monitor.paused_for_selection",
                            &[("task_id", record.id.as_str())],
                        );
                        continue;
                    }
                }
                Err(err) => {
                    crate::diagnostics::warn(
                        "torrent_monitor.metadata_backfill_failed",
                        &[("code", err.code())],
                    );
                    continue;
                }
            }
        }

        let stats = engine.stats(&handle);
        if !stats.finished {
            // Write ONLY on real byte progress. State-diff writes are
            // forbidden here: recovery re-adds the engine PAUSED and
            // marks the row ready/resumable — "correcting" the row to
            // running with zero bytes flowing used to fake a Running
            // badge over a paused engine (1.0.3 real-use finding).
            // Paused is a user decision and must survive ticks too.
            if stats.progress_bytes != record.downloaded_bytes {
                let state = if record.state == "paused" {
                    "paused"
                } else {
                    "running"
                };
                let phase = if resolved {
                    "downloading"
                } else {
                    "metadata_pending"
                };
                repo.update_task(
                    &record.id,
                    state,
                    phase,
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
    }
    Ok(())
}
