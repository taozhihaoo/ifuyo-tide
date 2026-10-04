# M9 Regression Matrix (§91–§99/§124)

Every M0–M8 area re-verified after the M9 UX polish. M9 touched ONLY
frontend presentation files + docs — no backend behavior changed —
so the automated M0–M8 suites are the regression proof, re-run after
every M9 commit.

| Area (§) | Suite | Result | Notes |
|---|---|---|---|
| M1 HTTP (§92 full cycle) | `m1_download` 13 tests | PASS | create/pause/resume/cancel/retry/complete/delete |
| M2 Persistence (§92) | `m2_persistence` 8 tests | PASS | crash/recovery/resume/migration |
| M3 Torrent (§93) | `m3_torrent` 7 tests | PASS | metadata/selection/start/pause/resume/availability/completion/error |
| M4 Streaming (§94) | `m4_streaming` 7 + `m4_lower` 12 tests | PASS | seek-while-unavailable covered (`torrent_read_waits_for_pieces_then_serves`, `critical_2`) |
| M5 Player (§94) | `playbackState` + `playbackController` 21 frontend tests | PASS | play/pause/seek/waiting/ended/error mapping |
| M6 Library (§95) | `m6_library` 11 + `m6_library_lower` 15 tests | PASS | task→library→play→open→remove→restart |
| M7 Scheduler (§96) | `m7_scheduler` 3 + scheduler units 22 tests | PASS | priority/playback demand/seek demand/pause/resume |
| M8 Windows (§97) | platform unit 12 + M5/M8 session tests | PASS | launch parsing/settings/drain; tray/activation manual pass |

**§98 Visual regression**: baseline states enumerated in
docs/M9-design-system.md (§111 checklist); verified via the manual
pass — no heavyweight snapshot framework introduced (§98 allows
lightweight/manual).

**§99 Interaction matrix**: Pause→paused, Resume→running, Cancel→
cancelled, selection persisted (M3), seek→position changes (M5),
Play→player opens (M6), Settings→persisted (§80 KV), Tray→restores
(M8) — each backed by the suites above; OS-visual rows are the
standing manual pass.

**NOT TESTED / KNOWN LIMITATION**: in-window visual screenshot
diffing (no framework, by design §98); magnet: OS association
(deliberate, M8 §65).
