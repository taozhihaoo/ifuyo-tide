//! Minimal structured logging for M0.
//!
//! - JSON-lines format, UTC epoch-millisecond timestamps.
//! - Production logs never contain URLs, magnets, file paths, peer IPs or
//!   user-provided names; only ids, event names and technical fields.
//! - Default level is `info` in release builds and `debug` in dev builds.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
        }
    }
}

struct Logger {
    file: Mutex<Option<File>>,
    min_level: Level,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initializes the global logger. Safe to call once during app startup.
pub fn init(log_path: &Path, min_level: Level) -> Result<(), crate::error::TideError> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| {
            crate::error::TideError::Internal("failed to create logs directory".into())
        })?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|_| crate::error::TideError::Internal("failed to open log file".into()))?;
    let _ = LOGGER.set(Logger {
        file: Mutex::new(Some(file)),
        min_level,
    });
    Ok(())
}

/// Emits one structured log line.
pub fn log(level: Level, event: &str, fields: &[(&str, &str)]) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    if level > logger.min_level {
        return;
    }
    let mut record = serde_json::json!({
        "ts": crate::core::time::now_unix_ms(),
        "level": level.as_str(),
        "event": event,
    });
    if let Some(obj) = record.as_object_mut() {
        for (key, value) in fields {
            obj.insert((*key).to_string(), serde_json::json!(value));
        }
    }
    let line = record.to_string();
    if let Ok(mut guard) = logger.file.lock() {
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }
    // Mirror to stderr so `tauri dev` consoles show lifecycle events.
    eprintln!("{line}");
}

pub fn error(event: &str, fields: &[(&str, &str)]) {
    log(Level::Error, event, fields);
}

pub fn warn(event: &str, fields: &[(&str, &str)]) {
    log(Level::Warn, event, fields);
}

pub fn info(event: &str, fields: &[(&str, &str)]) {
    log(Level::Info, event, fields);
}

pub fn debug(event: &str, fields: &[(&str, &str)]) {
    log(Level::Debug, event, fields);
}

/// Default level by build type: debug in dev, info in production.
pub fn default_level() -> Level {
    if cfg!(debug_assertions) {
        Level::Debug
    } else {
        Level::Info
    }
}
