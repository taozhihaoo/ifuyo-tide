# M3 — Security Notes (Torrent)

## Path traversal / hostile metadata (M3 §12)

Every file path inside a torrent is validated BEFORE the torrent is
accepted (inspect, create, and recovery all funnel through
`validate_torrent_relative_path` / `validate_torrent_paths`):

- absolute paths, drive letters (`C:`), UNC (`\\`), device namespaces
  (`\\?\`) → rejected
- `..` traversal in any component → rejected
- per-component: empty, Windows reserved names (with or without
  extension — `NUL.txt` is still NUL), illegal characters
  (`< > : " | ? *`, control chars), trailing dots/spaces (Windows
  drops them silently — two torrent files could collide) → rejected
- length cap 400 chars per relative path

One hostile path rejects the WHOLE torrent (structured
`DOWNLOAD_INVALID_TORRENT_PATH`) — never a silent rename.

## Network listeners (M3 §26)

- No UPnP port forwarding (explicitly disabled at session creation).
- No HTTP API, no WebUI, no Prometheus endpoint from librqbit features
  (not enabled).
- Listener is a normal BitTorrent TCP listener on port 0 (OS-assigned);
  tests bind 127.0.0.1 only.

## Upload policy (M3 §28)

DownloadOnly is enforced by the `disable-upload` cargo feature plus
`SessionOptions.disable_upload: true` — compile-time and runtime
verifiable. The UI shows upload bytes at 0 and makes no anonymity or
no-upload claims beyond what the code guarantees.

## Logs (M3 §29/§38/§53)

Torrent logs carry task id / info hash / byte counts / reasons only —
never peer IPs, full magnet URIs, tracker URLs or user paths.

## Dependency audit (M3 §54)

librqbit 9.0.1 locked since M0.5; `disable-upload` and `stream`
features are the only additions on top of defaults; Apache-2.0; recorded
in THIRD_PARTY_NOTICES.md.
