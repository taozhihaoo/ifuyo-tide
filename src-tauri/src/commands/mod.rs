//! Tauri IPC command layer.
//!
//! Every command follows the same path:
//! `validate (security) → domain → repository → DTO`.
//!
//! Errors never panic across IPC: they are structured `{ code, message }`
//! values, and every error is recorded in the diagnostics log without any
//! user-provided payload.

pub mod app_info;
pub mod download;
pub mod library;
pub mod playback;
pub mod scheduler;
pub mod settings;
// M10 §23/§52: the M0.5 spike command surface is development-only.
// Release builds compile fail-closed stubs under the SAME names, so
// the invoke_handler registration list stays identical while the
// production IPC surface cannot invoke a single spike command.
#[cfg(debug_assertions)]
pub mod spike;
#[cfg(not(debug_assertions))]
pub mod spike {
    use crate::error::TideError;
    macro_rules! disabled {
        ($($name:ident => $ret:ty),* $(,)?) => {
            $(
                #[tauri::command]
                pub fn $name() -> $ret {
                    Err(TideError::Security(
                        "debug commands are disabled in release builds".into(),
                    ))
                }
            )*
        };
    }
    disabled![
        spike_status => Result<(), TideError>,
        spike_list_media => Result<(), TideError>,
        spike_start_fixture_server => Result<(), TideError>,
        spike_fixture_slow_ms => Result<(), TideError>,
        spike_start_torrent => Result<(), TideError>,
        spike_torrent_status => Result<(), TideError>,
        spike_pause_torrent => Result<(), TideError>,
        spike_resume_torrent => Result<(), TideError>,
        spike_open_playback_session => Result<(), TideError>,
        spike_close_playback_session => Result<(), TideError>,
        spike_gateway_slow_ms => Result<(), TideError>,
        spike_gateway_read_timeout_ms => Result<(), TideError>,
        spike_events => Result<(), TideError>,
        spike_clear_events => Result<(), TideError>,
        spike_report_media_result => Result<(), TideError>,
        spike_read_media_results => Result<(), TideError>,
        spike_autorun_enabled => bool,
        spike_shutdown => Result<(), TideError>,
    ];
}
pub mod tasks;
pub mod torrent;
pub mod windows;

use crate::diagnostics;
use crate::error::TideError;

/// Logs an IPC failure (command name + stable code only, no payloads).
pub fn log_ipc_error(command: &str, err: &TideError) {
    diagnostics::warn("ipc.error", &[("command", command), ("code", err.code())]);
}

/// Wraps a command result with error logging.
pub fn ipc_result<T>(command: &'static str, result: Result<T, TideError>) -> Result<T, TideError> {
    if let Err(err) = &result {
        log_ipc_error(command, err);
    }
    result
}
