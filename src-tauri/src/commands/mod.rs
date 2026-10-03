//! Tauri IPC command layer.
//!
//! Every command follows the same path:
//! `validate (security) → domain → repository → DTO`.
//!
//! Errors never panic across IPC: they are structured `{ code, message }`
//! values, and every error is recorded in the diagnostics log without any
//! user-provided payload.

pub mod app_info;
pub mod settings;
pub mod spike;
pub mod tasks;

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
