//! Torrent startup recovery (M3 §15/§16): re-add persisted torrent
//! tasks to the librqbit session and reconcile Tide state with the
//! engine + filesystem. Idempotent: the session dedupes by info_hash
//! (AlreadyManaged), and re-adding over existing files re-verifies
//! pieces (M3 §22).

use super::engine::TorrentEngine;
use crate::error::TideError;
use crate::persistence::repositories::{TorrentRepository, TorrentTaskRecord};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TorrentRecoveryOutcome {
    pub task_id: String,
    pub action: &'static str,
    pub reason: &'static str,
}

/// Re-registers every persisted torrent task with the engine.
///
/// User intent is preserved exactly like M2 recovery (M3 §16):
/// paused/failed/cancelled tasks are re-added PAUSED (idle) — the user
/// resumes explicitly; ready tasks are re-added paused too but marked
/// `ready` (resumable) — the engine keeps the storage state so resume
/// is fast. Idempotent: re-running re-adds to the SAME session which
/// dedupes by info_hash (AlreadyManaged → no duplicate download).
pub async fn recover_torrents(
    repo: &Arc<TorrentRepository>,
    engine: &TorrentEngine,
) -> Result<Vec<TorrentRecoveryOutcome>, TideError> {
    crate::diagnostics::info("torrent_recovery_started", &[]);
    let records = repo.list_tasks()?;
    let mut outcomes = Vec::new();
    for record in &records {
        outcomes.push(reconcile_one(record, repo, engine).await?);
    }
    crate::diagnostics::info(
        "torrent_recovery_finished",
        &[("tasks", &outcomes.len().to_string())],
    );
    Ok(outcomes)
}

/// Metadata-copies are persisted for torrent_file sources; magnets
/// re-resolve from the stored URI. Shared by startup recovery and the
/// on-demand re-add in resume (1.0.6).
pub fn load_metadata_bytes(record: &TorrentTaskRecord) -> Result<Option<Vec<u8>>, TideError> {
    match record.source_kind.as_str() {
        "torrent_file" => {
            let path = record
                .metadata_path
                .as_ref()
                .filter(|p| p.exists())
                .ok_or_else(|| {
                    crate::diagnostics::warn(
                        "torrent_recovery_metadata_missing",
                        &[("task_id", &record.id)],
                    );
                    TideError::Download(
                        crate::core::download::DownloadFailure::InvalidTorrentPath {
                            path: format!("metadata copy missing for {}", record.id),
                        },
                    )
                })?;
            Ok(Some(std::fs::read(path)?))
        }
        "magnet" => Ok(None),
        other => Err(TideError::Internal(format!(
            "unknown torrent source kind: {other}"
        ))),
    }
}

/// The user's saved file selection as engine `only_files` indices.
/// None = download everything (no file rows yet, or all selected).
pub fn saved_selection(
    repo: &Arc<TorrentRepository>,
    record: &TorrentTaskRecord,
) -> Result<Option<Vec<usize>>, TideError> {
    let files = repo.list_files(&record.id)?;
    if files.is_empty() || files.iter().all(|f| f.selected) {
        return Ok(None);
    }
    Ok(Some(
        files
            .iter()
            .filter(|f| f.selected)
            .map(|f| f.file_index as usize)
            .collect(),
    ))
}

/// Re-adds a persisted task to the engine, restoring the saved file
/// selection. Shared by startup recovery and on-demand resume re-add.
pub async fn readd_to_engine(
    engine: &TorrentEngine,
    record: &TorrentTaskRecord,
    metadata_bytes: Option<Vec<u8>>,
    paused: bool,
    only_files: Option<Vec<usize>>,
) -> Result<super::engine::AddedTorrent, TideError> {
    match (record.source_kind.as_str(), &metadata_bytes) {
        ("torrent_file", Some(bytes)) => {
            engine
                .add_torrent_bytes(
                    bytes.clone(),
                    &record.storage_root,
                    only_files,
                    None,
                    paused,
                )
                .await
        }
        ("magnet", _) => {
            let uri = record
                .magnet_uri
                .clone()
                .ok_or_else(|| TideError::Internal("magnet task without magnet URI".into()))?;
            let added = engine
                .add_magnet(uri, &record.storage_root, None, paused)
                .await?;
            // Magnets accept no only_files at add time (the metadata may
            // still be resolving) — apply the saved selection after.
            if let Some(files) = &only_files {
                engine.set_only_files(&added.handle, files).await?;
            }
            Ok(added)
        }
        (kind, _) => Err(TideError::Internal(format!(
            "unknown torrent source kind: {kind}"
        ))),
    }
}

