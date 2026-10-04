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
    // Fail-closed stubs: every debug command either reports a Security
    // error or the conservative default. Return shapes are fixed per
    // command kind (Result vs bool) — a uniform macro body cannot span
    // both, which is exactly the drift that breaks release builds.
    macro_rules! disabled_err {
        ($($name:ident),* $(,)?) => {
            $(
                #[tauri::command]
                pub fn $name() -> Result<(), TideError> {
                    Err(TideError::Security(
                        "debug commands are disabled in release builds".into(),
                    ))
                }
            )*
        };
    }
    disabled_err![
        spike_status,
        spike_list_media,
        spike_start_fixture_server,
        spike_fixture_slow_ms,
        spike_start_torrent,
        spike_torrent_status,
        spike_pause_torrent,
        spike_resume_torrent,
        spike_open_playback_session,
        spike_close_playback_session,
        spike_gateway_slow_ms,
        spike_gateway_read_timeout_ms,
        spike_events,
        spike_clear_events,
        spike_report_media_result,
        spike_read_media_results,
        spike_shutdown,
    ];

    #[tauri::command]
    pub fn spike_autorun_enabled() -> bool {
        false
    }
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
