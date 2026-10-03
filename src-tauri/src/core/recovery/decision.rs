//! Recovery decision logic (M2 §10/§11/§57): pure, deterministic and
//! unit-testable. Inputs are the persisted record and the PHYSICAL
//! filesystem state; the output is one [`RecoveryDecision`].

use crate::persistence::repositories::DownloadTaskRecord;

/// Why the recovery layer classified a task this way (M2 §96).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    /// Process died while the task was active; the part survived.
    InterruptedRuntime,
    /// No part file: nothing to resume, start over.
    ArtifactMissing,
    /// Part is shorter than the DB checkpoint — trust the file.
    ArtifactShorterThanCheckpoint,
    /// Part is longer than the DB checkpoint — extra bytes are usable
    /// (they were received and written); reconciliation adopts them.
    ArtifactLargerThanCheckpoint,
    /// Final artifact present where the record says it should be.
    FinalArtifactDetected,
    /// Part holds the complete representation (Crash C): verify + finalize.
    CompleteArtifactDetected,
    /// Destination directory vanished/unreadable.
    DestinationUnavailable,
    /// DB row failed validation (bad state string, negative bytes…).
    DatabaseRecordInvalid,
}

impl RecoveryReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RecoveryReason::InterruptedRuntime => "interrupted_runtime",
            RecoveryReason::ArtifactMissing => "artifact_missing",
            RecoveryReason::ArtifactShorterThanCheckpoint => "artifact_shorter_than_checkpoint",
            RecoveryReason::ArtifactLargerThanCheckpoint => "artifact_larger_than_checkpoint",
            RecoveryReason::FinalArtifactDetected => "final_artifact_detected",
            RecoveryReason::CompleteArtifactDetected => "complete_artifact_detected",
            RecoveryReason::DestinationUnavailable => "destination_unavailable",
            RecoveryReason::DatabaseRecordInvalid => "database_record_invalid",
        }
    }
}

/// What startup recovery will DO with a task (M2 §10: pure logic, the
/// transient counterpart of the persisted state — no new lifecycle
/// states are invented).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Task stays as persisted; nothing to reconcile.
    Keep,
    /// Part reconciled; task is resumable (idle until user resumes).
    MarkResumable,
    /// The part holds the complete resource: verify + finalize on startup.
    Finalize,
    /// Completed task whose final artifact vanished: fail honestly.
    MarkFailed,
    /// Destination directory unavailable: keep the record, block resume.
    BlockUntilUserAction,
}

/// The decision for one persisted task.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RecoveryDecision {
    pub action: RecoveryAction,
    pub reason: RecoveryReason,
    /// The reconciled byte position recovery adopted (file truth).
    pub safe_resume_bytes: u64,
}

/// Physical filesystem facts about one task's artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FilesystemFacts {
    /// `.part` exists with this many bytes (None = missing).
    pub part_bytes: Option<u64>,
    /// Final file exists with this many bytes (None = missing).
    pub final_bytes: Option<u64>,
    /// Destination directory exists and is readable.
    pub destination_available: bool,
}

