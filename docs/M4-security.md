# M4 Security

The streaming layer's security posture (M4(下) §100–§107/§140/§185),
all of it enforced in code and proven by tests.

## Arbitrary file read (§100) — P0, denied by construction

The gateway has no path→file mapping at all. A request can only name
an opaque session token; tokens resolve to resources the backend
registered. There is no code path from an HTTP request to
`open(path)`. The matrix test hits every shape the spec lists —
`../../secret`, `..\..\secret`, `%2e%2e`, `%252e%252e`, `C:\`, `C:/`,
`\\server\share`, `\\?\C:\`, `file://` (all percent-encoded), drive
and UNC spellings, `....//`, query/hash smuggling — every one is 403
or 404 with an EMPTY body.

## Tokens (§101) — P0

UUIDv4 (122 random bits), runtime-only, never persisted (§70), never
logged (§185 — logs carry an 8-char session short id only). Expired,
closed, deleted-task, and guessed tokens all yield 404/denied. Close
additionally signals in-flight reads to abort.

## Resource isolation (§103)

| Root | Path | Who touches it |
|---|---|---|
| AppDataRoot | `%LOCALAPPDATA%\ifuyo\Tide\` | db, logs, torrent metadata (M2/M3) |
| DownloadRoot | user-chosen destination dirs | download engine + resources created FROM persisted records |
| TempRoot | OS temp (tests/fixtures) | self-generated test media only |

The gateway reaches ONLY `StreamingResource`s constructed by the
backend from persisted records (ResourceId → trusted mapping →
allowed storage, §60). No request-supplied path is ever opened.

## Task deletion (§102)

Chosen policy: **session invalidated**. `delete_download`,
`remove_torrent`, and `delete_torrent_data` call
`PlaybackManager::close_for_owner(task_id)`, which revokes gateway
access and cancels demands. No dangling path, no session outliving
its data. (The alternative "let the current session finish" was
rejected: it risks serving a half-deleted file.)

## LAN exposure & methods (§125)

Bind is `127.0.0.1:0` — the listener is unreachable off-host. The
Host header must be loopback (403 otherwise), which also kills
DNS-rebinding and cross-origin probing. Only GET/HEAD are served;
anything else is `405` + `Allow: GET, HEAD`.

## Tauri capabilities (§104)

Unchanged from M0 — `core:default` + `dialog:allow-open` only. No
filesystem/shell/http/process permissions were added for the gateway
or the player (audited; the gateway needs none: it is server-side).

## CORS (§58)

No CORS headers. WebView2 loads the loopback URL directly in the
media element; M0.5 measured that no CORS accommodation is needed.

## Log safety (§140/§185)

Never logged: full magnet, full URL, absolute local paths, peer IPs,
info-hash, gateway token, full query strings. Logged (low
cardinality, safe fields): `gateway.started/stopped` (port),
`gateway.request` (session short id, status, byte count, duration),
`playback.opened/closed/invalidated` (session id, source kind),
`streaming.range_unsupported`. The M4 code was audited against this
list.

## Error surface (§98/§99)

Structured `{ code, message }` only — the streaming taxonomy
(PLAYBACK_ERROR, GATEWAY_ERROR, RESOURCE_UNAVAILABLE,
RESOURCE_RANGE_UNSUPPORTED, DEMAND_ERROR, SCHEDULER_ERROR,
RANGE_ERROR, NETWORK_TIMEOUT, NETWORK_CONNECT, HTTP_STATUS,
WAIT_TIMEOUT) plus the human-safe mapping in the frontend. No panic
strings, no rqbit debug output, no OS error text reach the UI.
