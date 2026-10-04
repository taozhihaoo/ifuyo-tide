//! Library scanner (M6(上) §15/§16/§21/§68/§69).
//!
//! Enumerates a root, validates + classifies candidates, and commits
//! the whole scan as one repository transaction. The scanner executes
//! NO SQL itself (§32) and knows nothing about playback/downloads.
//!
//! Missing-verdict safety (§68/§69): only directories the scan really
//! enumerated contribute to Missing marking. Any directory whose
//! enumeration failed (permissions, reparse point, IO error) is
//! recorded as an UNCOVERED prefix, and items under it keep their
//! availability. If the ROOT itself cannot be read the scan fails and
//! the database is left completely untouched.

use super::classifier;
use super::model::MediaCategory;
use super::path;
use super::repository::{LibraryRepository, ScanCandidate};
use crate::error::TideError;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanStatus {
    Ok,
    Partial,
    Failed,
    /// §130: cancelled before commit — the database is untouched.
    Cancelled,
}

impl ScanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ScanStatus::Ok => "ok",
            ScanStatus::Partial => "partial",
            ScanStatus::Failed => "failed",
            ScanStatus::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanOutcome {
    pub status: ScanStatus,
    pub files_seen: u64,
    pub added: u64,
    pub missing: u64,
    pub errors: u64,
    pub duration_ms: u128,
}

/// M6(下) §109 failure-injection points (TEST-ONLY). Production always
/// passes `ScanFaults::default()`; the injected faults exercise
/// partial-failure and database-consistency paths that a real
/// filesystem cannot trigger deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScanFaults {
    /// One directory entry read fails (→ partial scan, uncovered).
    pub fail_metadata_read: bool,
    /// The scan aborts AFTER enumeration, BEFORE any database write
    /// (→ the DB must be completely untouched).
    pub fail_before_db_upsert: bool,
    /// The scan aborts AFTER a successful DB commit (→ data stays).
    pub fail_after_scan: bool,
}

/// Progress callback: `files_seen` so far (throttling is the caller's
/// job, §40).
pub fn scan_root_blocking(
    root_id: &str,
    root_path: &Path,
    repo: &LibraryRepository,
    mut progress: impl FnMut(u64),
) -> Result<ScanOutcome, TideError> {
    scan_root_blocking_with(
        root_id,
        root_path,
        repo,
        &mut progress,
        &std::sync::atomic::AtomicBool::new(false),
        ScanFaults::default(),
    )
}

