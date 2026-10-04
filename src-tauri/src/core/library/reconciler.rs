//! Library reconciler (M6(下) §117/§118).
//!
//! SQLite and the filesystem CANNOT commit atomically together — the
//! design goal is to DETECT, EXPLAIN and FIX the drift, never to
//! pretend it away. This module owns the four-case reconciliation:
//!
//! ```text
//! DB Available   + file exists            → Available
//! DB Available   + file missing           → Missing
//! DB Missing     + file exists again      → Available
//! DB metadata    + file metadata changed  → refresh (next full scan)
//! ```
//!
//! Deliberately NOT inside the scanner (§35/§118: scanner enumerates,
//! reconciler decides) and deliberately NOT smart (§119): a missing
//! file is never re-bound to a same-named file elsewhere — no
//! guessing, determinism first.

use super::model::AvailabilityStatus;
use super::repository::LibraryRepository;
use std::path::PathBuf;

/// Reconciles a batch of (item_id, canonical_path) rows against the
/// filesystem. Returns the number of rows whose availability changed.
pub fn reconcile_items_blocking(items: &[(String, PathBuf)], repo: &LibraryRepository) -> u64 {
    let mut changed = 0u64;
    for (id, canonical_path) in items {
        let status = match std::fs::metadata(canonical_path) {
            Ok(meta) if meta.is_file() => AvailabilityStatus::Available,
            Ok(_) => AvailabilityStatus::Inaccessible,
            Err(_) => AvailabilityStatus::Missing,
        };
        let Some(item) = repo.get_item(id).ok().flatten() else {
            continue;
        };
        if item.availability != status {
            let _ = repo.set_item_availability(id, status);
            changed += 1;
        }
    }
    changed
}
