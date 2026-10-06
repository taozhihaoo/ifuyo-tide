//! Torrent IPC commands (M3 §32).
//!
//! The engine and the torrent repository live in AppState; commands
//! validate, delegate, and return DTOs. librqbit types never cross IPC
//! (M3 §5/§32).

use crate::app::state::AppState;
use crate::core::torrent::source::validate_magnet_uri;
use crate::error::TideError;
use crate::persistence::repositories::{TorrentFileRecord, TorrentRepository, TorrentTaskRecord};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use tauri::State;

fn now_ms() -> i64 {
    crate::core::time::now_unix_ms()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFileDto {
    pub index: u32,
    pub path: String,
    pub size: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentDto {
    pub id: String,
    pub info_hash: Option<String>,
    pub source_kind: String,
    pub display_name: String,
    pub state: String,
    pub phase: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
    pub selected_count: Option<u32>,
    pub files: Vec<TorrentFileDto>,
    /// Live engine speed (bytes/sec); None = no live stats yet. Read
    /// at query time — never persisted, never faked (§39).
    #[serde(default)]
    pub download_speed_bps: Option<u64>,
    /// Last recovery note (e.g. readd_timeout) — drives the "Reset
    /// partial data" affordance in the UI.
    #[serde(default)]
    pub recovery_reason: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentInspectionDto {
    pub info_hash: String,
    pub name: String,
    pub total_size: u64,
    pub piece_length: u32,
    pub files: Vec<TorrentFileDto>,
}

fn torrent_dto(
    record: &TorrentTaskRecord,
    repo: &TorrentRepository,
    download_speed_bps: Option<u64>,
) -> TorrentDto {
    let files = repo
        .list_files(&record.id)
        .unwrap_or_default()
        .into_iter()
        .map(|f| TorrentFileDto {
            index: f.file_index,
            path: f.relative_path,
            size: f.size,
            selected: f.selected,
        })
        .collect();
    TorrentDto {
        id: record.id.clone(),
        info_hash: record.info_hash.clone(),
        source_kind: record.source_kind.clone(),
        display_name: record.display_name.clone(),
        state: record.state.clone(),
        phase: record.phase.clone(),
        total_bytes: record.total_bytes,
        downloaded_bytes: record.downloaded_bytes,
        uploaded_bytes: record.uploaded_bytes,
        selected_count: record.selected_count,
        files,
        download_speed_bps,
        recovery_reason: record.recovery_reason.clone(),
        error_code: record.error_code.clone(),
        error_message: record.error_message.clone(),
    }
}

/// Inspects a local `.torrent` file: metadata + file list + path safety
/// validation, WITHOUT creating a task or touching the network (M3 §59).
#[tauri::command]
pub async fn inspect_torrent_source(
    state: State<'_, AppState>,
    torrent_path: String,
) -> Result<TorrentInspectionDto, TideError> {
    let bytes = std::fs::read(&torrent_path)
        .map_err(|_| TideError::NotFound(format!("torrent file not found: {torrent_path}")))?;
    let engine = &state.torrent_engine;
    let metadata = engine.inspect_torrent_bytes(bytes).await?;
    Ok(TorrentInspectionDto {
        info_hash: metadata.info_hash,
        name: metadata.name,
        total_size: metadata.total_size,
        piece_length: metadata.piece_length,
        files: metadata
            .files
            .into_iter()
            .map(|f| TorrentFileDto {
                index: f.index as u32,
                path: f.relative_path,
                size: f.size,
                selected: true,
            })
            .collect(),
    })
}

/// Creates a torrent task from a magnet URI or a local `.torrent` file
/// and adds it to the engine (paused — the user starts/resumes it).
#[tauri::command]
pub async fn create_torrent_task(
    state: State<'_, AppState>,
    source_kind: String,
    magnet_uri: Option<String>,
    torrent_path: Option<String>,
    destination_dir: String,
) -> Result<DownloadStartedLike, TideError> {
    let started = Instant::now();
    let _dirs = crate::app::paths::AppDirs::resolve()?;
    let id = crate::core::ids::new_id();

    if !std::path::Path::new(&destination_dir).is_absolute() {
        return Err(TideError::InvalidInput(
            "download directory must be an absolute path".into(),
        ));
    }
    let destination = PathBuf::from(&destination_dir);
    std::fs::create_dir_all(&destination)?;

    let (record, metadata_bytes, info_hash) = match source_kind.as_str() {
        "magnet" => {
            let uri = magnet_uri.ok_or_else(|| {
                TideError::InvalidInput("magnet source requires magnet_uri".into())
            })?;
            let (uri, info_hash, name) = validate_magnet_uri(&uri)?;
            (
                TorrentTaskRecord {
                    id: id.clone(),
                    info_hash: Some(info_hash.clone()),
                    source_kind: "magnet".into(),
                    magnet_uri: Some(uri),
                    metadata_path: None,
                    metadata_sha256: None,
                    display_name: name.unwrap_or_else(|| info_hash.clone()),
                    storage_root: destination,
                    state: "ready".into(),
                    // Honest phase (§39): until the DHT delivers the
                    // metadata there is nothing to download yet — the
                    // monitor advances this to downloading on resolution.
                    phase: "metadata_pending".into(),
                    total_bytes: None,
                    downloaded_bytes: 0,
                    uploaded_bytes: 0,
                    selected_count: None,
                    error_code: None,
                    error_message: None,
                    recovery_reason: None,
                    created_at: now_ms(),
                    updated_at: now_ms(),
                    completed_at: None,
                },
                None,
                Some(info_hash),
            )
        }
        "torrent_file" => {
            let path = torrent_path.ok_or_else(|| {
                TideError::InvalidInput("torrent_file source requires torrent_path".into())
            })?;
            let bytes = std::fs::read(&path)
                .map_err(|_| TideError::NotFound(format!("torrent file not found: {path}")))?;
            // Inspect BEFORE creating the task: validates every path in
            // the metadata (M3 §12) and gives us the info hash.
            let metadata = state
                .torrent_engine
                .inspect_torrent_bytes(bytes.clone())
                .await?;
            // Stable metadata copy (M3 §6): survives the original moving.
            let meta_dir = dirs_root()?.join("torrent-metadata");
            std::fs::create_dir_all(&meta_dir)?;
            let meta_path = meta_dir.join(format!("{id}.torrent"));
            std::fs::write(&meta_path, &bytes)?;
            let mut hasher = crate::core::download::checksum::ChecksumWriter::new();
            hasher.update(&bytes);
            let sha = hasher.finalize_hex();
            let record = TorrentTaskRecord {
                id: id.clone(),
                info_hash: Some(metadata.info_hash.clone()),
                source_kind: "torrent_file".into(),
                magnet_uri: None,
                metadata_path: Some(meta_path),
                metadata_sha256: Some(sha),
                display_name: metadata.name.clone(),
                storage_root: destination,
                state: "ready".into(),
                phase: "downloading".into(),
                total_bytes: Some(metadata.total_size),
                downloaded_bytes: 0,
                uploaded_bytes: 0,
                selected_count: Some(metadata.files.len() as u32),
                error_code: None,
                error_message: None,
                recovery_reason: None,
                created_at: now_ms(),
                updated_at: now_ms(),
                completed_at: None,
            };
            let file_rows: Vec<TorrentFileRecord> = metadata
                .files
                .iter()
                .map(|f| TorrentFileRecord {
                    file_index: f.index as u32,
                    info_hash: metadata.info_hash.clone(),
                    relative_path: f.relative_path.clone(),
                    size: f.size,
                    selected: true,
                })
                .collect();
            (record, Some((bytes, file_rows)), Some(metadata.info_hash))
        }
        other => {
            return Err(TideError::InvalidInput(format!(
                "unknown torrent source kind: {other}"
            )));
        }
    };

    let repo = state.torrent_repository.clone();
    repo.insert_task(&record)?;
    if let Some((_bytes, file_rows)) = &metadata_bytes {
        if let Some(hash) = &info_hash {
            repo.replace_files(&id, hash, file_rows)?;
        }
    }

    // Add to the session PAUSED: the user starts/resumes explicitly
    // (M3 §23 — file list first, then download).
    let added = match source_kind.as_str() {
        "magnet" => {
            let uri = record.magnet_uri.clone().expect("magnet uri");
            state
                .torrent_engine
                .add_magnet(uri, &record.storage_root, None, false)
                .await?
        }
        "torrent_file" => {
            let meta_path = record.metadata_path.clone().expect("metadata copy");
            let bytes = std::fs::read(&meta_path)?;
            // M3 §23: metadata (and the file list) is known up front —
            // add PAUSED so nothing downloads until the user picks the
            // files and resumes.
            state
                .torrent_engine
                .add_torrent_bytes(bytes, &record.storage_root, None, None, true)
                .await?
        }
        _ => unreachable!(),
    };

    if record.info_hash.is_none() {
        repo.update_task(
            &id,
            "ready",
            "downloading",
            Some(&added.info_hash),
            record.total_bytes,
            0,
            0,
            None,
            None,
            None,
            None,
        )?;
    }
    crate::diagnostics::info(
        "torrent.created",
        &[
            ("task_id", &id),
            ("setup_ms", &started.elapsed().as_millis().to_string()),
        ],
    );
    Ok(DownloadStartedLike {
        id,
        info_hash: added.info_hash,
    })
}

fn dirs_root() -> Result<PathBuf, TideError> {
    Ok(crate::app::paths::AppDirs::resolve()?.root)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStartedLike {
    pub id: String,
    pub info_hash: String,
}

#[tauri::command]
pub async fn list_torrents(state: State<'_, AppState>) -> Result<Vec<TorrentDto>, TideError> {
    let repo = state.torrent_repository.clone();
    let records = repo.list_tasks()?;
    Ok(records
        .iter()
        .map(|r| {
            let speed = r
                .info_hash
                .as_deref()
                .and_then(|h| state.torrent_engine.handle_for(h))
                .and_then(|h| state.torrent_engine.live_download_speed_bps(&h));
            torrent_dto(r, &repo, speed)
        })
        .collect())
}

#[tauri::command]
pub async fn set_torrent_file_selection(
    state: State<'_, AppState>,
    id: String,
    selected: Vec<u32>,
) -> Result<(), TideError> {
    let record = state
        .torrent_repository
        .get_task(&id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {id}")))?;
    let hash = record
        .info_hash
        .clone()
        .ok_or_else(|| TideError::InvalidInput("torrent metadata is not resolved yet".into()))?;
    let files = state.torrent_repository.list_files(&id)?;
    let wanted: Vec<(u32, bool)> = files
        .iter()
        .map(|f| (f.file_index, selected.contains(&f.file_index)))
        .collect();
    state
        .torrent_repository
        .update_selection(&id, &hash, &wanted)?;
    let engine = &state.torrent_engine;
    if let Some(handle) = engine.handle_for(&hash) {
        let indices: Vec<usize> = selected.iter().copied().map(|i| i as usize).collect();
        engine.set_only_files(&handle, &indices).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn pause_torrent(state: State<'_, AppState>, id: String) -> Result<(), TideError> {
    let record = state
        .torrent_repository
        .get_task(&id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {id}")))?;
    let hash = record
        .info_hash
        .clone()
        .ok_or_else(|| TideError::InvalidInput("torrent metadata is not resolved yet".into()))?;
    let engine = &state.torrent_engine;
    if let Some(handle) = engine.handle_for(&hash) {
        engine.pause(&handle).await?;
    }
    // Pause is a TASK STATE, not an engine phase: the schema CHECK only
    // allows metadata_pending/metadata_failed/downloading/checking/
    // seeding/completed/error for phase, so keep the row's phase and
    // flip state only (writing phase='paused' fails the CHECK and the
    // whole update is rejected — the pause then never persists).
    repo_update(
        &state.torrent_repository,
        &id,
        "paused",
        &record.phase,
        &record,
        None,
    );
    Ok(())
}

#[tauri::command]
pub async fn resume_torrent(state: State<'_, AppState>, id: String) -> Result<(), TideError> {
    let record = state
        .torrent_repository
        .get_task(&id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {id}")))?;
    let hash = record
        .info_hash
        .clone()
        .ok_or_else(|| TideError::InvalidInput("torrent metadata is not resolved yet".into()))?;
    let engine = &state.torrent_engine;
    match engine.handle_for(&hash) {
        Some(handle) => engine.resume(&handle).await?,
        None => {
            // Not in the session — the startup re-add may have timed out
            // (1.0.4 bounded recovery on stale partial data). Re-add on
            // demand, LIVE, honoring the saved selection, instead of
            // telling the user to "restart the app" (which would hit
            // the same timeout loop — 1.0.6 real-use finding).
            let metadata_bytes = crate::core::torrent::recovery::load_metadata_bytes(&record)?;
            let only_files = crate::core::torrent::recovery::saved_selection(
                &state.torrent_repository,
                &record,
            )?;
            let readd = crate::core::torrent::recovery::readd_to_engine(
                engine,
                &record,
                metadata_bytes,
                false,
                only_files,
            );
            let added = match tokio::time::timeout(std::time::Duration::from_secs(60), readd).await
            {
                Ok(result) => result?,
                Err(_) => {
                    return Err(TideError::Internal(
                        "torrent engine is stuck re-adding this torrent — its partial data may be inconsistent. Use Remove (keep files) or Delete data, then add the source again".into(),
                    ));
                }
            };
            let new_phase = if engine.metadata_resolved(&added.handle) {
                "downloading"
            } else {
                "metadata_pending"
            };
            repo_update(
                &state.torrent_repository,
                &id,
                "running",
                new_phase,
                &record,
                None,
            );
            return Ok(());
        }
    }
    repo_update(
        &state.torrent_repository,
        &id,
        "running",
        "downloading",
        &record,
        None,
    );
    Ok(())
}

/// Remove from Tide: stops the engine task, removes the DB record and
/// file-selection rows; the DOWNLOADED DATA IS PRESERVED (M3 §20).
#[tauri::command]
pub async fn remove_torrent(state: State<'_, AppState>, id: String) -> Result<(), TideError> {
    let record = state
        .torrent_repository
        .get_task(&id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {id}")))?;
    if let Some(hash) = &record.info_hash {
        let engine = &state.torrent_engine;
        if let Some(handle) = engine.handle_for(hash) {
            engine.remove(&handle, false).await?;
        }
    }
    state.torrent_repository.delete_task(&id)?;
    // M4(下) §102: sessions on a removed task are invalidated —
    // no session outlives its engine handle.
    state.playback.close_for_owner(&id);
    Ok(())
}

/// Remove + Delete Data: also deletes the torrent's downloaded files
/// via the engine (M3 §20 "Remove + Delete Data").
#[tauri::command]
pub async fn delete_torrent_data(state: State<'_, AppState>, id: String) -> Result<(), TideError> {
    let record = state
        .torrent_repository
        .get_task(&id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {id}")))?;
    let mut in_engine = false;
    if let Some(hash) = &record.info_hash {
        let engine = &state.torrent_engine;
        if let Some(handle) = engine.handle_for(hash) {
            engine.remove(&handle, true).await?;
            in_engine = true;
        }
    }
    if !in_engine {
        // Not in the engine (e.g. its startup re-add timed out): the
        // engine path above never ran, so the files would be orphaned
        // on disk while the record — and the user's "delete data"
        // intent — disappeared. Clean them here.
        crate::core::torrent::recovery::reset_partial_files(&record, &state.torrent_repository)?;
    }
    state.torrent_repository.delete_task(&id)?;
    // M4(下) §102: data is gone — playback sessions must die with it.
    state.playback.close_for_owner(&id);
    Ok(())
}

/// "Reset partial data" (1.0.8): removes the task's downloaded/partial
/// files so a fresh re-add cannot hit the stale-data hang. The task and
/// its source (magnet URI / metadata copy) are kept — no need for the
/// user to re-provide anything. Refuses while the engine holds the
/// torrent (pause first); terminal tasks have nothing partial to reset.
#[tauri::command]
pub async fn reset_torrent_partial_data(
    state: State<'_, AppState>,
    id: String,
) -> Result<u32, TideError> {
    let record = state
        .torrent_repository
        .get_task(&id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {id}")))?;
    if matches!(record.state.as_str(), "completed" | "cancelled") {
        return Err(TideError::InvalidInput(
            "completed tasks have no partial data to reset".into(),
        ));
    }
    if let Some(hash) = &record.info_hash {
        if let Some(handle) = state.torrent_engine.handle_for(hash) {
            return Err(TideError::InvalidInput(
                "pause the torrent first (it is still held by the engine)".into(),
            ));
        }
    }
    let repo = state.torrent_repository.clone();
    let record_for_reset = record.clone();
    let removed = tokio::task::spawn_blocking(move || {
        crate::core::torrent::recovery::reset_partial_files(&record_for_reset, &repo)
    })
    .await
    .map_err(|err| TideError::Internal(format!("reset failed: {err}")))??;
    Ok(removed)
}

fn repo_update(
    repo: &TorrentRepository,
    id: &str,
    state: &str,
    phase: &str,
    record: &TorrentTaskRecord,
    completed_at: Option<i64>,
) {
    let _ = repo.update_task(
        id,
        state,
        phase,
        record.info_hash.as_deref(),
        record.total_bytes,
        record.downloaded_bytes,
        record.uploaded_bytes,
        record.selected_count,
        None,
        None,
        completed_at,
    );
}

// AppHandle is currently unused in torrent commands (state polling via
// list_torrents); kept out to avoid event-bridge scope creep in M3.
#[allow(unused_imports)]
use tauri::AppHandle as _AppHandleUnused;
