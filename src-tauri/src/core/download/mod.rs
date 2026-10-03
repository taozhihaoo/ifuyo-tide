//! M1 — HTTP Download Core.
//!
//! Layering (M1 §3.1):
//! `Command → DownloadEngine → HttpTransport → FileSink`
//!
//! The engine owns state/resume/retry/progress/cancellation; the
//! transport owns HTTP; the sink owns bytes on disk. UI consumes
//! DTOs and events only.

pub mod checksum;
pub mod engine;
pub mod file_sink;
pub mod filename;
pub mod http_range;
pub mod progress;
pub mod request;
pub mod state;
pub mod transport;
pub mod url;
pub mod validator;

pub use engine::{DownloadEngine, DownloadShared, DownloadStatus};
pub use request::DownloadRequest;
pub use state::{DownloadLifecycle, DownloadPhase};
pub use transport::HttpTransport;

pub use crate::persistence::repositories::{DownloadRepository, DownloadTaskRecord};
pub use url::DownloadFailure;
