//! Download IPC commands (M1 §47).
//!
//! The engine lives in AppState; commands validate input, delegate, and
//! return DTOs. Progress reaches the frontend via the throttled
//! `download://progress` Tauri event (M1 §48), not via polling loops.
//! List/snapshot DTOs report speed 0 — live speed streams via events.

use crate::app::state::AppState;
use crate::core::download::checksum::ChecksumAlgorithm;
use crate::core::download::file_sink::OverwritePolicy;
use crate::core::download::url::validate_download_url;
use crate::core::download::DownloadRequest;
use crate::dto::DownloadDto;
use crate::error::TideError;
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Result of starting one download.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStartedDto {
    pub download_id: String,
}

fn to_dto(download_id: &str, shared: &Arc<crate::core::download::DownloadShared>) -> DownloadDto {
    let status = shared.status();
    DownloadDto {
        download_id: download_id.to_string(),
        host: shared.request.url.host.clone(),
        filename: status.resolved_filename,
        state: status.lifecycle.as_str().to_string(),
        phase: status.phase.as_str().to_string(),
        downloaded_bytes: status.downloaded,
        total_bytes: status.total,
        percentage: status
            .total
            .filter(|t| *t > 0)
            .map(|t| ((status.downloaded as f64 / t as f64) * 100.0).clamp(0.0, 100.0)),
        speed_bps: 0,
        eta_secs: None,
        error_code: status.error.as_ref().map(|(code, _)| code.clone()),
        error_message: status.error.as_ref().map(|(_, message)| message.clone()),
    }
}

/// Starts an HTTP/HTTPS download.
///
/// `destination_dir` defaults to the persisted download directory from
/// Settings. `allow_private` opts into loopback/LAN targets (M1 §8
/// baseline is deny-by-default).
#[tauri::command]
pub async fn start_http_download(
    state: State<'_, AppState>,
    app: AppHandle,
    url: String,
    filename: Option<String>,
    destination_dir: Option<String>,
    overwrite_policy: Option<String>,
    expected_checksum: Option<String>,
    allow_private: Option<bool>,
) -> Result<DownloadStartedDto, TideError> {
    let result = start_impl(
        &state,
        url,
        filename,
        destination_dir,
        overwrite_policy,
        expected_checksum,
        allow_private.unwrap_or(false),
        Some(app),
    )
    .await;
    if let Err(err) = &result {
        crate::commands::log_ipc_error("start_http_download", err);
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn start_impl(
    state: &State<'_, AppState>,
    url: String,
    filename: Option<String>,
    destination_dir: Option<String>,
    overwrite_policy: Option<String>,
    expected_checksum: Option<String>,
    allow_private: bool,
    app: Option<AppHandle>,
) -> Result<DownloadStartedDto, TideError> {
    // Destination: explicit dir or the persisted settings value.
    let destination_dir = match destination_dir {
        Some(dir) => std::path::PathBuf::from(dir),
        None => {
            let dir = state.with_conn(|conn| {
                state
                    .settings_repository
                    .get(conn, crate::persistence::repositories::KEY_DOWNLOAD_DIR)
            })?;
            dir.map(std::path::PathBuf::from).ok_or_else(|| {
                TideError::InvalidInput(
                    "no download directory set — choose one in Settings first".into(),
                )
            })?
        }
    };
    if !destination_dir.is_absolute() {
        return Err(TideError::InvalidInput(
            "download directory must be an absolute path".into(),
        ));
    }

    let validated = validate_download_url(&url, allow_private)?;
    let policy = match overwrite_policy.as_deref() {
        None | Some("") => OverwritePolicy::AutoRename,
        Some(raw) => OverwritePolicy::parse(raw)?,
    };
    let checksum = expected_checksum
        .as_deref()
        .map(|hex| (ChecksumAlgorithm::Sha256, hex.to_string()));
    let request = DownloadRequest::new(validated, destination_dir, filename, policy, checksum)?;

    let started = state.download_engine.start(request, app).await?;
    Ok(DownloadStartedDto {
        download_id: started.request.download_id.clone(),
    })
}

#[tauri::command]
pub async fn pause_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<(), TideError> {
    state.download_engine.pause(&download_id).await
}

#[tauri::command]
pub async fn resume_download(
    state: State<'_, AppState>,
    app: AppHandle,
    download_id: String,
) -> Result<(), TideError> {
    state.download_engine.resume(&download_id, Some(app)).await
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<(), TideError> {
    state.download_engine.cancel(&download_id).await
}

/// Snapshot of every in-memory download (M1 sessions only; M2 adds
/// persistence).
#[tauri::command]
pub async fn list_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadDto>, TideError> {
    let entries = state.download_engine.list().await;
    Ok(entries
        .iter()
        .map(|shared| to_dto(&shared.request.download_id, shared))
        .collect())
}

/// Point snapshot of one download (used after events were missed).
#[tauri::command]
pub async fn get_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<DownloadDto, TideError> {
    let shared = state
        .download_engine
        .get(&download_id)
        .await
        .ok_or_else(|| TideError::NotFound(format!("download not found: {download_id}")))?;
    Ok(to_dto(&download_id, &shared))
}

/// Retry a failed download (M2 §52): same task id, new runtime, resume
/// from the safe point. Works after an app restart — the task is
/// restored from the persisted record and re-validated.
#[tauri::command]
pub async fn retry_download(
    state: State<'_, AppState>,
    app: AppHandle,
    download_id: String,
) -> Result<(), TideError> {
    state.download_engine.retry(&download_id, Some(app)).await
}

/// Delete a task (M2 §53): DB record + .part are removed; the
/// downloaded final file is user data and is preserved.
#[tauri::command]
pub async fn delete_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<(), TideError> {
    state.download_engine.delete(&download_id).await
}
