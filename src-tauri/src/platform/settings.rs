//! M8 Windows-integration settings (§80): close-to-tray behaviour and
//! notification toggles — persisted via the EXISTING settings KV
//! (§80: no second configuration store). Autostart itself is managed
//! by the autostart plugin; only its DESIRED state is persisted here
//! so the Settings UI can reflect it.

use crate::error::TideError;
use crate::persistence::repositories::SettingsRepository;
use rusqlite::Connection;
use serde::Serialize;

const KEY_CLOSE_TO_TRAY: &str = "win:close_to_tray";
const KEY_NOTIFY_COMPLETED: &str = "win:notify_download_completed";
const KEY_NOTIFY_FAILED: &str = "win:notify_download_failed";
const KEY_NOTIFY_TORRENT: &str = "win:notify_torrent_completed";

/// Defaults (§15.3: notifications must be disableable; §16.1:
/// autostart default OFF — autostart itself lives in the plugin).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsSettings {
    pub close_to_tray: bool,
    pub notify_download_completed: bool,
    pub notify_download_failed: bool,
    pub notify_torrent_completed: bool,
}

impl Default for WindowsSettings {
    fn default() -> Self {
        WindowsSettings {
            close_to_tray: true,
            notify_download_completed: true,
            notify_download_failed: true,
            notify_torrent_completed: true,
        }
    }
}

fn get_flag(conn: &Connection, key: &str, default: bool) -> bool {
    match SettingsRepository.get(conn, key) {
        Ok(Some(value)) => value == "1",
        _ => default,
    }
}

pub fn load(conn: &Connection) -> Result<WindowsSettings, TideError> {
    Ok(WindowsSettings {
        close_to_tray: get_flag(conn, KEY_CLOSE_TO_TRAY, true),
        notify_download_completed: get_flag(conn, KEY_NOTIFY_COMPLETED, true),
        notify_download_failed: get_flag(conn, KEY_NOTIFY_FAILED, true),
        notify_torrent_completed: get_flag(conn, KEY_NOTIFY_TORRENT, true),
    })
}

pub fn store(conn: &Connection, settings: &WindowsSettings) -> Result<(), TideError> {
    let repo = SettingsRepository;
    repo.set(conn, KEY_CLOSE_TO_TRAY, if settings.close_to_tray { "1" } else { "0" })?;
    repo.set(
        conn,
        KEY_NOTIFY_COMPLETED,
        if settings.notify_download_completed { "1" } else { "0" },
    )?;
    repo.set(
        conn,
        KEY_NOTIFY_FAILED,
        if settings.notify_download_failed { "1" } else { "0" },
    )?;
    repo.set(
        conn,
        KEY_NOTIFY_TORRENT,
        if settings.notify_torrent_completed { "1" } else { "0" },
    )?;
    Ok(())
}
