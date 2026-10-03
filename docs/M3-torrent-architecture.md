# M3 — Torrent Architecture

```text
Tide Application (commands/torrent.rs)
        ↓ DTOs only
TorrentEngine (core/torrent/engine.rs)   ← single librqbit Session
        ↓
librqbit 9.0.1 (library form, no external process)
```

- ONE session per app process; many ManagedTorrent handles.
- State ownership (M3 §14): SQLite stores Tide's product-level task
  state (identity, intent, selection, errors); librqbit owns piece
  state, fast resume, peers, storage. Recovery (core/recovery +
  torrent/recovery.rs) reconciles the two at startup.
- librqbit types never cross IPC — commands convert to DTOs.
- Storage layout: user downloads live under the user-chosen directory
  (single-file torrents directly, multi-file under the torrent's root
  folder — librqbit's own layout); librqbit session state stays under
  `%LOCALAPPDATA%\ifuyo\Tide\`; imported `.torrent` copies under
  `torrent-metadata\<task-id>.torrent`.

## Upload / Sharing Policy (M3 §28)

DownloadOnly is a TECHNICAL guarantee: the librqbit `disable-upload`
cargo feature is enabled AND `SessionOptions.disable_upload: true` is
set at session creation. The UI therefore does not show an upload speed
control; uploaded_bytes stays 0 in stats (verified in the swarm tests).
The product does not claim anonymity.

## Network defaults (M3 §26)

- DHT: enabled for real use, disabled in tests (loopback seeding).
- Trackers: enabled (user-supplied via magnet/torrent metadata only —
  Tide ships no tracker list).
- UPnP port forwarding: explicitly disabled at session creation.
- uTP: left at librqbit default (TcpOnly listener).
- No HTTP API, no Prometheus, no WebUI, no proxy.
