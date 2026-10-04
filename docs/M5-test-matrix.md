# M5 Test Matrix

## Automated results

- Frontend: **24 tests, 0 failures** (`pnpm test`): App shell 3 +
  state-machine 6 + controller/error-model 15 (mock media element).
- `tsc --noEmit` clean.
- Rust: full suite green (M4 suites unchanged and passing — the
  backend additions are covered by the two new M5 session tests).

## §63 player matrix — how each row is covered

| Scenario (§63) | Coverage |
|---|---|
| MP4/H264/AAC, WebM/VP9 play/pause/seek | M0.5 measured (real WebView2) + M4 wire-level suites behind the same gateway |
| unsupported codec → error UI | controller test: media error 4 → UnsupportedMedia, no auto-reload |
| audio | same element pipeline (video element plays audio; no separate `<audio>` needed) |
| zero/unknown duration | Timeline/TimeDisplay render `--:--`; seek disabled honestly |
| corrupt media → error | DecodeError classification (unit) |
| progressive HTTP | M4 Critical #1 + this milestone's adoption path |
| progressive Torrent | M4 Critical #3/#4 suites |
| seek unavailable region → buffering | controller waiting→buffering mapping + M4 Critical #2 |
| slow resource → buffering | M4 frontier-wait suites |
| network interruption → recover/error | bounded-reload controller test (1s/3s, cap 2) |
| resource closed → graceful | M4 503/close_for_owner suites + ResourceUnavailable mapping |
| session expired → graceful | SessionExpired mapping (NOT_FOUND) |
| source switch → old session released | `source_switch_releases_old_session` (Rust) |
| app UI reload → session recover | `frontend_reload_recovers_active_session_projection` (Rust) + App adopt path |
| fullscreen enter/exit | element Fullscreen API + fullscreenchange (manual verification item) |
| keyboard | `useKeyboardShortcuts` (Space/arrows/M/F, input-conflict guard) — manual verification item |
| volume mute/unmute | lastNonZeroVolume logic in PlayerControls + controller setVolume/setMuted (manual verification item) |
| playback rate | element playbackRate via controller (manual verification item) |

## §64 key integration tests (mapping)

| Test | Where |
|---|---|
| 1 completed local resource → play | m4_streaming File-resource suite |
| 2 HTTP downloading → play | m4_lower Critical #1 |
| 3 Torrent partial → play | m4_streaming waits-for-pieces |
| 4 Torrent seek unavailable → demand → resume | m4_streaming tail-range + seek-cancels |
| 5 player pause, download continues | element-native pause (no backend call — by construction, §29/§39); download-continues proven by concurrent-read suite |
| 6 player stop, resource intact | stop only revokes the session (source-switch test asserts 404; files untouched — no delete call exists on the stop path) |
| 7 source switch | `source_switch_releases_old_session` |
| 8 invalid token → rejected | m4_lower security matrix |
| 9 frontend reload | `frontend_reload_recovers_active_session_projection` |
| 10 resource deleted → error, no crash | M4 §102 + 503 suites; frontend maps ResourceUnavailable |

## Manual verification items (release build, real WebView2)

fullscreen/keyboard/volume/rate interactions and the §99 acceptance
chain end-to-end in-window. The environment cannot script the
WebView2 window; these are recorded as the standing manual pass,
same as M4's §18 statement. Steps: start a download → Play → verify
controls/keys/volume/rate/fullscreen → seek past the frontier
("正在等待下载数据…" → resumes) → reload the window (session/position
recover) → switch source (old session dead).
