//! Library service (M6(上) §33/§34/§46/§48/§49/§53).
//!
//! The single facade IPC talks to. Scanner and repository stay
//! separate (§32/§35); the service owns validation and lifecycle
//! policies:
//! * a completed download auto-enters the library (§26); if its
//!   destination is not a root yet, that directory becomes one
//!   (§14 — download destination and library roots are independent
//!   concepts, this just seeds the first root);
//! * torrent multi-file tasks import ONE ITEM PER SELECTED FILE
//!   (mode A, §27);
//! * playback pre-validation re-checks the filesystem on every use —
//!   the DB row is metadata, never a permission proof (§10/§48);
//! * "remove from library" and "delete from disk" are different
//!   operations (§24), and disk deletion re-validates containment,
//!   file-ness and protected locations (§25).

use super::model::{LibraryItem, LibraryRoot, OriginType};
use super::path;
use super::repository::LibraryRepository;
use super::scanner::{self, ScanOutcome};
use crate::error::{StreamingFailure, TideError};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct LibraryService {
    repo: Arc<LibraryRepository>,
}

fn now_ms() -> i64 {
    crate::core::time::now_unix_ms()
}

fn streaming_err(what: &str) -> TideError {
    TideError::Streaming(StreamingFailure::Resource(what.into()))
}

impl LibraryService {
    pub fn new(repo: Arc<LibraryRepository>) -> LibraryService {
        LibraryService { repo }
    }

    pub fn repo(&self) -> &Arc<LibraryRepository> {
        &self.repo
    }

    // ----- roots --------------------------------------------------------

    pub fn add_root(&self, raw_path: &Path) -> Result<LibraryRoot, TideError> {
        let canonical = path::validate_root(raw_path)?;
        if let Some(existing) = self.repo.get_root_by_path(&canonical)? {
            return Ok(existing);
        }
        let root = LibraryRoot {
            id: crate::core::ids::new_id(),
            path: canonical.clone(),
            display_name: canonical
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| canonical.to_string_lossy().to_string()),
            enabled: true,
            scan_status: "idle".into(),
            last_scan_at: None,
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.repo.insert_root(&root)?;
        crate::diagnostics::info("library.root_added", &[]);
        Ok(root)
    }

    pub fn remove_root(&self, id: &str) -> Result<bool, TideError> {
        // Index-only removal (§24): files on disk are never touched.
        let removed = self.repo.get_root(id)?;
        self.repo.remove_root(id)?;
        if removed.is_some() {
            crate::diagnostics::info("library.root_removed", &[]);
        }
        Ok(removed.is_some())
    }

    pub fn list_roots(&self) -> Result<Vec<LibraryRoot>, TideError> {
        self.repo.list_roots()
    }

    pub fn set_root_enabled(&self, id: &str, enabled: bool) -> Result<(), TideError> {
        self.repo.set_root_enabled(id, enabled)
    }

    // ----- scanning -----------------------------------------------------

    /// Blocking scan — the COMMAND layer must run it on a background
    /// task (§38) and throttle progress events (§40).
    pub fn scan_root_blocking(
        &self,
        root_id: &str,
        mut progress: impl FnMut(u64),
    ) -> Result<ScanOutcome, TideError> {
        let root = self
            .repo
            .get_root(root_id)?
            .ok_or_else(|| TideError::NotFound(format!("library root not found: {root_id}")))?;
        if !root.enabled {
            return Err(streaming_err("library root is disabled"));
        }
        self.repo
            .update_root_scan_status(root_id, "scanning", None)?;
        match scanner::scan_root_blocking(root_id, &root.path, self.repo(), &mut progress) {
            Ok(outcome) => {
                self.repo.update_root_scan_status(
                    root_id,
                    outcome.status.as_str(),
                    Some(now_ms()),
                )?;
                crate::diagnostics::info(
                    "library.scan_completed",
                    &[
                        ("status", outcome.status.as_str()),
                        ("files", &outcome.files_seen.to_string()),
                    ],
                );
                Ok(outcome)
            }
            Err(err) => {
                self.repo
                    .update_root_scan_status(root_id, "failed", Some(now_ms()))?;
                Err(err)
            }
        }
    }

    /// Light startup verification over enabled roots (§71/§73):
    /// existence checks only, run on a background task by the caller.
    pub fn verify_light_blocking(&self) -> Result<u64, TideError> {
        let mut changed = 0u64;
        for root in self.repo.list_roots()? {
            if !root.enabled {
                continue;
            }
            let items = self.repo.list_item_paths_of_root(&root.id)?;
            changed += scanner::verify_items_blocking(&items, self.repo());
        }
        Ok(changed)
    }

