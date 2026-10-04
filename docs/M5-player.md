# M5 Player Architecture

Answers M5 §96 (who does what) and §97 (lifecycles). The player did
NOT create a new media data path — it consumes the M4 one (§101).

## Who does what

```text
User
 ↓
Player UI (React: PlayerPage, Controls, Timeline, overlays)
 ↓  renders snapshots, calls controller methods — never the element
PlaybackController (frontend, the ONE element driver)
 ↓  HTMLMediaElement events → state machine → snapshot
PlaybackSession (backend: PlaybackManager)
 ↓  token → resource mapping, capability, availability, position
Stream Gateway (M4, unchanged)
 ↓  HTTP Range on 127.0.0.1:{random}
Resource.read(range) (M4: File / Part / Torrent / Http)
 ↓
Availability / DemandScheduler / download & torrent engines
```

| Layer | Responsible for | Explicitly NOT |
|---|---|---|
| React components | rendering snapshots, capturing user intent (clicks/keys/drags) | touching the element, computing byte offsets, polling loops |
| PlaybackController | driving the element, media-event → state mapping, error classification, bounded reload, metrics, position reporting | byte math (§19), network, backend policy |
| Backend session | authorization, capability, live availability, last position, owner invalidation | playback control (play/pause are element-native, §39) |
| Gateway/Resource | bytes, ranges, waits, security | anything UI |

Media bytes flow `WebView2 → loopback HTTP Range → gateway` — never
through IPC, never through a Blob (§59/§60). No MSE (§61); no native
player (M0.5 proved WebView2 sufficient).

## Player state machine (§9/§10)

`idle → loading → ready → (playing ⇄ paused/buffering/seeking) →
ended/error → disposed`, with the exact legal-transition table in
`state/playbackState.ts` and illegal transitions rejected. These are
PLAYBACK states — DownloadTask state is untouched (§9).

Two observation layers (§13): the machine is driven by REAL media
events; backend availability (polled at 1 s) only EXPLAINS a wait
("正在等待下载数据…" vs "缓冲中…") — it never overrides what the
media pipeline reports.

## Lifecycle (§97)

```text
OPEN (Play button)
 → create_playback_session(resource id)        [backend authorizes]
 → capability URL returned (token, runtime-only)
 → controller.load(url) → video.src
 → loadstart → loadedmetadata (state=ready, TTM recorded)
 → play() → playing (TTFF recorded)
 → waiting ⇄ playing (buffer/stall; backend explains)
 → seek → seeking → (range via browser) → seeked → playing
 → ended
 → Stop: stop_playback_session → gateway revoked, demands cancelled
   (task deletion additionally invalidates via owner, M4 §102)
```

Abnormal: `open → error → (terminal error UI | bounded reload
≤2, backoff 1s/3s, §34/§35)`. Any state → disposed (controller.dispose
removes every listener/timer, §87).

Source switch (§31): the app stops the previous session (gateway
revoked — proven by the source-switch test) before opening the next.

## Frontend reload (§37/§69)

React state is NOT the source of truth. On mount, App queries
`get_active_playback_session`; if the backend still holds a session,
PlayerPage adopts it and resumes at the session's last reported
position (player reports position every 5 s, §36 — session-scoped,
no watch history).

## Keyboard & controls

Space/K toggle, ←/→ ±5 s, ↑/↓ volume ±10 %, M mute, F fullscreen —
with an input/slider focus guard (§46). Controls auto-hide after 3 s
of mouse idle while playing (§45). Timeline seeks commit on
pointerup/Enter; drag is preview-only (§24).

## Fullscreen (§47)

Element Fullscreen API on the video surface (double-click or F or
button); fullscreenchange tracks state; Esc exits natively — verified
in the manual matrix.