/// §130: the full scan entry point. `cancelled` is checked between
/// directories and file batches — a cancelled scan commits NOTHING,
/// so the database stays exactly as it was.
pub fn scan_root_blocking_with(
    root_id: &str,
    root_path: &Path,
    repo: &LibraryRepository,
    progress: &mut dyn FnMut(u64),
    cancelled: &std::sync::atomic::AtomicBool,
    faults: ScanFaults,
) -> Result<ScanOutcome, TideError> {
    let started = Instant::now();
    let mut candidates: Vec<ScanCandidate> = Vec::new();
    let mut uncovered: Vec<PathBuf> = Vec::new();
    let mut errors = 0u64;
    let mut files_seen = 0u64;
    let mut injected_metadata_fault = false;

    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root_path.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        // §130: cooperative cancellation between directories.
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(ScanOutcome {
                status: ScanStatus::Cancelled,
                files_seen,
                added: 0,
                missing: 0,
                errors,
                duration_ms: started.elapsed().as_millis(),
            });
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                if dir == root_path {
                    // Root itself unreadable → abort WITHOUT any
                    // database write (§68: no scan coverage, no
                    // Missing verdicts).
                    return Err(TideError::Streaming(
                        crate::error::StreamingFailure::Resource(format!(
                            "library root could not be read ({})",
                            err_kind(&err)
                        )),
                    ));
                }
                errors += 1;
                uncovered.push(dir);
                continue;
            }
        };
        for entry in entries {
            let Ok(entry) = entry else {
                errors += 1;
                continue;
            };
            // §109: injected metadata-read failure (partial scan path).
            if faults.fail_metadata_read && !injected_metadata_fault {
                injected_metadata_fault = true;
                errors += 1;
                uncovered.push(entry.path());
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                errors += 1;
                continue;
            };
            if metadata.is_dir() {
                // §12: never traverse links out of the root. Detection:
                // modern Rust resolves junctions transparently (and may
                // drop the reparse attribute), so canonicalize and
                // compare — a junction resolves to a DIFFERENT path.
                let is_link = path::canonicalize(&entry.path())
                    .map(|resolved| !path::same_path(&resolved, &entry.path()))
                    .unwrap_or(true);
                if is_link {
                    errors += 1;
                    uncovered.push(entry.path());
                } else {
                    queue.push_back(entry.path());
                }
                continue;
            }
            if !metadata.is_file() {
                // Neither directory nor file: a reparse point (modern
                // Rust reports junctions this way — no dir/file bit).
                // Never traversed, never indexed (§12); recorded as
                // uncovered so previously indexed items inside stay.
                errors += 1;
                uncovered.push(entry.path());
                continue;
            }
            let file_path = entry.path();
            let Some(name) = file_path.file_name() else {
                continue;
            };
            let name = name.to_string_lossy().to_string();
            let extension = classifier::extension_of(&name);
            if let Some(ext) = &extension {
                if classifier::is_excluded_extension(ext) {
                    continue; // §16: never index .part/.tmp/...
                }
            }
            files_seen += 1;
            let relative = path::relative_path(root_path, &file_path);
            candidates.push(ScanCandidate {
                canonical_path: file_path,
                relative_path: relative,
                display_name: name,
                extension: extension.clone(),
                media_category: classifier::classify(extension.as_deref()),
                mime_type: classifier::mime_for(extension.as_deref()).map(str::to_string),
                size_bytes: metadata.len(),
                modified_at: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            });
            if files_seen.is_multiple_of(256) {
                progress(files_seen);
            }
        }
    }
    progress(files_seen);

    // §109: abort BEFORE any database write — the DB must stay exactly
    // as it was (failure-injection acceptance criterion).
    if faults.fail_before_db_upsert {
        return Err(TideError::Streaming(crate::error::StreamingFailure::Resource(
            "injected failure before database upsert".into(),
        )));
    }
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(ScanOutcome {
            status: ScanStatus::Cancelled,
            files_seen,
            added: 0,
            missing: 0,
            errors,
            duration_ms: started.elapsed().as_millis(),
        });
    }

    let (added, missing) = repo.apply_scan(root_id, &candidates, &uncovered)?;

    // §109: abort AFTER a successful commit — committed data stays.
    if faults.fail_after_scan {
        return Err(TideError::Streaming(crate::error::StreamingFailure::Resource(
            "injected failure after scan commit".into(),
        )));
    }
    let status = if errors > 0 {
        ScanStatus::Partial
    } else {
        ScanStatus::Ok
    };
    Ok(ScanOutcome {
        status,
        files_seen,
        added,
        missing,
        errors,
        duration_ms: started.elapsed().as_millis(),
    })
}

fn err_kind(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "not found",
        std::io::ErrorKind::PermissionDenied => "permission denied",
        _ => "io error",
    }
}

/// Light startup verification (§71/§73): existence + metadata only —
/// never hashing. Delegates to the reconciler (§118), which owns all
/// availability decisions.
pub fn verify_items_blocking(items: &[(String, PathBuf)], repo: &LibraryRepository) -> u64 {
    super::reconciler::reconcile_items_blocking(items, repo)
}

/// Category re-export for the service's import path (keeps the
/// classifier the single mapping point, §77).
pub fn classify_import(name: &str) -> (Option<String>, MediaCategory, Option<String>) {
    let extension = classifier::extension_of(name);
    let category = classifier::classify(extension.as_deref());
    let mime = classifier::mime_for(extension.as_deref()).map(str::to_string);
    (extension, category, mime)
}
