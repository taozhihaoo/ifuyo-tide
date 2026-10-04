# M5 Error Model

The four error sources (M5 §56) and how they become honest UI.

## Sources and mapping

| Source | Signal | → Player error kind |
|---|---|---|
| Media decoder | `MediaError` code | 4 → UnsupportedMedia, 3 → DecodeError, 2 → NetworkError, 1/other → UnknownMediaError |
| Backend (M4 §98 codes) | IPC `{code,message}` | RESOURCE_UNAVAILABLE→ResourceUnavailable, RESOURCE_RANGE_UNSUPPORTED→UnsupportedMedia, NETWORK_TIMEOUT/WAIT_TIMEOUT→PlaybackTimeout, NETWORK_CONNECT/HTTP_STATUS→NetworkError, GATEWAY_ERROR/PLAYBACK_ERROR/RANGE_ERROR/DEMAND_ERROR/SCHEDULER_ERROR→GatewayError, NOT_FOUND→SessionExpired |
| Session lifecycle | unknown/expired session id | SessionExpired |
| Gateway transport | early close / 404 on stream URL | ResourceClosed / SessionExpired |

Each `PlaybackError` carries safe diagnostic context (§33):
`kind, message, recoverable, mediaErrorCode?, backendCode?, sessionId?,
contentType?, playerState?, timestamp`. NEVER: full magnets, URLs,
peer IPs, tokens, local paths.

## Recovery (§34/§35)

- `NetworkError`/`PlaybackTimeout` are transient → the controller
  auto-reloads with backoff (1 s, 3 s), AT MOST 2 times
  (`MAX_RELOADS`); the reload is guarded by state (no reload loops,
  no reload after dispose, no reload across a source switch).
- Everything else (decode/unsupported/expired/deleted) is terminal:
  the error overlay shows the message + "关闭播放器"; a Retry button
  appears only for recoverable kinds.
- A rejected `play()` (autoplay policy) is caught and mapped to
  `paused` — never an unhandled rejection (§28).

## UI text (§32/§99)

User-safe Chinese messages per kind, defined in
`errorModel.ts` (`USER_MESSAGES`) — e.g.
"此媒体格式当前环境无法播放。" — no error numbers, no internals.
Raw context still lands in the local diagnostics panel (dev builds)
and backend logs (redacted per M4 §140/§185).
