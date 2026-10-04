//! Library IPC commands (M6(上) §60–§65).
//!
//! Typed requests, stable DTOs; the renderer passes IDs and validated
//! strings only — never trusted paths (§11/§59: every path is
//! canonicalized and containment-checked inside the service).
//! Scanning runs on a background task with throttled progress events
//! (§38/§40) and never blocks the UI thread.

use crate::app::state::AppState;
use crate::core::library::model::{
    AvailabilityStatus, ItemQuery, LibraryItem, LibraryRoot, MediaCategory, OriginType,
};
use crate::core::library::scanner::ScanOutcome;
use crate::error::TideError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRootDto {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub scan_status: String,
    pub last_scan_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemDto {
    pub id: String,
    pub root_id: String,
    pub display_name: String,
    pub relative_path: String,
    pub extension: Option<String>,
    pub category: String,
    pub mime_type: Option<String>,
    pub size_bytes: u64,
    pub modified_at: i64,
    pub availability: String,
    pub origin: String,
    pub task_id: Option<String>,
}

/// §61: the canonical internal path is NOT exposed to the frontend.
fn root_dto(root: &LibraryRoot) -> LibraryRootDto {
    LibraryRootDto {
        id: root.id.clone(),
        display_name: root.display_name.clone(),
        enabled: root.enabled,
        scan_status: root.scan_status.clone(),
        last_scan_at: root.last_scan_at,
    }
}

fn item_dto(item: &LibraryItem) -> LibraryItemDto {
    LibraryItemDto {
        id: item.id.clone(),
        root_id: item.root_id.clone(),
        display_name: item.display_name.clone(),
        relative_path: item.relative_path.clone(),
        extension: item.extension.clone(),
        category: item.media_category.as_str().to_string(),
        mime_type: item.mime_type.clone(),
        size_bytes: item.size_bytes,
        modified_at: item.modified_at,
        availability: item.availability.as_str().to_string(),
        origin: item.origin.as_str().to_string(),
        task_id: item.task_id.clone(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemQueryDto {
    pub search: Option<String>,
    pub category: Option<String>,
    pub availability: Option<String>,
    pub origin: Option<String>,
    pub sort: Option<String>,
    pub ascending: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

fn parse_query(dto: ItemQueryDto) -> Result<ItemQuery, TideError> {
    Ok(ItemQuery {
        search: dto.search.filter(|s| !s.trim().is_empty()),
        category: match dto.category.as_deref() {
            None | Some("all") => None,
            Some(raw) => Some(
                MediaCategory::parse(raw)
                    .ok_or_else(|| TideError::InvalidInput(format!("unknown category: {raw}")))?,
            ),
        },
        availability: match dto.availability.as_deref() {
            None | Some("all") => None,
            Some(raw) => {
                Some(AvailabilityStatus::parse(raw).ok_or_else(|| {
                    TideError::InvalidInput(format!("unknown availability: {raw}"))
                })?)
            }
        },
        origin: match dto.origin.as_deref() {
            None | Some("all") => None,
            Some(raw) => Some(
                OriginType::parse(raw)
                    .ok_or_else(|| TideError::InvalidInput(format!("unknown origin: {raw}")))?,
            ),
        },
        sort: dto.sort,
        ascending: dto.ascending.unwrap_or(true),
        page: dto.page.unwrap_or(0),
        page_size: dto.page_size.unwrap_or(50),
    })
}

#[tauri::command]
pub fn library_list_roots(state: State<'_, AppState>) -> Result<Vec<LibraryRootDto>, TideError> {
    Ok(state.library.list_roots()?.iter().map(root_dto).collect())
}

/// The path string comes from the directory picker — it is NEVER
/// trusted: the service canonicalizes and validates it (§11/§13).
#[tauri::command]
pub fn library_add_root(
    state: State<'_, AppState>,
    path: String,
) -> Result<LibraryRootDto, TideError> {
    let root = state.library.add_root(&PathBuf::from(path))?;
    Ok(root_dto(&root))
}

#[tauri::command]
pub fn library_remove_root(state: State<'_, AppState>, root_id: String) -> Result<bool, TideError> {
    state.library.remove_root(&root_id)
}

#[tauri::command]
pub fn library_set_root_enabled(
    state: State<'_, AppState>,
    root_id: String,
    enabled: bool,
) -> Result<(), TideError> {
    state.library.set_root_enabled(&root_id, enabled)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressDto {
    pub root_id: String,
    pub files_seen: u64,
    pub added: u64,
    pub missing: u64,
    pub errors: u64,
    /// running | ok | partial | failed
    pub status: String,
}

/// Starts a scan on a BACKGROUND task (§38) and returns immediately.
/// Progress events (`library://scan`) are throttled (§40); percentages
/// are never faked — the event carries raw counts (§39).
#[tauri::command]
pub fn library_scan(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    root_id: Option<String>,
) -> Result<(), TideError> {
    let roots = match root_id {
        Some(id) => vec![state
            .library
            .repo()
            .get_root(&id)?
            .ok_or_else(|| TideError::NotFound(format!("library root not found: {id}")))?],
        None => state
            .library
            .list_roots()?
            .into_iter()
            .filter(|r| r.enabled)
            .collect(),
    };
    // §131 single-flight: registering a BUSY root fails the whole
    // command with a structured BUSY error (the frontend stays usable).
    let mut flags = Vec::with_capacity(roots.len());
    for root in &roots {
        let flag = state.library.begin_scan(&root.id)?;
        flags.push((root.id.clone(), flag));
    }
    let library = state.library.clone();
    tauri::async_runtime::spawn(async move {
        for (root_id, flag) in flags {
            let throttle = Mutex::new((std::time::Instant::now(), 0u64));
            let root_id_for_progress = root_id.clone();
            let app_for_progress = app.clone();
            let mut progress = move |files_seen: u64| {
                let mut guard = throttle.lock().unwrap();
                let since = guard.0.elapsed();
                if since >= std::time::Duration::from_millis(300) || files_seen < guard.1 {
                    guard.0 = std::time::Instant::now();
                    guard.1 = files_seen;
                    let _ = app_for_progress.emit(
                        "library://scan",
                        ScanProgressDto {
                            root_id: root_id_for_progress.clone(),
                            files_seen,
                            added: 0,
                            missing: 0,
                            errors: 0,
                            status: "running".into(),
                        },
                    );
                }
            };
            let outcome: Result<ScanOutcome, TideError> = tauri::async_runtime::spawn_blocking({
                let library = library.clone();
                let root_id = root_id.clone();
                move || {
                    library.scan_root_blocking_with(
                        &root_id,
                        &flag,
                        Default::default(),
                        &mut progress,
                    )
                }
            })
            .await
            .unwrap_or_else(|_| Err(TideError::Internal("scan task failed".into())));
            library.end_scan(&root_id);
            let dto = match outcome {
                Ok(o) => ScanProgressDto {
                    root_id: root_id.clone(),
                    files_seen: o.files_seen,
                    added: o.added,
                    missing: o.missing,
                    errors: o.errors,
                    status: o.status.as_str().to_string(),
                },
                Err(_) => ScanProgressDto {
                    root_id: root_id.clone(),
                    files_seen: 0,
                    added: 0,
                    missing: 0,
                    errors: 1,
                    status: "failed".into(),
                },
            };
            let _ = app.emit("library://scan", dto);
        }
    });
    Ok(())
}

/// §130: requests cancellation of the root's running scan. The scan
/// stops between directories and commits NOTHING — the database stays
/// consistent and the root returns to idle.
#[tauri::command]
pub fn library_cancel_scan(state: State<'_, AppState>, root_id: String) -> bool {
    state.library.cancel_scan(&root_id)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPageDto {
    pub items: Vec<LibraryItemDto>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[tauri::command]
pub fn library_list_items(
    state: State<'_, AppState>,
    query: ItemQueryDto,
) -> Result<ItemPageDto, TideError> {
    let page = state.library.list_items(&parse_query(query)?)?;
    Ok(ItemPageDto {
        items: page.items.iter().map(item_dto).collect(),
        total: page.total,
        page: page.page,
        page_size: page.page_size,
    })
}

#[tauri::command]
pub fn library_get_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<Option<LibraryItemDto>, TideError> {
    Ok(state.library.get_item(&item_id)?.as_ref().map(item_dto))
}

/// §49: re-stat one item (external modification / deletion).
#[tauri::command]
pub fn library_refresh_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<LibraryItemDto, TideError> {
    Ok(item_dto(&state.library.refresh_item(&item_id)?))
}

/// §24: remove the INDEX entry only — the file stays on disk.
#[tauri::command]
pub fn library_remove_item(state: State<'_, AppState>, item_id: String) -> Result<bool, TideError> {
    state.library.remove_item(&item_id)
}

/// §25: delete the FILE behind the item — backend re-validates
/// containment/file-ness/protected locations; the frontend supplies
/// the item id only. Any playback session on this item is invalidated.
#[tauri::command]
pub fn library_delete_item(state: State<'_, AppState>, item_id: String) -> Result<(), TideError> {
    // M6(下) §136: an active playback session means the file is IN
    // USE — deletion is REFUSED (structured FILE_IN_USE), never done
    // silently behind a playing video.
    if state.playback.has_session_for_owner(&item_id) {
        return Err(TideError::Streaming(
            crate::error::StreamingFailure::FileInUse,
        ));
    }
    super::ipc_result(
        "library_delete_item",
        state.library.delete_item_from_disk(&item_id),
    )
}

/// §91: basic Open File / Open Folder via Explorer — backend-resolved
/// validated paths only. Spawning, not shell-injecting: the path is a
/// canonicalized value from the database, passed as ONE argument.
#[tauri::command]
pub async fn library_reveal(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    item_id: String,
    folder: bool,
) -> Result<(), TideError> {
    use tauri_plugin_opener::OpenerExt;
    let (canonical, _, _) = state.library.play_source(&item_id)?;
    let opener = app.opener();
    let _ = &state;
    if folder {
        let parent = canonical
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| canonical.display().to_string());
        opener.open_path(parent, None::<&str>).map_err(|_| {
            TideError::Streaming(crate::error::StreamingFailure::Resource(
                "could not open the folder".into(),
            ))
        })
    } else {
        opener
            .reveal_item_in_dir(canonical.display().to_string())
            .map_err(|_| {
                TideError::Streaming(crate::error::StreamingFailure::Resource(
                    "could not open the file location".into(),
                ))
            })
    }
}
