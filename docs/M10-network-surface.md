# M10 Network Surface Inventory (§34)

All network communication Tide can perform, from actual code audit.
No other network code exists (verified by grep over src/ for socket/
reqwest/client usage outside these components).

| Surface | Protocol | Initiator | Default? | Destination | Purpose | User Data? | Security |
|---|---|---|---|---|---|---|---|
| HTTP Downloader | HTTP/HTTPS | Tide (M1 engine) | Only when user creates a task | User-provided URL (validated; private/loopback blocked by default) | Download | URL is user data — never logged | Scheme allowlist, per-hop redirect re-validation (bounded), private-IP block, checksum |
| Torrent peers | BitTorrent TCP/uTP | librqbit session | Only when a torrent task runs (added PAUSED by default; user resumes) | Peers for the task's swarm | Piece transfer | Info hash visible on the wire (inherent to BitTorrent) | DownloadOnly upload disabled; no UPnP (M3 §28); loopback listener option in tests |
| Trackers | HTTP/HTTPS/UDP | librqbit | Only for torrents whose metadata carries trackers | Tracker URLs from the user's .torrent/magnet | Peer discovery | Info hash + peer IP visible to tracker (inherent) | Tracker-less (DHT-only) magnets supported; DHT toggle exists (off in tests) |
| DHT | UDP | librqbit | On when torrent engine runs with dht:true (production default ON for magnet resolution) | DHT network | Peer discovery | Info hash (inherent to DHT) | librqbit boundary; no Tide-specific exposure |
| Stream Gateway | HTTP (localhost) | WebView2 `<video>` | Only while a playback session is open | 127.0.0.1:{random port} | Progressive range streaming | Validated Range requests; capability token in path | Loopback-only bind, UUIDv4 token, Host check, single-range, 256 KiB chunks, revoked on stop |
| Update checker | — | — | NOT PRESENT | — | — | — | No updater exists in M10 (§27: architecture decision documented, not implemented) |
| Telemetry/analytics | — | — | NOT PRESENT | — | — | — | §126: grep + runtime design audit confirm none |

## Default-launch connections (§35)

| Flow | Expected connections | Unexpected |
|---|---|---|
| App startup (no tasks) | NONE | any connection would be a violation |
| Task restore (paused) | NONE | — |
| Torrent engine start | DHT UDP + peer TCP for resumed torrents | — |
| Playback (complete local file) | NONE (file read direct) | — |
| Playback (in-progress download) | Origin HTTP range fetch / torrent peers only | — |
| App exit | NONE | — |
