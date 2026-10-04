use serde::ser::{Serialize, Serializer};

/// Unified error type for all Tide runtime paths.
///
/// IPC boundaries serialize this as `{ code, message }`; raw internals
/// (Rust debug output, SQLite messages) must never reach the frontend.
#[derive(Debug, thiserror::Error)]
pub enum TideError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Persistence(String),
    #[error("illegal state transition from {from} to {to}")]
    StateTransition { from: String, to: String },
    /// A security boundary was crossed (path escape, bad host, bad
    /// token). Never leaks details to the frontend beyond the code.
    #[error("{0}")]
    Security(String),
    /// Domain failures of the download engine (M1 §45) — one wrapper
    /// variant instead of a second error type.
    #[error(transparent)]
    Download(#[from] crate::core::download::DownloadFailure),
    /// Streaming failures (M4(下) §98): playback, gateway, resource,
    /// demand, scheduler, range, timeout — each a stable code, never a
    /// raw internal string.
    #[error(transparent)]
    Streaming(#[from] StreamingFailure),
    #[error("{0}")]
    Internal(String),
}

/// Streaming-layer failure taxonomy (M4(下) §98/§143). Every variant
/// maps to a stable code; messages stay human-safe.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StreamingFailure {
    #[error("playback failed: {0}")]
    Playback(String),
    #[error("gateway request rejected: {0}")]
    Gateway(String),
    #[error("resource unavailable: {0}")]
    Resource(String),
    #[error("resource does not support range requests")]
    RangeUnsupported,
    #[error("demand rejected: {0}")]
    Demand(String),
    #[error("scheduler failure: {0}")]
    Scheduler(String),
    #[error("invalid range: {0}")]
    Range(String),
    #[error("network timeout while waiting for data")]
    NetworkTimeout,
    /// Connection-level failure (reset/refused/DNS) — one code, details
    /// stay in logs.
    #[error("connection to the source failed")]
    NetworkConnect,
    #[error("source returned HTTP {0}")]
    HttpStatus(u16),
    #[error("timed out waiting for data to become available")]
    WaitTimeout,
}

impl StreamingFailure {
    pub fn code(&self) -> &'static str {
        match self {
            StreamingFailure::Playback(_) => "PLAYBACK_ERROR",
            StreamingFailure::Gateway(_) => "GATEWAY_ERROR",
            StreamingFailure::Resource(_) => "RESOURCE_UNAVAILABLE",
            StreamingFailure::RangeUnsupported => "RESOURCE_RANGE_UNSUPPORTED",
            StreamingFailure::Demand(_) => "DEMAND_ERROR",
            StreamingFailure::Scheduler(_) => "SCHEDULER_ERROR",
            StreamingFailure::Range(_) => "RANGE_ERROR",
            StreamingFailure::NetworkTimeout => "NETWORK_TIMEOUT",
            StreamingFailure::NetworkConnect => "NETWORK_CONNECT",
            StreamingFailure::HttpStatus(_) => "HTTP_STATUS",
            StreamingFailure::WaitTimeout => "WAIT_TIMEOUT",
        }
    }
}

impl TideError {
    /// Stable machine-readable error code exposed to the frontend.
    pub fn code(&self) -> &'static str {
        match self {
            TideError::InvalidInput(_) => "INVALID_INPUT",
            TideError::NotFound(_) => "NOT_FOUND",
            TideError::Persistence(_) => "PERSISTENCE",
            TideError::StateTransition { .. } => "STATE_TRANSITION",
            TideError::Security(_) => "SECURITY",
            TideError::Download(failure) => failure.code(),
            TideError::Streaming(failure) => failure.code(),
            TideError::Internal(_) => "INTERNAL",
        }
    }

    pub fn message(&self) -> String {
        match self {
            TideError::StateTransition { from, to } => {
                format!("illegal state transition from {from} to {to}")
            }
            other => other.to_string(),
        }
    }
}

impl From<rusqlite::Error> for TideError {
    fn from(err: rusqlite::Error) -> Self {
        // SQLite error details stay in backend logs; the frontend only
        // receives the generic persistence code plus the numeric error
        // code (no SQL text, no schema details).
        let code = err
            .sqlite_error_code()
            .map(|c| format!("{:?}", c))
            .unwrap_or_default();
        eprintln!("SQLITE_ERR: {code}: {err}");
        TideError::Persistence(format!("database operation failed ({code})"))
    }
}

impl From<std::io::Error> for TideError {
    fn from(_err: std::io::Error) -> Self {
        TideError::Persistence("filesystem operation failed".to_string())
    }
}

impl From<serde_json::Error> for TideError {
    fn from(_err: serde_json::Error) -> Self {
        TideError::Internal("serialization failed".to_string())
    }
}

impl Serialize for TideError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TideError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.message())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(TideError::InvalidInput("x".into()).code(), "INVALID_INPUT");
        assert_eq!(TideError::NotFound("x".into()).code(), "NOT_FOUND");
        assert_eq!(TideError::Persistence("x".into()).code(), "PERSISTENCE");
        assert_eq!(TideError::Security("x".into()).code(), "SECURITY");
        assert_eq!(TideError::Internal("x".into()).code(), "INTERNAL");
        assert_eq!(
            TideError::StateTransition {
                from: "completed".into(),
                to: "running".into()
            }
            .code(),
            "STATE_TRANSITION"
        );
        // M4(下) §98 streaming taxonomy codes are stable.
        use StreamingFailure as S;
        let cases: [(S, &str); 11] = [
            (S::Playback("x".into()), "PLAYBACK_ERROR"),
            (S::Gateway("x".into()), "GATEWAY_ERROR"),
            (S::Resource("x".into()), "RESOURCE_UNAVAILABLE"),
            (S::RangeUnsupported, "RESOURCE_RANGE_UNSUPPORTED"),
            (S::Demand("x".into()), "DEMAND_ERROR"),
            (S::Scheduler("x".into()), "SCHEDULER_ERROR"),
            (S::Range("x".into()), "RANGE_ERROR"),
            (S::NetworkTimeout, "NETWORK_TIMEOUT"),
            (S::NetworkConnect, "NETWORK_CONNECT"),
            (S::HttpStatus(503), "HTTP_STATUS"),
            (S::WaitTimeout, "WAIT_TIMEOUT"),
        ];
        for (failure, code) in cases {
            assert_eq!(TideError::Streaming(failure).code(), code);
        }
    }

    #[test]
    fn state_transition_message_has_no_backtrace_details() {
        let err = TideError::StateTransition {
            from: "completed".into(),
            to: "running".into(),
        };
        assert_eq!(
            err.message(),
            "illegal state transition from completed to running"
        );
    }
}
