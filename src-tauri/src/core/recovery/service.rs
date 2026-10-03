//! Startup recovery service (M2 §9/§84): scan persisted tasks,
//! reconcile with the filesystem, persist corrected metadata and
//! register resumable tasks with the engine. IDEMPOTENT (M2 §32): a
//! second scan over recovered state is a no-op by construction (the
//! decisions converge because each scan re-reads the reconciled truth).

use super::decision::{decide, FilesystemFacts, RecoveryAction};
use crate::core::download::DownloadShared;
use crate::core::download::DownloadRequest;
use crate::error::TideError;
use crate::persistence::repositories::DownloadTaskRecord;
use std::path::Path;
use std::sync::Arc;

/// Outcome of reconciling one task (for logs/tests).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RecoveryOutcome {
    pub task_id: String,
    pub action: RecoveryAction,
    pub reason: &'static str,
    pub safe_resume_bytes: u64,
}

/// Filesystem facts for a record's artifacts.
fn filesystem_facts(record: &DownloadTaskRecord) -> FilesystemFacts {
    let destination_available = Path::new(&record.destination_dir)
        .is_dir()
        .then_some(())
        .map(|_| true)
        .unwrap_or(false);
    let part_bytes = std::fs::metadata(&record.part_path).ok().map(|m| m.len());
    let final_bytes = std::fs::metadata(&record.final_path).ok().map(|m| m.len());
    FilesystemFacts {
        part_bytes,
        final_bytes,
        destination_available,
    }
}

/// Reconciles ONE record against the filesystem and returns the outcome
/// plus (optionally) a corrected copy of the record to persist.
pub fn reconcile_one(
    record: &DownloadTaskRecord,
    repo: &crate::persistence::repositories::DownloadRepository,
) -> Result<RecoveryOutcome, TideError> {
    let facts = filesystem_facts(record);
    let decision = decide(record, facts);
    crate::diagnostics::info(
        "task_recovery_decision",
        &[
            ("task_id", &record.id),
            ("action", &format!("{:?}", decision.action)),
            ("reason", decision.reason.as_str()),
        ],
    );

    match decision.action {
        RecoveryAction::Keep => {}
        RecoveryAction::MarkResumable => {
            // Persisted state: Running is not a durable fact (M2 §8) —
            // interrupted/active tasks become `ready` (resumable, idle);
            // Paused stays Paused (M2 §68: user intent is preserved).
            let state = match record.state.as_str() {
                "running" | "created" => "ready".to_string(),
                other => other.to_string(),
            };
            if state != record.state || decision.safe_resume_bytes != record.downloaded_bytes {
                repo.update_state(
                    &record.id,
                    &state,
                    decision.safe_resume_bytes,
                    None,
                    Some(decision.reason.as_str()),
                    None,
                )?;
            }
        }
        RecoveryAction::Finalize => {
            // Crash C: the part holds the complete representation. Verify
            // the configured checksum (if any) BEFORE renaming — startup
            // recovery finalization must respect the same rules as the
            // engine (M2 §60).
            let checksum_ok = match (&record.checksum_algorithm, &record.checksum_expected) {
                (Some(_), Some(expected)) => {
                    let mut writer = crate::core::download::checksum::ChecksumWriter::new();
                    let mut file = std::fs::File::open(&record.part_path)?;
                    use std::io::Read;
                    let mut buf = vec![0u8; 256 * 1024];
                    loop {
                        match file.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => writer.update(&buf[..n]),
                            Err(err) => {
                                let err = crate::core::download::file_sink::classify_io_error(
                                    &err,
                                    "recovery checksum read",
                                );
                                repo.update_state(
                                    &record.id,
                                    "failed",
                                    record.downloaded_bytes,
                                    Some((err.code(), &err.message())),
                                    Some("recovery_finalize_failed"),
                                    None,
                                )?;
                                return Ok(RecoveryOutcome {
                                    task_id: record.id.clone(),
                                    action: crate::core::recovery::decision::RecoveryAction::MarkFailed,
                                    reason: "recovery_finalize_failed",
                                    safe_resume_bytes: decision.safe_resume_bytes,
                                });
                            }
                        }
                    }
                    crate::core::download::checksum::matches(expected, &writer.finalize_hex())
                }
                _ => true, // no checksum configured: size equality is the check
            };
            if checksum_ok {
                std::fs::rename(&record.part_path, &record.final_path)?;
                repo.update_state(
                    &record.id,
                    "completed",
                    decision.safe_resume_bytes,
                    None,
                    Some(decision.reason.as_str()),
                    Some(crate::core::time::now_unix_ms()),
                )?;
            } else {
                // Checksum mismatch on a complete part: keep the part for
                // diagnosis, fail honestly (mirrors the engine, M1 §30).
                repo.update_state(
                    &record.id,
                    "failed",
                    decision.safe_resume_bytes,
                    Some(("DOWNLOAD_CHECKSUM_MISMATCH", "checksum mismatch")),
                    Some("recovery_checksum_mismatch"),
                    None,
                )?;
                return Ok(RecoveryOutcome {
                    task_id: record.id.clone(),
                    action: crate::core::recovery::decision::RecoveryAction::MarkFailed,
                    reason: "checksum_mismatch",
                    safe_resume_bytes: decision.safe_resume_bytes,
                });
            }
        }
        RecoveryAction::MarkFailed => {
            repo.update_state(
                &record.id,
                "failed",
                0,
                Some(("DOWNLOAD_ARTIFACT_MISSING", "final artifact missing after completion")),
                Some(decision.reason.as_str()),
                None,
            )?;
        }
        RecoveryAction::BlockUntilUserAction => {
            repo.update_state(
                &record.id,
                &record.state,
                record.downloaded_bytes,
                Some(("DOWNLOAD_DESTINATION_UNAVAILABLE", "destination directory unavailable")),
                Some(decision.reason.as_str()),
                None,
            )?;
        }
    }

    Ok(RecoveryOutcome {
        task_id: record.id.clone(),
        action: decision.action,
        reason: decision.reason.as_str(),
        safe_resume_bytes: decision.safe_resume_bytes,
    })
}