/// Pure reconciliation (M2 §57/§58): the record is a HINT, the file is
/// the truth. `total_bytes` (when known) enables the Crash-C
/// "part holds the complete resource" detection.
///
/// Rules (each mapped to the decision table in docs/M2-recovery.md):
///  1. destination unavailable → BlockUntilUserAction
///  2. persisted state Completed → final must exist → Finalize-check /
///     MarkFailed
///  3. final file exists for an active task → another task's artifact
///     or a previous finalize: keep the record, treat as Keep with the
///     final detected (engine overwrite policy governs the actual write)
///  4. part missing → downloaded = 0 → restart from zero (MarkResumable
///     with 0 bytes; the resource identity is unchanged)
///  5. part present → safe resume point = PHYSICAL part size (M2 §6/§59);
///     part == total (when known, total > 0) → CompleteArtifactDetected
///     → Finalize
pub fn decide(record: &DownloadTaskRecord, fs: FilesystemFacts) -> RecoveryDecision {
    use RecoveryAction::*;
    use RecoveryReason::*;

    if !fs.destination_available {
        return RecoveryDecision {
            action: BlockUntilUserAction,
            reason: DestinationUnavailable,
            safe_resume_bytes: record.downloaded_bytes,
        };
    }

    match record.state.as_str() {
        "completed" => {
            return if fs.final_bytes.is_some() {
                RecoveryDecision {
                    action: Keep,
                    reason: FinalArtifactDetected,
                    safe_resume_bytes: record.downloaded_bytes,
                }
            } else {
                // Crash D: DB says completed but the artifact is gone —
                // never keep claiming Completed (M2 §25).
                RecoveryDecision {
                    action: MarkFailed,
                    reason: ArtifactMissing,
                    safe_resume_bytes: 0,
                }
            };
        }
        "cancelled" | "failed" => {
            // User intent / settled failure: never silently auto-resume
            // (M2 §68). The user can Retry explicitly.
            return RecoveryDecision {
                action: Keep,
                reason: InterruptedRuntime,
                safe_resume_bytes: record.downloaded_bytes,
            };
        }
        _ => {}
    }

    // Active task (created/ready/running/paused): reconcile the part.
    match fs.part_bytes {
        None => RecoveryDecision {
            action: MarkResumable,
            reason: ArtifactMissing,
            safe_resume_bytes: 0,
        },
        Some(part_bytes) => {
            // Crash C detection: the part holds the whole representation.
            if record.total_bytes == Some(part_bytes) && part_bytes > 0 {
                return RecoveryDecision {
                    action: Finalize,
                    reason: CompleteArtifactDetected,
                    safe_resume_bytes: part_bytes,
                };
            }
            let reason = if part_bytes > record.downloaded_bytes {
                ArtifactLargerThanCheckpoint
            } else if part_bytes < record.downloaded_bytes {
                // DB ahead of file — the dangerous direction (M2 §6):
                // trust the FILE, never resume past physical data.
                ArtifactShorterThanCheckpoint
            } else {
                InterruptedRuntime
            };
            RecoveryDecision {
                action: MarkResumable,
                reason,
                safe_resume_bytes: part_bytes,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn record(state: &str, downloaded: u64, total: Option<u64>) -> DownloadTaskRecord {
        DownloadTaskRecord {
            id: "t".into(),
            source_url: "http://example.com/f".into(),
            allow_private: false,
            destination_dir: PathBuf::from("C:\\d"),
            filename: "f.bin".into(),
            part_path: PathBuf::from("C:\\d\\f.bin.part"),
            final_path: PathBuf::from("C:\\d\\f.bin"),
            overwrite_policy: "auto_rename".into(),
            state: state.into(),
            total_bytes: total,
            downloaded_bytes: downloaded,
            content_type: None,
            etag: Some("\"v\"".into()),
            last_modified: None,
            checksum_algorithm: None,
            checksum_expected: None,
            retry_count: 0,
            last_error_code: None,
            last_error_message: None,
            recovery_reason: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            completed_at: None,
        }
    }

    #[test]
    fn destination_unavailable_blocks() {
        let r = record("paused", 500, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: Some(500),
                final_bytes: None,
                destination_available: false,
            },
        );
        assert_eq!(d.action, RecoveryAction::BlockUntilUserAction);
    }

    #[test]
    fn completed_with_artifact_keeps() {
        let r = record("completed", 1000, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: None,
                final_bytes: Some(1000),
                destination_available: true,
            },
        );
        assert_eq!(d.action, RecoveryAction::Keep);
        assert_eq!(d.reason, RecoveryReason::FinalArtifactDetected);
    }

    #[test]
    fn completed_without_artifact_fails_honestly() {
        // Crash D
        let r = record("completed", 1000, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: None,
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.action, RecoveryAction::MarkFailed);
        assert_eq!(d.reason, RecoveryReason::ArtifactMissing);
    }

    #[test]
    fn cancelled_never_auto_resumes() {
        // Invariant 3
        let r = record("cancelled", 400, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: Some(400),
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.action, RecoveryAction::Keep);
        assert_eq!(d.safe_resume_bytes, 400);
    }

    #[test]
    fn interrupted_runtime_resumes_from_file_truth() {
        // Crash A: DB behind file → adopt the file size.
        let r = record("running", 400, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: Some(420),
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.action, RecoveryAction::MarkResumable);
        assert_eq!(d.reason, RecoveryReason::ArtifactLargerThanCheckpoint);
        assert_eq!(d.safe_resume_bytes, 420);
    }

    #[test]
    fn db_ahead_of_file_trusts_file() {
        // M1 §79/§79-adjacent corruption case: DB says 500, file 420 —
        // resume point MUST be 420.
        let r = record("running", 500, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: Some(420),
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.reason, RecoveryReason::ArtifactShorterThanCheckpoint);
        assert_eq!(d.safe_resume_bytes, 420);
    }

    #[test]
    fn complete_part_finalizes() {
        // Crash C
        let r = record("running", 1000, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: Some(1000),
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.action, RecoveryAction::Finalize);
        assert_eq!(d.reason, RecoveryReason::CompleteArtifactDetected);
    }

    #[test]
    fn missing_part_restarts_from_zero() {
        let r = record("paused", 700, Some(1000));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: None,
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.action, RecoveryAction::MarkResumable);
        assert_eq!(d.reason, RecoveryReason::ArtifactMissing);
        assert_eq!(d.safe_resume_bytes, 0);
    }

    #[test]
    fn u64_extremes_do_not_panic() {
        let r = record("running", u64::MAX, Some(u64::MAX));
        let d = decide(
            &r,
            FilesystemFacts {
                part_bytes: Some(u64::MAX),
                final_bytes: None,
                destination_available: true,
            },
        );
        assert_eq!(d.safe_resume_bytes, u64::MAX);
    }
}