/// Deletes the task's downloaded/partial files from disk (by the saved
/// file rows, containment-checked against the storage root) and zeroes
/// the row's progress. Used by "reset partial data" when a re-add is
/// wedged by stale partial data (1.0.8): a fresh add over a clean
/// directory cannot hit the librqbit stale-data hang. Returns the
/// number of files removed.
pub fn reset_partial_files(
    record: &TorrentTaskRecord,
    repo: &Arc<TorrentRepository>,
) -> Result<u32, TideError> {
    use crate::core::library::path::{canonicalize, is_contained};
    use std::fs;

    let root = canonicalize(&record.storage_root)?;
    let files = repo.list_files(&record.id)?;
    let mut removed = 0u32;
    for file in &files {
        let candidate = record.storage_root.join(&file.relative_path);
        if !candidate.exists() {
            continue;
        }
        let canonical = canonicalize(&candidate)?;
        if !is_contained(&root, &canonical) {
            return Err(TideError::Security(format!(
                "torrent file path escapes the storage root: {}",
                file.relative_path
            )));
        }
        match fs::remove_file(&canonical) {
            Ok(()) => removed += 1,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(TideError::Internal(format!(
                    "failed to remove partial file {}: {err}",
                    file.relative_path
                )));
            }
        }
    }
    repo.update_task(
        &record.id,
        &record.state,
        &record.phase,
        record.info_hash.as_deref(),
        record.total_bytes,
        0,
        record.uploaded_bytes,
        record.selected_count,
        None,
        Some("partial_reset"),
        record.completed_at,
    )?;
    crate::diagnostics::info(
        "torrent.partial_data_reset",
        &[("task_id", &record.id), ("files", &removed.to_string())],
    );
    Ok(removed)
}

async fn reconcile_one(
    record: &TorrentTaskRecord,
    repo: &Arc<TorrentRepository>,
    engine: &TorrentEngine,
) -> Result<TorrentRecoveryOutcome, TideError> {
    let metadata_bytes = load_metadata_bytes(record)?;

    // Re-validate the metadata paths (M3 §12: never trust stored input).
    if let Some(bytes) = &metadata_bytes {
        let list = engine.list_only(bytes.clone()).await.inspect_err(|err| {
            crate::diagnostics::warn(
                "torrent_recovery_metadata_invalid",
                &[("task_id", &record.id), ("code", err.code())],
            );
        })?;
        let paths: Vec<String> = list
            .info
            .iter_file_details_ext()
            .map(|d| {
                d.details
                    .filename
                    .to_pathbuf()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        super::source::validate_torrent_paths(&paths)?;
    }

    // Terminal states are restored WITHOUT re-adding to the engine.
    let settled_error = record
        .error_code
        .clone()
        .map(|c| (c, record.error_message.clone().unwrap_or_default()));

    if matches!(record.state.as_str(), "completed" | "cancelled") {
        repo.update_task(
            &record.id,
            &record.state,
            &record.phase,
            record.info_hash.as_deref(),
            record.total_bytes,
            record.downloaded_bytes,
            record.uploaded_bytes,
            record.selected_count,
            settled_error
                .as_ref()
                .map(|(c, m)| (c.as_str(), m.as_str())),
            record.recovery_reason.as_deref(),
            record.completed_at,
        )?;
        return Ok(TorrentRecoveryOutcome {
            task_id: record.id.clone(),
            action: "kept",
            reason: "terminal_state",
        });
    }

    let active_error = record
        .error_code
        .clone()
        .map(|c| (c, record.error_message.clone().unwrap_or_default()));

    // Re-add PAUSED: startup recovery never auto-resumes (M3 §16/M2 §68).
    // Bounded: librqbit can stall indefinitely re-adding a torrent over
    // stale partial data (observed in real use, 1.0.3 investigation) —
    // one bad task must not wedge engine availability forever. On
    // timeout the row keeps its state and the user can retry manually.
    // Re-add restores the user's saved selection — previously the
    // engine came back with every file selected regardless of the
    // checkboxes (1.0.6 real-use finding).
    let only_files = saved_selection(repo, record)?;
    let readd = async { readd_to_engine(engine, record, metadata_bytes, true, only_files).await };
    let added = match tokio::time::timeout(std::time::Duration::from_secs(60), readd).await {
        Ok(result) => result?,
        Err(_) => {
            crate::diagnostics::warn("torrent_recovery_readd_timeout", &[("task_id", &record.id)]);
            repo.update_task(
                &record.id,
                &record.state,
                &record.phase,
                record.info_hash.as_deref(),
                record.total_bytes,
                record.downloaded_bytes,
                record.uploaded_bytes,
                record.selected_count,
                None,
                Some("readd_timeout"),
                record.completed_at,
            )?;
            return Ok(TorrentRecoveryOutcome {
                task_id: record.id.clone(),
                action: "skipped",
                reason: "readd_timeout",
            });
        }
    };

    // Persist the reconciled Tide state (info_hash may have just been
    // resolved for a magnet task). Ready = resumable-idle (M2 §69).
    let new_state = if record.state == "running" || record.state == "created" {
        "ready"
    } else {
        &record.state
    };
    let new_phase = if engine.metadata_resolved(&added.handle) {
        "downloading"
    } else {
        "metadata_pending"
    };
    repo.update_task(
        &record.id,
        new_state,
        new_phase,
        Some(&added.info_hash),
        record.total_bytes,
        record.downloaded_bytes,
        record.uploaded_bytes,
        record.selected_count,
        active_error.as_ref().map(|(c, m)| (c.as_str(), m.as_str())),
        record.recovery_reason.as_deref(),
        record.completed_at,
    )?;

    crate::diagnostics::info(
        "torrent_recovery_reconciled",
        &[
            ("task_id", &record.id),
            ("already_managed", &added.already_managed.to_string()),
            ("ready", new_state),
        ],
    );
    Ok(TorrentRecoveryOutcome {
        task_id: record.id.clone(),
        action: "re_added",
        reason: "restored_to_session",
    })
}
