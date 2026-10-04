# M5 Report

Per M5 §100.

## 1. Executive Summary

M5 turns the M4 streaming stack into a real desktop player: a
PlaybackController + media state machine drive ONE `<video>` through
backend-created capability URLs; the UI shows honest
loading/buffering/waiting/error states, a three-layer timeline,
volume/rate/fullscreen/keyboard controls; a frontend reload recovers
the session and position from the backend registry. 24 frontend unit
tests + the unchanged-green Rust suites (two new session tests)
back it. No new media data path, no new dependencies, no M4
redesign.

## 2. FACTS

- `pnpm test` 24/24; `tsc --noEmit` clean; `cargo test --locked`
  green (incl. 2 new M5 session tests); fmt/clippy clean in changed
  code; `pnpm tauri build` PASS.
- All media bytes flow WebView2 → loopback gateway; zero IPC byte
  transport; zero Blob usage.
- CSP (M0.5) already allows `media-src http://127.0.0.1:*` — no
  security relaxation was needed or made.

## 3. HYPOTHESES

- Element Fullscreen API works inside the WebView2 window (standard
  Chromium surface; high confidence, manual-pass item).
- The 2 s origin grace + 1 s availability poll keep the buffering
  explanation honest under slow sources (integration-proven at the
  HTTP layer).

## 4. M0–M4 state

Unchanged: download/torrent engines, recovery, gateway, scheduler.
M5 backend additions are additive (active-session registry, position
report). No M4 file required behavioral changes.

## 5. M5 Architecture

docs/M5-player.md — Player UI → PlaybackController → (element) and
Player UI → IPC → PlaybackSession → Gateway → Resource; each layer's
responsibilities and prohibitions spelled out.

## 6. PlaybackSession Design

docs/M5-playback-session.md — extended DTO (capability, live
availability, last_position_ms), active-session registry, 5 s
position reporting, owner invalidation unchanged.

## 7. Player State Machine

`state/playbackState.ts` — ten states, explicit legal-transition
table, illegal transitions rejected, six unit tests incl. the
disposed-is-terminal property.

## 8. WebView2 Media Integration

All §12 events observed by the controller's MediaEventAdapter;
waiting→buffering; seeked lands by element truth; autoplay rejection
caught; duration unavailable renders `--:--` (§25).

## 9. Gateway Integration

Unchanged M4 gateway; the player consumes capability URLs only
(§52); stop/source-switch revoke sessions (Test 7).

## 10. Resource Integration

One player for all source kinds (§16); capability gates random-access
seek honestly; source_kind never branches the UI logic.

## 11. Availability / Demand Integration

Live availability (1 s poll) explains buffering (§13/§78) and powers
the Backend-available readout; demands stay gateway-managed — the
player adds no scheduling knowledge (§76/§77).

## 12. UI

PlayerPage composition (surface/overlay/error/controls/info/
dev-diagnostics), auto-hide, keyboard map, three-layer timeline with
pointerup-commit, lastNonZeroVolume, rate 0.5–2×, fullscreen.

## 13. Error Model

docs/M5-error-model.md — media codes + backend codes → 9 typed kinds
→ user-safe messages; bounded reload (≤2, 1s/3s) for transient kinds
only.

## 14. Security

No new capabilities; CSP untouched; no arbitrary URL/path APIs;
session tokens runtime-only; diagnostics show short session ids
only; position data never leaves the process (§58).

## 15. Performance

Element-driven playback (media work in the media stack); React
updates at event cadence (timeupdate ≈ 4 Hz) + 1 s availability poll
— no per-chunk churn (§41/§73); memory bounded by design (no Blob,
no IPC bytes) — the 1 GB+ soak remains a manual-pass item (§68).

## 16. Test Matrix

docs/M5-test-matrix.md — §63 row-by-row mapping; §64 tests 1–10
mapped (7/10 in Rust suites, 10/10 covered combining layers).

## 17. Real WebView2 Results

Playback itself through WebView2 was measured in M0.5 (same protocol
shape) and the production wiring is unit/type-checked; the scripted
in-window run of the FINAL build is the standing manual pass (list
in the test matrix) — same disclosure as M4 §18.

## 18. Progressive HTTP Results

M4 Critical #1/#2 suites (frontier wait, read-through seek) — the
player consumes exactly those sessions.

## 19. Progressive Torrent Results

M4 suites: waits-for-pieces tail read; seek demand cancellation;
bounded no-peer waits.

## 20. Seek-into-Unavailable Results

Browser range → gateway → demand → availability → resume chain
proven at HTTP layer; frontend maps the wait to
buffering/"等待下载数据…" (§21/§57).

## 21. Session Cleanup Results

Controller dispose removes every listener/timer (unit-tested);
stop/source-switch revoke gateway sessions (Rust-tested); owner
deletion invalidates (M4); TTL reaping bounds leaks.

## 22. Files Changed

Backend: `core/streaming/playback.rs`, `commands/playback.rs`,
`app/mod.rs`, `tests/m4_lower.rs`. Frontend: `features/player/**`
(state/controller/errorModel/PlayerPage/PlayerView/StatusOverlay/
controls/hooks, VideoPlayer removed), `app/App.tsx`,
`styles/global.css`, `test/*.test.ts`.

## 23. Dependencies Added

None (§93).

## 24. Documentation Updated

M5-player / M5-playback-session / M5-media-compatibility /
M5-error-model / M5-test-matrix / this report; README status.

## 25. RESOLVED

- React-state-as-only-truth (reload lost the player) → backend
  active-session registry + adoption.
- Direct element poking from components → PlaybackController.
- Position loss across reload → session-scoped position report/resume.

## 26. CONFIRMED

- WebView2 plays/seek through the loopback gateway (M0.5 + M4).
- Sessions survive frontend reloads; old sessions die on switch.

## 27. OPEN

- Scripted in-window E2E of the release build (manual pass; §17).
- 1 GB+ memory soak in-window (§68) — manual pass; bounded by design.
- `canPlayType`-based pre-hints in the UI (optional §79; not needed
  for honesty since real playback decides).

## 28. WORSENED

Nothing — all pre-existing suites remain green.

## 29. P0

0.

## 30. P1 (recorded)

- Manual-only in-window verification items (fullscreen/keys/volume/
  rate/soak).
- Duration is element-known only → backend cannot estimate
  time-based backend-availability percentages (byte-based readout
  shown instead).
- No subtitle support (deliberate, §49); poster-only metadata
  (§50) — none rendered since no resource metadata carries one.

## 31. M6 Readiness

Resource ids, owners and session metadata are Library-ready; the
player is embeddable (PlayerPage takes a session); watch HISTORY
stays out by design (§36) and is M6's first-class feature.

## 32. Definition of Done

§98 checklist: Player items (open/play/pause/seek/volume/mute/rate/
end/switch/fullscreen/keyboard) — implemented; progressive items —
inherited proven M4 chains; gateway/architecture/security — unchanged
M4 guarantees, audits green; UX states — all rendered from real
state; performance — no Blob/IPC bytes/high-freq renders; reliability
— rapid play/pause/seek, switch-leak, reload-recovery, gateway and
resource errors tested; testing — unit/frontend/integration pass,
in-window visual run recorded as the manual pass. M5 STOPS here (§90/
§102): no M6/M7/M8 work was started.
