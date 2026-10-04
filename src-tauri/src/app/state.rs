//! Unified application state, injected into Tauri commands.
//!
//! No global mutable singletons: the state is constructed once at startup
//! and handed to commands explicitly through Tauri's managed state.

use crate::core::download::DownloadEngine;
use crate::core::streaming::playback::PlaybackManager;

use crate::error::TideError;
use crate::persistence::repositories::{
    SettingsRepository, SourceRepository, TaskRepository, TorrentRepository,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct AppState {
    db: Mutex<Connection>,
    pub task_repository: TaskRepository,
    pub source_repository: SourceRepository,
    pub settings_repository: SettingsRepository,
    /// M1: HTTP download engine (dependency-injected, M1 §63/§64 —
    /// no global singleton).
    pub download_engine: DownloadEngine,
    /// M3: torrent engine (librqbit adapter) + its repository.
    pub torrent_engine: crate::core::torrent::TorrentEngine,
    pub torrent_repository: Arc<TorrentRepository>,
    /// M4(上): playback sessions + the streaming gateway.
    pub playback: Arc<PlaybackManager>,
    /// M6(上): the local library (roots, items, scans).
    pub library: Arc<crate::core::library::LibraryService>,
    /// M7(上): the scheduler runtime + its registries.
    pub scheduler: Arc<crate::core::scheduler::SchedulerRuntime>,
    pub user_priorities:
        Arc<Mutex<std::collections::HashMap<String, crate::core::scheduler::UserPriority>>>,
    pub global_bandwidth_bps: Arc<std::sync::atomic::AtomicU64>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        conn: Connection,
        download_engine: DownloadEngine,
        torrent_engine: crate::core::torrent::TorrentEngine,
        torrent_repository: Arc<TorrentRepository>,
        playback: Arc<PlaybackManager>,
        library: Arc<crate::core::library::LibraryService>,
        scheduler: Arc<crate::core::scheduler::SchedulerRuntime>,
        user_priorities: Arc<
            Mutex<std::collections::HashMap<String, crate::core::scheduler::UserPriority>>,
        >,
        global_bandwidth_bps: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        AppState {
            db: Mutex::new(conn),
            task_repository: TaskRepository,
            source_repository: SourceRepository,
            settings_repository: SettingsRepository,
            download_engine,
            torrent_engine,
            torrent_repository,
            playback,
            library,
            scheduler,
            user_priorities,
            global_bandwidth_bps,
        }
    }

    /// Runs `f` with a shared connection reference.
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, TideError>,
    ) -> Result<T, TideError> {
        let conn = self
            .db
            .lock()
            .map_err(|_| TideError::Internal("database lock poisoned".into()))?;
        f(&conn)
    }

    /// Runs `f` inside a write transaction; commits on success and lets
    /// the transaction roll back automatically on error.
    pub fn with_tx<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, TideError>,
    ) -> Result<T, TideError> {
        let mut conn = self
            .db
            .lock()
            .map_err(|_| TideError::Internal("database lock poisoned".into()))?;
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }
}
