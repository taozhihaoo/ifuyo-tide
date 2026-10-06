pub mod paths;
pub mod state;

use crate::commands;
use crate::diagnostics;
use crate::persistence;
use state::AppState;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

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
    // the session PAUSED; the user resumes explicitly. Runs on a
    // BACKGROUND task: a slow or hung engine re-add (librqbit can stall
    // re-adding a torrent over stale partial data) must never delay
    // the first window — the UI reads the DB rows, which already exist,
    // and the completion monitor reconciles independently.
    {
        let torrent_engine = torrent_engine.clone();
        let torrent_repository = torrent_repository.clone();
        tauri::async_runtime::spawn(async move {
            match crate::core::torrent::recover_torrents(&torrent_repository, &torrent_engine).await
            {
                Ok(outcomes) => {
                    diagnostics::info(
                        "torrent_recovery_scan",
                        &[("tasks", &outcomes.len().to_string())],
                    );
                }
                Err(err) => {
                    diagnostics::warn("torrent_recovery_failed", &[("code", err.code())]);
                }
            }
        });
    }
    // M6(上): the library service (own connection, same versioned
    // database file).
    let library_repo = match crate::core::library::LibraryRepository::open(&dirs.database_file) {
        Ok(repo) => std::sync::Arc::new(repo),
        Err(err) => fatal_startup("failed to open library repository", &err),
    };
    let library = std::sync::Arc::new(crate::core::library::LibraryService::new(library_repo));

    // M6(上) §26: completed downloads enter the library automatically.
    // The hook is best-effort: an import failure must never fail the
    // (already finished) download.
    {
        let library = library.clone();
        download_engine.set_on_complete(std::sync::Arc::new(move |shared| {
            let status = shared.status();
            if let Some(final_path) = &status.final_path {
                if let Err(err) = library.import_download(
                    final_path,
                    &shared.request.destination_dir,
                    &shared.request.download_id,
                    shared
                        .request
                        .expected_checksum
                        .as_ref()
                        .map(|(_, hex)| hex.as_str()),
                ) {
                    crate::diagnostics::warn("library.import_failed", &[("code", err.code())]);
                }
            }
        }));
    }

    // M6(上) §27 enabling fix: torrent completion was never detected —
    // the monitor reconciles finished torrents and imports their files.
    crate::core::torrent::monitor::spawn(
        torrent_engine.clone(),
        torrent_repository.clone(),
        library.clone(),
    );

    // M6(上) §71/§73: light startup verification on a background task —
    // the UI never waits for it (§72).
    {
        let library = library.clone();
        tauri::async_runtime::spawn(async move {
            match tauri::async_runtime::spawn_blocking(move || library.verify_light_blocking())
                .await
            {
                Ok(Ok(changed)) if changed > 0 => {
                    crate::diagnostics::info(
                        "library.startup_verify",
                        &[("changed", &changed.to_string())],
                    );
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => {
                    crate::diagnostics::warn("library.startup_verify_failed", &[]);
                }
            }
        });
    }

    // M4(上): playback manager owns the ONE gateway + the demand
    // scheduler (playback beats background, M4(上) §71).
    let demand_scheduler =
        std::sync::Arc::new(crate::core::streaming::demand::DemandScheduler::default());
    let playback = std::sync::Arc::new(
        tauri::async_runtime::block_on(crate::core::streaming::PlaybackManager::new(
            demand_scheduler,
        ))
        .unwrap_or_else(|err| fatal_startup("failed to start streaming gateway", &err)),
    );
    // M4(上) §55 / M4(下) §138: idle-session reaping — one low-frequency
    // task, no per-request timers, no busy loop.
    {
        let playback = playback.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
                playback.reap_expired();
            }
        });
    }

    // M7(上): the scheduler runtime — event-driven + 500 ms tick (§18).
    // Holds Arcs only; process exit ends the loop (§132).
    let user_priorities = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
        String,
        crate::core::scheduler::UserPriority,
    >::new()));
    let global_bandwidth_bps = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let scheduler_provider = crate::core::scheduler::AppSnapshotProvider {
        downloads: download_engine.clone(),
        torrents: torrent_engine.clone(),
        torrent_repo: torrent_repository.clone(),
        playback: playback.clone(),
        user_priorities: user_priorities.clone(),
        global_bandwidth_bps: global_bandwidth_bps.clone(),
    };
    let scheduler_runtime = crate::core::scheduler::SchedulerRuntime::new(
        crate::core::scheduler::SchedulerConfig::default(),
        Box::new(scheduler_provider),
        download_engine.clone(),
        torrent_engine.clone(),
    );
    scheduler_runtime.spawn_loop();
    diagnostics::info("scheduler.started", &[]);
    // M8 §80: load Windows-integration settings (separate short-lived
    // connection; the app-state `conn` moves into AppState below).
    let windows_settings = {
        match rusqlite::Connection::open(&dirs.database_file) {
            Ok(conn) => crate::platform::settings::load(&conn).unwrap_or_default(),
            Err(_) => Default::default(),
        }
    };
    let app_state = AppState::new(
        conn,
        download_engine,
        torrent_engine,
        torrent_repository,
        playback,
        library,
        scheduler_runtime,
        user_priorities,
        global_bandwidth_bps,
        windows_settings,
    );

    // M7(上) §72: load persisted scheduler preferences (user priorities
    // + bandwidth limit) into the runtime registries.
    commands::scheduler::load_persisted(&app_state);

    // M0.5 SPIKE: separate managed state for experiment infrastructure.
    let spike_state = crate::spike::state::SpikeState::default();

    // M8 §73 startup order: single instance FIRST (second launches are
    // forwarded here and never reach the builder), then core state,
    // then the Windows integrations. A launch request arriving before
    // the frontend is ready goes to the pending queue (§74).
    let pending_launches = std::sync::Arc::new(crate::platform::router::PendingLaunchQueue::new());

    tauri::Builder::default()
        // §8.1: single instance MUST be registered FIRST. The callback
        // receives (&AppHandle, argv, cwd) on the SECOND process; the
        // first process's app handle routes the request (§8.3).
        .plugin(tauri_plugin_single_instance::init({
            let pending_launches = pending_launches.clone();
            move |app: &tauri::AppHandle, argv: Vec<String>, _cwd: String| {
                // §57: forwarded args are UNTRUSTED — classify + validate.
                let args: Vec<String> = argv.iter().skip(1).cloned().collect();
                match crate::platform::launch::parse_args(&args) {
                    Ok(requests) => {
                        crate::platform::tray::activate_main_window(app);
                        for request in requests {
                            let _ = crate::platform::router::route_launch_request(
                                app,
                                &pending_launches,
                                request,
                            );
                        }
                    }
                    Err(err) => {
                        crate::diagnostics::warn(
                            "launch_request.rejected",
                            &[("code", err.code())],
                        );
                        crate::platform::tray::activate_main_window(app);
                    }
                }
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_deep_link::init())
        .manage(pending_launches.clone())
        .manage(app_state)
        .manage(spike_state)
        .setup(move |app| {
            // M8 §73: Windows integrations initialize AFTER core state;
            // each one degrades independently (§72) — a tray/deep-link
            // failure is a warning, never a startup failure.
            if let Err(err) = crate::platform::tray::create_tray(app.handle()) {
                crate::diagnostics::warn("tray.failed", &[("code", err.code())]);
            }
            if let Err(err) = app.deep_link().register("tide") {
                crate::diagnostics::warn("deeplink.register_failed", &[]);
                let _ = err; // details stay out of user-facing logs
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // §13.3/§22.2: close-to-tray — closing the window hides it;
            // the app keeps running in the tray (§14 window hidden ≠
            // application stopped).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .app_handle()
                    .try_state::<crate::app::state::AppState>()
                    .map(|state| state.windows_settings().close_to_tray)
                    .unwrap_or(false);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info::get_app_info,
            commands::app_info::health_check,
            commands::tasks::list_tasks,
            commands::tasks::create_test_task,
            commands::tasks::delete_task,
            commands::tasks::update_task_state,
            commands::settings::get_settings,
            commands::settings::set_download_directory,
            commands::settings::get_onboarding_completed,
            commands::settings::set_onboarding_completed,
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
            // M7(上) scheduler commands
            commands::scheduler::scheduler_set_task_priority,
            commands::scheduler::scheduler_set_global_bandwidth_limit,
            commands::scheduler::scheduler_get_snapshot,
            // M8 Windows integration
            commands::windows::windows_get_settings,
            commands::windows::windows_set_settings,
            commands::windows::windows_autostart_enable,
            commands::windows::windows_autostart_disable,
            commands::windows::windows_autostart_is_enabled,
            commands::windows::windows_drain_launch_requests,
            // M6(上) library commands
            commands::library::library_list_roots,
            commands::library::library_add_root,
            commands::library::library_remove_root,
            commands::library::library_set_root_enabled,
            commands::library::library_scan,
            commands::library::library_cancel_scan,
            commands::library::library_list_items,
            commands::library::library_get_item,
            commands::library::library_refresh_item,
            commands::library::library_remove_item,
            commands::library::library_delete_item,
            commands::library::library_reveal,
            // M4(上) playback commands
            commands::playback::create_playback_session,
            commands::playback::stop_playback_session,
            commands::playback::get_playback_info,
            commands::playback::get_active_playback_session,
            commands::playback::report_playback_position,
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
