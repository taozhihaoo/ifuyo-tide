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
    #[error("{0}")]
    Internal(String),
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
    fn from(_err: rusqlite::Error) -> Self {
        // SQLite error details stay in backend logs; the frontend only
        // receives the generic persistence code.
        TideError::Persistence("database operation failed".to_string())
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
