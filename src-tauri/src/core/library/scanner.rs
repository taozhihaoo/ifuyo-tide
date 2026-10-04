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
}

impl ScanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ScanStatus::Ok => "ok",
            ScanStatus::Partial => "partial",
            ScanStatus::Failed => "failed",
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

/// Progress callback: `files_seen` so far (throttling is the caller's
/// job, §40).
pub fn scan_root_blocking(
    root_id: &str,
    root_path: &Path,
    repo: &LibraryRepository,
    mut progress: impl FnMut(u64),
) -> Result<ScanOutcome, TideError> {
    let started = Instant::now();
    let mut candidates: Vec<ScanCandidate> = Vec::new();
    let mut uncovered: Vec<PathBuf> = Vec::new();
    let mut errors = 0u64;
    let mut files_seen = 0u64;

    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root_path.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                if dir == root_path {
                    // Root itself unreadable → abort WITHOUT any
                    // database write (§68: no scan coverage, no
                    // Missing verdicts).
                    return Err(TideError::Streaming(crate::error::StreamingFailure::Resource(
                        format!("library root could not be read ({})", err_kind(&err)),
                    )));
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
            let Ok(metadata) = entry.metadata() else {
                errors += 1;
                continue;
            };
            if metadata.is_dir() {
                // §12: never traverse reparse points — a junction could
                // lead anywhere on the system. Recorded as uncovered so
                // items inside it (if previously indexed) are safe.
                if path::is_reparse_point(&metadata) {
                    errors += 1;
                    uncovered.push(entry.path());
                } else {
                    queue.push_back(entry.path());
                }
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let file_path = entry.path();
            let Some(name) = file_path.file_name() else { continue };
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
            if files_seen % 256 == 0 {
                progress(files_seen);
            }
        }
    }
    progress(files_seen);

    let (added, missing) = repo.apply_scan(root_id, &candidates, &uncovered)?;
    let status = if errors > 0 { ScanStatus::Partial } else { ScanStatus::Ok };
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
/// never hashing. Items whose file disappeared become Missing;
/// reappearing files become Available again.
pub fn verify_items_blocking(
    items: &[(String, PathBuf)],
    repo: &LibraryRepository,
) -> u64 {
    let mut changed = 0u64;
    for (id, canonical_path) in items {
        let status = match std::fs::metadata(canonical_path) {
            Ok(meta) if meta.is_file() => super::model::AvailabilityStatus::Available,
            Ok(_) => super::model::AvailabilityStatus::Inaccessible,
            Err(_) => super::model::AvailabilityStatus::Missing,
        };
        let item = match repo.get_item(id) {
            Ok(Some(item)) => item,
            _ => continue,
        };
        if item.availability != status {
            let _ = repo.set_item_availability(id, status);
            changed += 1;
        }
    }
    changed
}

/// Category re-export for the service's import path (keeps the
/// classifier the single mapping point, §77).
pub fn classify_import(name: &str) -> (Option<String>, MediaCategory, Option<String>) {
    let extension = classifier::extension_of(name);
    let category = classifier::classify(extension.as_deref());
    let mime = classifier::mime_for(extension.as_deref()).map(str::to_string);
    (extension, category, mime)
}
