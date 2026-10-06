//! 1.1.1 — default public tracker list.
//!
//! Networks that filter DHT bootstrap traffic leave magnet-only tasks
//! without any peer source; well-known public trackers (HTTP/TCP
//! announce) restore peer discovery. Announcing to these servers is
//! standard BitTorrent behavior and is disclosed in PRIVACY.md; the
//! feature is a Settings toggle (default on).

pub const DEFAULT_PUBLIC_TRACKERS: &[&str] = &[
    "http://tracker.opentrackr.org:1337/announce",
    "https://tracker.tamersunion.org:443/announce",
    "udp://tracker.opentrackr.org:1337/announce",
    "http://tracker.files.fm:6969/announce",
    "udp://open.demonii.com:1333/announce",
];

pub fn default_trackers() -> Option<Vec<String>> {
    Some(
        DEFAULT_PUBLIC_TRACKERS
            .iter()
            .map(|t| t.to_string())
            .collect(),
    )
}
