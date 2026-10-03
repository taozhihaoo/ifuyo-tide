//! Basic application info / health commands.

use crate::app::state::AppState;
use crate::commands::ipc_result;
use crate::dto::AppInfoDto;
use crate::error::TideError;
use tauri::State;

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> Result<AppInfoDto, TideError> {
    let _ = state;
    ipc_result(
        "get_app_info",
        Ok(AppInfoDto {
            app_name: "ifuyo Tide".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            milestone: "M0".to_string(),
        }),
    )
}

#[tauri::command]
pub fn health_check() -> Result<(), TideError> {
    ipc_result("health_check", Ok(()))
}