/// Full startup scan: load every persisted record, reconcile, persist
/// the corrected metadata, and register a RESTORED runtime entry with
/// the engine for each task (idle; the user resumes explicitly).
pub fn recover_all(
    repo: &crate::persistence::repositories::DownloadRepository,
    engine: &crate::core::download::DownloadEngine,
) -> Result<Vec<RecoveryOutcome>, TideError> {
    crate::diagnostics::info("task_recovery_started", &[]);
    let records = repo.list()?;
    let mut outcomes = Vec::new();
    for record in records {
        let outcome = reconcile_one(&record, repo)?;
        // Reload the (possibly corrected) record and register the runtime
        // entry so `list`/`get`/`resume` work without recreating tasks.
        if let Some(corrected) = repo.get(&record.id)? {
            let shared = restored_shared(&corrected, engine.repo().clone())?;
            engine.register_restored(shared);
        }
        crate::diagnostics::info(
            "task_recovery_reconciled",
            &[
                ("task_id", &outcome.task_id),
                ("action", &format!("{:?}", outcome.action)),
            ],
        );
        outcomes.push(outcome);
    }
    crate::diagnostics::info(
        "task_recovery_finished",
        &[("tasks", &outcomes.len().to_string())],
    );
    Ok(outcomes)
}

/// Builds the idle runtime entry for a restored record.
fn restored_shared(
    record: &DownloadTaskRecord,
    repo: Arc<crate::persistence::repositories::DownloadRepository>,
) -> Result<Arc<DownloadShared>, TideError> {
    // Security re-validation (M2 §36): the database is NOT a security
    // boundary — a restored source URL goes through the same validation
    // as a freshly typed one. allow_private comes from the persisted
    // opt-in.
    let validated = crate::core::download::url::validate_download_url(
        &record.source_url,
        record.allow_private,
    )?;
    let policy = crate::core::download::file_sink::OverwritePolicy::parse(
        &record.overwrite_policy,
    )?;
    let checksum = record
        .checksum_algorithm
        .as_deref()
        .zip(record.checksum_expected.as_deref())
        .map(|(algo, hex)| {
            crate::core::download::checksum::ChecksumAlgorithm::parse(algo)
                .map(|algo| (algo, hex.to_string()))
        })
        .transpose()?;
    let request = DownloadRequest::new(validated, record.destination_dir.clone(), None, policy, checksum)?;
    // The request gets a NEW id inside `new`; restored tasks keep their
    // persisted identity (M2 §14: the id never changes) — overwrite it.
    let mut request = request;
    request.download_id = record.id.clone();

    let lifecycle = crate::core::download::DownloadLifecycle::from_state_str(&record.state);
    Ok(Arc::new(DownloadShared::from_record(
        request, repo, record, lifecycle,
    )))
}
