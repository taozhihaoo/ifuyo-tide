//! Playback IPC commands (M4(上) §68–§70, M4(下) §99/§102/§127/§146).
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
use crate::core::streaming::{content_type_for_name, OriginSource, StreamingResource};
use crate::error::TideError;
use serde::Deserialize;
use tauri::State;

/// What to play. Resolved through trusted backend mappings only.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlaybackSourceRequest {
    /// HTTP download task: the `.part` while in progress (progressive
    /// playback of what exists + origin read-through for seeks beyond
    /// the frontier, M4(下) §127), the final file once complete.
    DownloadTask { task_id: String },
    /// One file inside a torrent task (M3 repository is the trust root).
    TorrentFile { task_id: String, file_index: u32 },
    /// A library item (M6(上) §46/§59: item id → backend validation →
    /// capability URL; the renderer never sees a path).
    LibraryItem { item_id: String },
}

/// Opens a playback session and returns the loopback URL for
/// `video.src` (M4(上) §69 `create_playback_session(resource_id)`).
#[tauri::command]
pub async fn create_playback_session(
    state: State<'_, AppState>,
    source: PlaybackSourceRequest,
) -> Result<PlaybackSessionInfo, TideError> {
    let (resource, name, owner) = resolve_resource(&state, source).await?;
    let content_type = content_type_for_name(&name).to_string();
    super::ipc_result(
        "create_playback_session",
        state.playback.open(resource, content_type, owner),
    )
}

/// Stops a session: the URL stops serving immediately, in-flight reads
/// abort, and its queued demands are cancelled (M4(上) §53/§55/§80).
#[tauri::command]
pub fn stop_playback_session(state: State<'_, AppState>, session_id: String) -> bool {
    state.playback.close(&session_id)
}

/// Session metadata + live backend availability (§68
/// `get_playback_info`, M4(下) §149); `None` if unknown/expired.
#[tauri::command]
pub async fn get_playback_info(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<PlaybackSessionInfo>, TideError> {
    Ok(state.playback.status(&session_id).await)
}

/// The most recently opened still-alive session (M5 §37: frontend
/// reload recovers the player projection from THIS, never from React
/// state); `None` when nothing has played.
#[tauri::command]
pub async fn get_active_playback_session(
    state: State<'_, AppState>,
) -> Result<Option<PlaybackSessionInfo>, TideError> {
    match state.playback.active_session() {
        Some(info) => Ok(state.playback.status(&info.session_id).await),
        None => Ok(None),
    }
}

/// M5 §36: the player reports its position (throttled, while playing)
/// so a frontend reload can resume at the same spot. Session-scoped
/// only — no watch history is kept.
#[tauri::command]
pub fn report_playback_position(
    state: State<'_, AppState>,
    session_id: String,
    position_ms: u64,
) -> bool {
    state.playback.report_position(&session_id, position_ms)
}

/// The player seeked: demands queued for the old position are obsolete
/// (§80) — cancel them so the new range is served first.
#[tauri::command]
pub fn notify_seek(state: State<'_, AppState>, session_id: String) {
    state.playback.notify_seek(&session_id);
}

/// Maps a playback request to a readable resource via the trusted
/// repositories, returning the resource, its display file name, and
/// the owning task id (§102 invalidation key).
async fn resolve_resource(
    state: &State<'_, AppState>,
    source: PlaybackSourceRequest,
) -> Result<(StreamingResource, String, Option<String>), TideError> {
    match source {
        PlaybackSourceRequest::DownloadTask { task_id } => {
            Ok(resolve_download(state, &task_id).map(|(r, n)| (r, n, Some(task_id)))?)
        }
        PlaybackSourceRequest::TorrentFile {
            task_id,
            file_index,
        } => {
            let (r, n) = resolve_torrent_file(state, &task_id, file_index).await?;
            Ok((r, n, Some(task_id)))
        }
        PlaybackSourceRequest::LibraryItem { item_id } => {
            // §48: the service re-validates the filesystem right now;
            // a completed library file maps to the plain File resource
            // — fully available, ranged reads through the gateway.
            let (canonical, size, _name) = state.library.play_source(&item_id)?;
            let name = canonical
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok((
                StreamingResource::File {
                    path: canonical,
                    len: size,
                },
                name,
                Some(item_id),
            ))
        }
    }
}

/// HTTP download: serve the final file once complete; while in
/// progress serve the `.part` — a live durable-len watch (event-driven
/// frontier, M4(下) §115) plus an origin read-through source for
/// beyond-frontier seeks (§127).
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
        return Err(TideError::Streaming(
            crate::error::StreamingFailure::Playback(
                "nothing downloaded yet — start the task first".into(),
            ),
        ));
    }
    // Live engine entry (if the task is restored) provides the
    // event-driven frontier signal.
    let len_rx = state
        .download_engine
        .get_sync(task_id)
        .map(|shared| shared.subscribe_durable_len());
    Ok((
        StreamingResource::Part {
            path: record.part_path.clone(),
            total_len: record.total_bytes,
            len_rx,
            origin: Some(OriginSource {
                url: record.source_url.clone(),
            }),
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
        TideError::Streaming(crate::error::StreamingFailure::Playback(
            "torrent metadata not resolved yet — cannot play".into(),
        ))
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
        return Err(TideError::Streaming(
            crate::error::StreamingFailure::Resource(
                "torrent is not in the engine — resume the task first".into(),
            ),
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
