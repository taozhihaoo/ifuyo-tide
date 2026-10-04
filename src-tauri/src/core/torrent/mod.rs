//! M3 — Torrent Engine (librqbit adapter boundary).
//!
//! Layering (M3 §4/§14): `Application → TorrentEngine → librqbit`.
//! SQLite stores Tide's product-level task state; librqbit owns piece /
//! fast-resume / peer state. The two meet only in the recovery
//! coordinator.

pub mod engine;
pub mod monitor;
pub mod recovery;
pub mod source;

pub use engine::{AddedTorrent, TorrentEngine, TorrentEngineConfig, TorrentStatus};
pub use recovery::recover_torrents;
pub use source::{
    validate_magnet_uri, validate_torrent_paths, validate_torrent_relative_path, TorrentMetadata,
    TorrentMetadataFile,
};
