//! Settings IPC commands.
//!
//! M0 persists the download directory only (real downloads start in M1).
//! The path is validated in Rust; the diagnostics log never records the
//! path itself.

use crate::app::state::AppState;
use crate::commands::ipc_result;
use crate::dto::SettingsDto;
use crate::error::TideError;
use crate::persistence::repositories::KEY_DOWNLOAD_DIR;
use crate::security;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsDto, TideError> {
    let result = state.with_conn(|conn| {
        let download_dir = state.settings_repository.get(conn, KEY_DOWNLOAD_DIR)?;
        Ok(SettingsDto { download_dir })
    });
    ipc_result("get_settings", result)
}

#[tauri::command]
pub fn set_download_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<SettingsDto, TideError> {
    let result = state.with_tx(|conn| {
        let dir = security::validate_download_dir(&path)?;
        // Ensure the chosen directory exists. This is the only
        // user-initiated filesystem write in M0 and it is scoped to the
        // explicit directory the user picked.
        std::fs::create_dir_all(&dir)
            .map_err(|_| TideError::InvalidInput("download directory cannot be created".into()))?;
        state
            .settings_repository
            .set(conn, KEY_DOWNLOAD_DIR, &dir)?;
        crate::diagnostics::info("settings.updated", &[("key", KEY_DOWNLOAD_DIR)]);
        Ok(SettingsDto {
            download_dir: Some(dir),
        })
    });
    ipc_result("set_download_directory", result)
}
