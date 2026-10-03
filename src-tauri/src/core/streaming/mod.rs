//! M4(上) — Progressive Streaming core.
//!
//! Components (M4(上) §6/§37/§51/§71/§75):
//! - `resource`: unified `StreamingResource` (File / Http / Torrent)
//! - `demand`: Demand model + minimal priority scheduler
//! - `gateway`: the ONE loopback Range gateway (promoted from M0.5 spike)
//! - `playback`: PlaybackSession manager (separate from DownloadTask)

pub mod demand;
pub mod gateway;
pub mod playback;
pub mod range;
pub mod resource;

pub use demand::{Demand, DemandId, DemandPriority, DemandReason, DemandScheduler, DemandState};
pub use gateway::StreamingGateway;
pub use playback::{PlaybackManager, PlaybackSessionInfo};
pub use range::{resolve_range_header, RangeResolution};
pub use resource::{Availability, StreamingResource};

/// Response Content-Type from a file name. Labels only — the player
/// capability matrix comes from the M0.5 facts (M4(上) §63); unknown
/// extensions stay `application/octet-stream` so the player decides.
pub fn content_type_for_name(name: &str) -> &'static str {
    let ext = name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "ogg" | "opus" => "audio/ogg",
        "ts" | "m2ts" => "video/mp2t",
        _ => "application/octet-stream",
    }
}
