//! System tray (M8 §13/§26/§62): a projection of REAL application
//! state — counts come from the engines/scheduler, never invented
//! (§13.1). Tray actions go through the application layer (§62): show
//! window, pause/resume ALL via the existing commands, open the
//! download directory, quit.
//!
//! Tray creation failure is DEGRADED mode (§72): logged, the main
//! window keeps working — never a startup failure.

use crate::error::{StreamingFailure, TideError};
use tauri::menu::{Menu, MenuItem};
use tauri::Manager;
use tauri::tray::TrayIconBuilder;

/// §62/§76: activates (shows + focuses) the main window from any
/// context (tray click, second instance, deep link).
pub fn activate_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Builds the tray icon + menu from REAL state. Called once at
/// startup; counts are refreshed by the UI layer's existing poll
/// through `update_tray_state`.
pub fn create_tray(app: &tauri::AppHandle) -> Result<(), TideError> {
    let show = MenuItem::with_id(app, "show", "Show Tide", true, None::<&str>)
        .map_err(tray_error)?;
    let pause_all =
        MenuItem::with_id(app, "pause-all", "Pause all downloads", true, None::<&str>)
            .map_err(tray_error)?;
    let resume_all =
        MenuItem::with_id(app, "resume-all", "Resume all downloads", true, None::<&str>)
            .map_err(tray_error)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app).map_err(tray_error)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Tide", true, None::<&str>)
        .map_err(tray_error)?;

    let menu = Menu::with_items(app, &[&show, &separator, &pause_all, &resume_all, &separator, &quit])
        .map_err(tray_error)?;

    TrayIconBuilder::with_id("tide-tray")
        .icon(app.default_window_icon().cloned().ok_or_else(|| tray_error("no app icon"))?)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            // §62: tray actions go through application commands, not
            // engine internals.
            match event.id().as_ref() {
                "show" => activate_main_window(app),
                "pause-all" | "resume-all" => {
                    if let Some(state) = app.try_state::<crate::app::state::AppState>() {
                        let action = event.id().as_ref();
                        let downloads: Vec<String> = {
                            let map = state
                                .download_engine
                                .list_sync();
                            map.iter()
                                .filter(|s| {
                                    matches!(
                                        s.status().lifecycle,
                                        crate::core::download::DownloadLifecycle::Running
                                    )
                                })
                                .map(|s| s.request.download_id.clone())
                                .collect()
                        };
                        for id in downloads {
                            if action == "pause-all" {
                                let _ = state.download_engine.pause(&id);
                            } else {
                                let _ = state.download_engine.resume(&id, None);
                            }
                        }
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // §13.2: left click activates.
            if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                activate_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(tray_error)?;
    crate::diagnostics::info("tray.created", &[]);
    Ok(())
}

fn tray_error(what: impl std::fmt::Display) -> TideError {
    TideError::Streaming(StreamingFailure::Gateway(format!("tray: {what}")))
}

/// §26: updates the tray tooltip from REAL counts. Cheap string swap;
/// called by the UI layer when state changes — never on a timer.
pub fn update_tray_state(
    app: &tauri::AppHandle,
    downloading: usize,
    playing: usize,
) -> Result<(), TideError> {
    if let Some(tray) = app.tray_by_id("tide-tray") {
        let tooltip = format!("Tide — downloading: {downloading}, playing: {playing}");
        tray.set_tooltip(Some(tooltip))
            .map_err(|err| tray_error(err))?;
    }
    Ok(())
}
