//! Playback IPC commands (M4(上) §68–§70).
//!
//! The renderer passes resource IDs only — never filesystem paths
//! (M4(上) §59/§60): the backend maps IDs to storage through the
//! trusted repositories (M2 download records / M3 torrent repository).
//! Session URLs and tokens are runtime-generated and never touch
//! SQLite (§69/§70).
//!
//! Per §68's principle ("能由 WebView2 原生事件直接完成的，不要多造
//! IPC"), play/pause/seek/progress are native `<video>` element
//! operations; only session lifecycle and seek-driven demand
//! invalidation cross IPC.

use crate::app::state::AppState;
use crate::core::streaming::playback::PlaybackSessionInfo;
use crate::core::streaming::{content_type_for_name, resource::StreamingResource};
use crate::error::TideError;
use serde::Deserialize;
use tauri::State;

/// What to play. Resolved through trusted backend mappings only.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlaybackSourceRequest {
    /// HTTP download task: the `.part` while in progress (progressive
    /// playback of what exists), the final file once complete.
    DownloadTask { task_id: String },
    /// One file inside a torrent task (M3 repository is the trust root).
    TorrentFile { task_id: String, file_index: u32 },
}

/// Opens a playback session and returns the loopback URL for
/// `video.src` (M4(上) §69 `create_playback_session(resource_id)`).
#[tauri::command]
pub async fn create_playback_session(
    state: State<'_, AppState>,
    source: PlaybackSourceRequest,
) -> Result<PlaybackSessionInfo, TideError> {
    let (resource, name) = resolve_resource(&state, source).await?;
    let content_type = content_type_for_name(&name).to_string();
    super::ipc_result(
        "create_playback_session",
        state.playback.open(resource, content_type),
    )
}

/// Stops a session: the URL stops serving immediately and its queued
/// demands are cancelled (M4(上) §53/§55/§80).
#[tauri::command]
pub fn stop_playback_session(state: State<'_, AppState>, session_id: String) -> bool {
    state.playback.close(&session_id)
}

/// Session metadata for an already-open session (§68
/// `get_playback_info`); `None` if unknown/expired.
#[tauri::command]
pub fn get_playback_info(
    state: State<'_, AppState>,
    session_id: String,
) -> Option<PlaybackSessionInfo> {
    state.playback.info(&session_id)
}

/// The player seeked: demands queued for the old position are obsolete
/// (§80) — cancel them so the new range is served first.
#[tauri::command]
pub fn notify_seek(state: State<'_, AppState>, session_id: String) {
    state.playback.notify_seek(&session_id);
}

/// Maps a playback request to a readable resource via the trusted
/// repositories, returning the resource and its display file name.
async fn resolve_resource(
    state: &State<'_, AppState>,
    source: PlaybackSourceRequest,
) -> Result<(StreamingResource, String), TideError> {
    match source {
        PlaybackSourceRequest::DownloadTask { task_id } => resolve_download(state, &task_id),
        PlaybackSourceRequest::TorrentFile { task_id, file_index } => {
            resolve_torrent_file(state, &task_id, file_index).await
        }
    }
}

/// HTTP download: serve the final file once complete, otherwise the
/// `.part` with its PHYSICAL size — the M2 invariant that the durable
/// file is the resume truth, never the DB number.
fn resolve_download(
    state: &State<'_, AppState>,
    task_id: &str,
) -> Result<(StreamingResource, String), TideError> {
    let record = state
        .download_engine
        .repo()
        .get(task_id)?
        .ok_or_else(|| TideError::NotFound(format!("download task not found: {task_id}")))?;

    if record.state == "completed" && record.final_path.exists() {
        let len = std::fs::metadata(&record.final_path)?.len();
        return Ok((
            StreamingResource::File {
                path: record.final_path.clone(),
                len,
            },
            record.filename.clone(),
        ));
    }

    // In progress / paused / interrupted: whatever is durably on disk.
    if !record.part_path.exists() {
        return Err(TideError::InvalidInput(
            "nothing downloaded yet — start the task first".into(),
        ));
    }
    let len = std::fs::metadata(&record.part_path)?.len();
    Ok((
        StreamingResource::File {
            path: record.part_path.clone(),
            len,
        },
        record.filename.clone(),
    ))
}

/// Torrent file: resolved through the M3 torrent repository
/// (info_hash + file_index + declared size). A paused torrent is
/// resumed so playback is not starved (§52 allows Task=Paused +
/// Playback=Playing; §81 makes playback strict-priority).
async fn resolve_torrent_file(
    state: &State<'_, AppState>,
    task_id: &str,
    file_index: u32,
) -> Result<(StreamingResource, String), TideError> {
    let record = state
        .torrent_repository
        .get_task(task_id)?
        .ok_or_else(|| TideError::NotFound(format!("torrent task not found: {task_id}")))?;
    let info_hash = record.info_hash.ok_or_else(|| {
        TideError::InvalidInput("torrent metadata not resolved yet — cannot play".into())
    })?;
    let file = state
        .torrent_repository
        .list_files(task_id)?
        .into_iter()
        .find(|f| f.file_index == file_index)
        .ok_or_else(|| {
            TideError::NotFound(format!("file index {file_index} not in torrent {task_id}"))
        })?;

    if let Some(handle) = state.torrent_engine.handle_for(&info_hash) {
        if record.state == "paused" {
            state.torrent_engine.resume(&handle).await?;
        }
    } else {
        return Err(TideError::NotFound(
            "torrent is not in the engine — resume the task first".into(),
        ));
    }

    let name = file
        .relative_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&file.relative_path)
        .to_string();
    Ok((
        StreamingResource::Torrent {
            engine: state.torrent_engine.clone(),
            info_hash,
            file_index: file_index as usize,
            len: file.size,
        },
        name,
    ))
}
