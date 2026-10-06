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

async fn reconcile_one(
    record: &TorrentTaskRecord,
    repo: &Arc<TorrentRepository>,
    engine: &TorrentEngine,
) -> Result<TorrentRecoveryOutcome, TideError> {
    // Metadata-copies are persisted for torrent_file sources; magnets
    // re-resolve from the stored URI.
    let metadata_bytes = match record.source_kind.as_str() {
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
            Some(std::fs::read(path)?)
        }
        "magnet" => None,
        other => {
            return Err(TideError::Internal(format!(
                "unknown torrent source kind: {other}"
            )));
        }
    };

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
    let readd = async {
        match (record.source_kind.as_str(), &metadata_bytes) {
            ("torrent_file", Some(bytes)) => {
                engine
                    .add_torrent_bytes(
                        bytes.clone(),
                        &record.storage_root,
                        None,
                        None,
                        true, // paused: never auto-resume on startup
                    )
                    .await
            }
            ("magnet", _) => {
                let uri = record
                    .magnet_uri
                    .clone()
                    .ok_or_else(|| TideError::Internal("magnet task without magnet URI".into()))?;
                engine
                    .add_magnet(uri, &record.storage_root, None, true)
                    .await
            }
            (kind, _) => Err(TideError::Internal(format!(
                "unknown torrent source kind: {kind}"
            ))),
        }
    };
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
