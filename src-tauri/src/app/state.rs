//! Unified application state, injected into Tauri commands.
//!
//! No global mutable singletons: the state is constructed once at startup
//! and handed to commands explicitly through Tauri's managed state.

use crate::core::download::DownloadEngine;
use crate::error::TideError;
use crate::persistence::repositories::{SettingsRepository, SourceRepository, TaskRepository};
use rusqlite::Connection;
use std::sync::Mutex;

pub struct AppState {
    db: Mutex<Connection>,
    pub task_repository: TaskRepository,
    pub source_repository: SourceRepository,
    pub settings_repository: SettingsRepository,
    /// M1: HTTP download engine (dependency-injected, M1 §63/§64 —
    /// no global singleton).
    pub download_engine: DownloadEngine,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        AppState {
            db: Mutex::new(conn),
            task_repository: TaskRepository,
            source_repository: SourceRepository,
            settings_repository: SettingsRepository,
            download_engine: DownloadEngine::new(),
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
