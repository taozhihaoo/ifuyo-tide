//! M6(上) — Local Library.
//!
//! Components: model (roots/items/queries), path (Windows path
//! safety — P0), classifier (extension → category/MIME, §77),
//! repository (all SQL, batch/transactional scans), scanner
//! (filesystem enumeration, partial-scan safe), service (validation
//! + lifecycle policies; the only IPC-facing piece).
//!
//! Boundaries kept (§3): Task / Resource / LibraryItem / Playback
//! stay four concepts; the library never talks HTTP, torrents, or the
//! gateway; playback goes LibraryItem id → validated canonical path →
//! M4/M5 PlaybackSession.

pub mod classifier;
pub mod model;
pub mod path;
pub mod reconciler;
pub mod repository;
pub mod scanner;
pub mod service;

pub use model::{
    AvailabilityStatus, ItemPage, ItemQuery, LibraryItem, LibraryRoot, MediaCategory, OriginType,
};
pub use repository::LibraryRepository;
pub use scanner::{ScanOutcome, ScanStatus};
pub use service::LibraryService;
