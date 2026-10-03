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
        &[("version", env!("CARGO_PKG_VERSION")), ("milestone", "M2")],
    );

    let conn = match persistence::open(&dirs.database_file) {
        Ok(conn) => conn,
        Err(err) => fatal_startup("failed to open database", &err),
    };

    // M2: the download repository owns its own connection (WAL: one
    // writer + readers) so engine checkpoints never block app queries.
    let download_repo =
        match crate::persistence::repositories::DownloadRepository::open(&dirs.database_file) {
            Ok(repo) => std::sync::Arc::new(repo),
            Err(err) => fatal_startup("failed to open download repository", &err),
        };

    // M2 startup recovery (M2 §84): scan persisted tasks, reconcile with
    // the filesystem and register idle runtime entries BEFORE the
    // frontend loads. Not triggered by the UI.
    let download_engine = crate::core::download::DownloadEngine::new(download_repo.clone());
    match crate::core::recovery::recover_all(&download_repo, &download_engine) {
        Ok(outcomes) => {
            diagnostics::info(
                "task_recovery_scan",
                &[("tasks", &outcomes.len().to_string())],
            );
        }
        Err(err) => fatal_startup("startup recovery failed", &err),
    }

    // M3: torrent engine (single librqbit Session, M3 §34) + repository.
    let torrent_metadata_dir = dirs.metadata_dir;
    let torrent_config = crate::core::torrent::TorrentEngineConfig {
        metadata_dir: torrent_metadata_dir,
        dht: true,
        listen_loopback_only: false,
        initial_peers: None,
    };
    let torrent_engine = match tauri::async_runtime::block_on(
        crate::core::torrent::TorrentEngine::new(&torrent_config),
    ) {
        Ok(engine) => engine,
        Err(err) => fatal_startup("failed to initialize torrent engine", &err),
    };
    let torrent_repository =
        match crate::persistence::repositories::TorrentRepository::open(&dirs.database_file) {
            Ok(repo) => std::sync::Arc::new(repo),
            Err(err) => fatal_startup("failed to open torrent repository", &err),
        };
    // M3 torrent recovery (M3 §15): re-add persisted torrent tasks to
    // the session PAUSED; the user resumes explicitly.
    match tauri::async_runtime::block_on(crate::core::torrent::recover_torrents(
        &torrent_repository,
        &torrent_engine,
    )) {
        Ok(outcomes) => {
            diagnostics::info(
                "torrent_recovery_scan",
                &[("tasks", &outcomes.len().to_string())],
            );
        }
        Err(err) => fatal_startup("torrent recovery failed", &err),
    }
    // M4(上): playback manager owns the ONE gateway + the demand
    // scheduler (playback beats background, M4(上) §71).
    let demand_scheduler = std::sync::Arc::new(
        crate::core::streaming::demand::DemandScheduler::default(),
    );
    let playback = std::sync::Arc::new(
        tauri::async_runtime::block_on(crate::core::streaming::PlaybackManager::new(
            demand_scheduler,
        ))
        .unwrap_or_else(|err| fatal_startup("failed to start streaming gateway", &err)),
    );
    let app_state = AppState::new(
        conn,
        download_engine,
        torrent_engine,
        torrent_repository,
        playback,
    );

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
            // M1 HTTP download commands
            commands::download::start_http_download,
            commands::download::pause_download,
            commands::download::resume_download,
            commands::download::cancel_download,
            commands::download::list_downloads,
            commands::download::get_download,
            // M2 persistence commands
            commands::download::retry_download,
            commands::download::delete_download,
            // M3 torrent commands
            commands::torrent::inspect_torrent_source,
            commands::torrent::create_torrent_task,
            commands::torrent::list_torrents,
            commands::torrent::set_torrent_file_selection,
            commands::torrent::pause_torrent,
            commands::torrent::resume_torrent,
            commands::torrent::remove_torrent,
            commands::torrent::delete_torrent_data,
            // M4(上) playback commands
            commands::playback::create_playback_session,
            commands::playback::stop_playback_session,
            commands::playback::get_playback_info,
            commands::playback::notify_seek,
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
            commands::spike::spike_autorun_enabled,
            commands::spike::spike_shutdown,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|err| fatal_startup_raw("failed to build application", &err.to_string()))
        .run(|_app, event| {
            // M2 §31: graceful exit is best-effort. Every engine state
            // transition is already persisted; crash recovery does not
            // depend on this handler running.
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
