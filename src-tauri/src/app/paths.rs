//! Application data directories.
//!
//! Windows convention: `%LOCALAPPDATA%\ifuyo\Tide\`.
//!
//! Application data (database, logs) is strictly separated from the
//! user's chosen download directory — app updates or uninstall flows
//! must never touch downloaded files.

use crate::error::TideError;
use std::path::PathBuf;

pub struct AppDirs {
    pub root: PathBuf,
    pub database_file: PathBuf,
    pub log_file: PathBuf,
    /// M3: stable copies of imported .torrent metadata files.
    pub metadata_dir: PathBuf,
}

impl AppDirs {
    pub fn resolve() -> Result<AppDirs, TideError> {
        let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
            TideError::Internal("LOCALAPPDATA environment variable is not set".into())
        })?;
        if local_app_data.trim().is_empty() {
            return Err(TideError::Internal(
                "LOCALAPPDATA environment variable is empty".into(),
            ));
        }
        let root = PathBuf::from(local_app_data).join("ifuyo").join("Tide");
        let database_dir = root.join("database");
        let logs_dir = root.join("logs");
        // M0 really creates the database and logs directories; the rest
        // (config/cache/sessions/temp) appears when actually needed.
        std::fs::create_dir_all(&database_dir)
            .map_err(|_| TideError::Internal("failed to create database directory".into()))?;
        std::fs::create_dir_all(&logs_dir)
            .map_err(|_| TideError::Internal("failed to create logs directory".into()))?;
        let metadata_dir = root.join("torrent-metadata");
        std::fs::create_dir_all(&metadata_dir)
            .map_err(|_| TideError::Internal("failed to create torrent metadata dir".into()))?;
        Ok(AppDirs {
            root,
            database_file: database_dir.join("tide.db"),
            log_file: logs_dir.join("tide.log"),
            metadata_dir,
        })
    }
}
