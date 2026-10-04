//! Windows-integration IPC (M8 §44/§78/§80): settings get/set via the
//! existing settings KV, autostart via the official plugin, pending
//! launch-request drain (§74) — the frontend confirms every external
//! source before a task is created (§46).

use crate::app::state::AppState;
use crate::error::TideError;
use crate::platform::settings::WindowsSettings;
use tauri::State;
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn windows_get_settings(state: State<'_, AppState>) -> Result<WindowsSettings, TideError> {
    Ok(state.windows_settings())
}

#[tauri::command]
pub fn windows_set_settings(
    state: State<'_, AppState>,
    settings: WindowsSettings,
) -> Result<(), TideError> {
    state.set_windows_settings(settings)
}

#[tauri::command]
pub fn windows_autostart_enable(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), TideError> {
    app.autolaunch().enable().map_err(|_| {
        TideError::Streaming(crate::error::StreamingFailure::Resource(
            "autostart could not be enabled".into(),
        ))
    })?;
    // Persist the DESIRED state so Settings reflects it after restart.
    let mut s = state.windows_settings();
    s.autostart_enabled = true;
    state.set_windows_settings(s)
}

#[tauri::command]
pub fn windows_autostart_disable(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), TideError> {
    app.autolaunch().disable().map_err(|_| {
        TideError::Streaming(crate::error::StreamingFailure::Resource(
            "autostart could not be disabled".into(),
        ))
    })?;
    let mut s = state.windows_settings();
    s.autostart_enabled = false;
    state.set_windows_settings(s)
}

#[tauri::command]
pub fn windows_autostart_is_enabled(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, TideError> {
    let enabled = app.autolaunch().is_enabled().map_err(|_| {
        TideError::Streaming(crate::error::StreamingFailure::Resource(
            "autostart state unavailable".into(),
        ))
    })?;
    // Keep the persisted view in sync with the registry truth.
    let mut s = state.windows_settings();
    s.autostart_enabled = enabled;
    let _ = state.set_windows_settings(s);
    Ok(enabled)
}

/// §74: drains the pending launch-request queue (oldest first). Empty
/// after the call — each request is delivered exactly once.
#[tauri::command]
pub fn windows_drain_launch_requests(
    app: tauri::AppHandle,
) -> Result<Vec<crate::platform::launch::LaunchRequest>, TideError> {
    use tauri::Manager;
    let queue = app.state::<crate::platform::router::PendingLaunchQueue>();
    Ok(queue.drain())
}