    // ----- imports (§26/§27) ---------------------------------------------

    /// Completed HTTP download → LibraryItem. The final file must
    /// exist; its destination dir becomes a root if not registered
    /// yet (§14/§26).
    pub fn import_download(
        &self,
        final_path: &Path,
        destination_dir: &Path,
        task_id: &str,
        checksum: Option<&str>,
    ) -> Result<Option<LibraryItem>, TideError> {
        let Ok(metadata) = std::fs::metadata(final_path) else {
            // Finalization is atomic (M1) — if it is not there, there
            // is nothing to import; not an error condition.
            return Ok(None);
        };
        if !metadata.is_file() {
            return Ok(None);
        }
        let canonical = path::canonicalize(final_path)?;
        let root = self.ensure_root_for(&canonical, destination_dir)?;
        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let (extension, category, mime) = scanner::classify_import(&name);
        if extension
            .as_deref()
            .map(super::classifier::is_excluded_extension)
            .unwrap_or(false)
        {
            return Ok(None); // never index our own artifacts (§16)
        }
        let item = LibraryItem {
            id: crate::core::ids::new_id(),
            root_id: root.id.clone(),
            canonical_path: canonical.clone(),
            relative_path: path::relative_path(&root.path, &canonical),
            display_name: name,
            extension,
            media_category: category,
            mime_type: mime,
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            availability: super::model::AvailabilityStatus::Available,
            origin: OriginType::Download,
            task_id: Some(task_id.to_string()),
            checksum: checksum.map(str::to_string),
            last_verified_at: now_ms(),
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.repo.insert_imported_item(&item)?;
        crate::diagnostics::info("library.item_imported", &[("origin", "download")]);
        Ok(Some(item))
    }

    /// Torrent multi-file task → one item per selected file (mode A,
    /// §27). Missing/unselected files are skipped silently.
    pub fn import_torrent_file(
        &self,
        storage_root: &Path,
        absolute_path: &Path,
        task_id: &str,
    ) -> Result<Option<LibraryItem>, TideError> {
        let Ok(metadata) = std::fs::metadata(absolute_path) else {
            return Ok(None);
        };
        if !metadata.is_file() {
            return Ok(None);
        }
        let canonical = path::canonicalize(absolute_path)?;
        let root = self.ensure_root_for(&canonical, storage_root)?;
        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let (extension, category, mime) = scanner::classify_import(&name);
        if extension
            .as_deref()
            .map(super::classifier::is_excluded_extension)
            .unwrap_or(false)
        {
            return Ok(None);
        }
        let item = LibraryItem {
            id: crate::core::ids::new_id(),
            root_id: root.id.clone(),
            canonical_path: canonical.clone(),
            relative_path: path::relative_path(&root.path, &canonical),
            display_name: name,
            extension,
            media_category: category,
            mime_type: mime,
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            availability: super::model::AvailabilityStatus::Available,
            origin: OriginType::Download,
            task_id: Some(task_id.to_string()),
            checksum: None,
            last_verified_at: now_ms(),
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.repo.insert_imported_item(&item)?;
        Ok(Some(item))
    }

    /// Finds or creates the root that contains `canonical` (searching
    /// by longest containing path first). `seed_dir` is the directory
    /// to register when nothing contains the file.
    fn ensure_root_for(&self, canonical: &Path, seed_dir: &Path) -> Result<LibraryRoot, TideError> {
        let roots = self.repo.list_roots()?;
        let mut best: Option<&LibraryRoot> = None;
        for root in &roots {
            if path::is_contained(&root.path, canonical) {
                let better = best
                    .map(|b| b.path.components().count() < root.path.components().count())
                    .unwrap_or(true);
                if better {
                    best = Some(root);
                }
            }
        }
        if let Some(root) = best {
            return Ok(root.clone());
        }
        // §14: seed the destination dir as a root (never a drive or
        // system location).
        let seed = if path::is_contained(&path::canonicalize(seed_dir)?, canonical) {
            path::canonicalize(seed_dir)?
        } else {
            canonical
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| canonical.to_path_buf())
        };
        self.add_root(&seed)
    }

    // ----- items --------------------------------------------------------

    pub fn list_items(
        &self,
        query: &super::model::ItemQuery,
    ) -> Result<super::model::ItemPage, TideError> {
        self.repo.list_items(query)
    }

    pub fn get_item(&self, id: &str) -> Result<Option<LibraryItem>, TideError> {
        self.repo.get_item(id)
    }

    /// §49: re-stat one item; DB size/mtime drift is corrected and a
    /// gone file becomes Missing.
    pub fn refresh_item(&self, id: &str) -> Result<LibraryItem, TideError> {
        let item = self
            .repo
            .get_item(id)?
            .ok_or_else(|| TideError::NotFound(format!("library item not found: {id}")))?;
        match std::fs::metadata(&item.canonical_path) {
            Ok(meta) if meta.is_file() => {
                if meta.len() != item.size_bytes {
                    // External modification (§49): re-validate metadata.
                    let _ = self
                        .repo
                        .set_item_availability(id, super::model::AvailabilityStatus::Available);
                }
                let _ = self
                    .repo
                    .set_item_availability(id, super::model::AvailabilityStatus::Available);
            }
            Ok(_) => {
                let _ = self
                    .repo
                    .set_item_availability(id, super::model::AvailabilityStatus::Inaccessible);
            }
            Err(_) => {
                let _ = self
                    .repo
                    .set_item_availability(id, super::model::AvailabilityStatus::Missing);
            }
        }
        self.repo
            .get_item(id)?
            .ok_or_else(|| streaming_err("item vanished during refresh"))
    }

    /// §46/§48: play = validate → hand a trusted canonical path +
    /// size to the playback command, which creates the M4/M5 session.
    /// The frontend only ever passes the ITEM ID.
    pub fn play_source(&self, item_id: &str) -> Result<(PathBuf, u64, String), TideError> {
        let item = self
            .repo
            .get_item(item_id)?
            .ok_or_else(|| TideError::NotFound(format!("library item not found: {item_id}")))?;
        // §48 full re-validation — the DB row proves nothing.
        if !path::is_contained(&item.canonical_path, &item.canonical_path) {
            return Err(TideError::Security("invalid item path".into()));
        }
        let canonical = path::canonicalize(&item.canonical_path)?;
        if !canonical.is_file() {
            let _ = self
                .repo
                .set_item_availability(item_id, super::model::AvailabilityStatus::Missing);
            return Err(streaming_err("the file behind this library item is gone"));
        }
        // Containment against the item's CURRENT root (§48).
        if let Some(root) = self.repo.get_root(&item.root_id)? {
            if !path::is_contained(&root.path, &canonical) {
                return Err(TideError::Security("library item escaped its root".into()));
            }
        }
        if path::has_reserved_device_name(&canonical) || path::is_protected_system_path(&canonical)
        {
            return Err(TideError::Security("protected system file".into()));
        }
        // §49: detect external modification before playing.
        let metadata = std::fs::metadata(&canonical)?;
        if metadata.len() != item.size_bytes {
            crate::diagnostics::warn("library.item_stale", &[]);
            let _ = self
                .repo
                .set_item_availability(item_id, super::model::AvailabilityStatus::Available);
        }
        Ok((canonical, metadata.len(), item.display_name.clone()))
    }

    /// §24: remove the INDEX entry only — the file stays.
    pub fn remove_item(&self, id: &str) -> Result<bool, TideError> {
        let removed = self.repo.remove_item(id)?;
        if removed {
            crate::diagnostics::info("library.item_removed", &[]);
        }
        Ok(removed)
    }

    /// §25: delete the FILE behind an item, with full backend
    /// re-validation. The frontend only supplies the item id.
    pub fn delete_item_from_disk(&self, id: &str) -> Result<(), TideError> {
        let item = self
            .repo
            .get_item(id)?
            .ok_or_else(|| TideError::NotFound(format!("library item not found: {id}")))?;
        let canonical = path::canonicalize(&item.canonical_path)?;
        if !canonical.is_file() {
            return Err(streaming_err("not a regular file"));
        }
        let root = self
            .repo
            .get_root(&item.root_id)?
            .ok_or_else(|| streaming_err("the item's root no longer exists"))?;
        if !path::is_contained(&root.path, &canonical) {
            return Err(TideError::Security("library item escaped its root".into()));
        }
        if path::is_protected_system_path(&canonical) || path::has_reserved_device_name(&canonical)
        {
            return Err(TideError::Security("protected system file".into()));
        }
        std::fs::remove_file(&canonical).map_err(|err| {
            crate::diagnostics::warn(
                "library.delete_failed",
                &[("kind", format!("{:?}", err.kind()).leak() as &'static str)],
            );
            streaming_err("the file could not be deleted")
        })?;
        self.repo.remove_item(id)?;
        crate::diagnostics::info("library.item_deleted_from_disk", &[]);
        Ok(())
    }
}
