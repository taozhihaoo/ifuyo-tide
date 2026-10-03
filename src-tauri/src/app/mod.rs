pub mod paths;
pub mod state;

use crate::commands;
use crate::diagnostics;
use crate::persistence;
use state::AppState;

/// Builds and runs the Tide application.
///
/// Startup failures (paths, logging, database) are fatal and reported to
/// stderr with a non-zero exit code — never silently swallowed.
pub fn run() {
    let dirs = match paths::AppDirs::resolve() {
        Ok(dirs) => dirs,
        Err(err) => fatal_startup("failed to resolve application directories", &err),
    };

    if let Err(err) = diagnostics::init(&dirs.log_file, diagnostics::default_level()) {
        eprintln!("warning: logging disabled: {}", err.message());
    }
    diagnostics::info(
        "app.started",
        &[("version", env!("CARGO_PKG_VERSION")), ("milestone", "M0")],
    );

    let conn = match persistence::open(&dirs.database_file) {
        Ok(conn) => conn,
        Err(err) => fatal_startup("failed to open database", &err),
    };

    let app_state = AppState::new(conn);
    // M0.5 SPIKE: separate managed state for experiment infrastructure.
    let spike_state = crate::spike::state::SpikeState::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(spike_state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info::get_app_info,
            commands::app_info::health_check,
            commands::tasks::list_tasks,
            commands::tasks::create_test_task,
            commands::tasks::delete_task,
            commands::tasks::update_task_state,
            commands::settings::get_settings,
            commands::settings::set_download_directory,
            // M0.5 SPIKE commands
            commands::spike::spike_status,
            commands::spike::spike_list_media,
            commands::spike::spike_start_fixture_server,
            commands::spike::spike_fixture_slow_ms,
            commands::spike::spike_start_torrent,
            commands::spike::spike_torrent_status,
            commands::spike::spike_pause_torrent,
            commands::spike::spike_resume_torrent,
            commands::spike::spike_open_playback_session,
            commands::spike::spike_close_playback_session,
            commands::spike::spike_gateway_slow_ms,
            commands::spike::spike_gateway_read_timeout_ms,
            commands::spike::spike_events,
            commands::spike::spike_clear_events,
            commands::spike::spike_report_media_result,
            commands::spike::spike_read_media_results,
            commands::spike::spike_shutdown,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|err| fatal_startup_raw("failed to build application", &err.to_string()))
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                diagnostics::info("app.shutdown", &[]);
            }
        });
}

fn fatal_startup(what: &str, err: &crate::error::TideError) -> ! {
    fatal_startup_raw(what, &err.message())
}

fn fatal_startup_raw(what: &str, detail: &str) -> ! {
    eprintln!("fatal: {what}: {detail}");
    std::process::exit(1);
}
