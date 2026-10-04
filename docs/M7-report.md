# M7(上) Report

## 1. Executive Summary

The scheduler now exists as an independent CONTROL PLANE: a pure
deterministic policy computes bandwidth allocations from an immutable
snapshot (tasks + playback sessions + global cap), and the data plane
enforces them for real — per-task HTTP token buckets in the M1 chunk
loop and a session-level torrent rate via librqbit's PUBLIC ratelimits
API. Playback scheduling follows REAL browser-buffer seconds with
hysteresis; background tasks keep an anti-starvation floor. Every
decision carries a decomposable reason. 16 policy/bandwidth unit tests
incl. the §91–§96 examples, 3 integration tests over real engines,
286 Rust + 25 frontend tests green, release build PASS.

## 2. FACTS

- Rust 286/0 (incl. 16 scheduler unit + 3 scheduler integration
  tests), frontend 25/0, `tsc` clean, fmt clean, clippy zero warnings
  in scheduler code, `pnpm tauri build` PASS.
- REAL pacing proven: 1 MiB @ 256 KiB/s completes in ≈2 s (unlimited
  ≈0.1 s); the §17 diff applies 100k → (skip) → 250k exactly once each.
- rqbit 9.0.1 exposes `Session.ratelimits.set_download_bps` — the
  torrent adapter needs NO fork (§12).
- Config defaults are marked INITIAL DEFAULTS (§74) — critical 4 s /
  low 12 s / target 30 s / recovery 10 s, emergency min 2 MiB/s,
  playback target 6 MiB/s.

## 3. HYPOTHESES

- The 500 ms tick + diff keeps allocation churn invisible in the UI
  (no jitter observed in the debug panel; formal measurement deferred
  to M7(下)).
- Real-network watermark tuning (§74) is expected to move the
  watermarks; the config centralization makes that a one-file change.

## 4. M0–M6 state

Unchanged and green. The M0-era coarse `Priority` enum was absorbed
into `PriorityClass` (its only consumer was its own test). M4's
DemandScheduler stays the per-chunk runtime registry; M7's policy
layer aggregates intent ABOVE it (no rewrite, §91-reuse discipline).

## 5. Architecture (§3/§14/§81)

`SchedulerSnapshot` (provider: engines + playback) → `policy::compute`
(pure) → `AllocationDecision` intents → adapters: HTTP per-task
limiters (real enforcement in the chunk loop), Torrent session rate
(public API). The scheduler never touches sockets/peers/files.

## 6. Playback model (§8/§27–§34)

Buffer seconds are the REAL `bufferedAhead` from the media element
(reported with position via the M5 controller); `None` = assume
healthy — an unreporting client cannot fake an emergency. States:
Startup/Playing/LowBuffer/CriticalBuffer/Seeking/Paused/Ended/Error
with watermark hysteresis (Critical exits at RECOVERY 10 s, not 1 s).
SEEKING → Emergency elevation; Paused → demand removed (§60).

## 7. Fairness / aging (§20–§22)

Weighted fair split within the leftover; aging adds up to +0.5 weight
after 60 s for Normal/Background only — capped far below Emergency.
Background keeps a floor of the leftover when playback is healthy
(§95) and yields to near-zero when stressed (§59).

## 8. Integration points

HTTP: M1 keeps retry/range/resume (§65/§66 — the scheduler only sets
rates). Torrent: rqbit owns piece selection (§35/§64); Tide sets the
session rate. Playback ↔ Download separation unchanged (§90): the
scheduler only observes tasks and serves sessions.

## 9. Persistence (§72)

User priorities + global bandwidth limit live in the EXISTING settings
KV table (`sched:` namespace) — no new migration; loaded at startup;
scores/allocations/tick state stay in-memory.

## 10. IPC & UI (§75/§76/§78/§79)

`scheduler_set_task_priority` (existence-validated, unknown →
NOT_FOUND), `scheduler_set_global_bandwidth_limit`,
`scheduler_get_snapshot` (DTO only). UI: per-task priority selector
(Downloads), global bandwidth limit (Settings), dev-only debug panel
with live decisions + reason codes (never in release).

## 11. Test coverage

Policy: §91–§96 examples, aging cap, critical hysteresis, paused
release, determinism, user-priority decomposition. Bandwidth: EMA +
bucket pacing. Integration: real pacing, tick diff, torrent rate
smoke. Frontend: telemetry report signature.

## 12. Security (§87–§89)

Priority/range/task-id inputs are re-validated against the live
registries; unknown tasks → NOT_FOUND; no arbitrary resource access;
no implicit download tasks are created (§89); no new capabilities.

## 13. P0 / P1

- P0: 0.
- P1 (recorded): torrent rate is SESSION-level (public API limit);
  watermark tuning awaits real-network experiments (§74); the
  playback view derives states only from reported telemetry (a
  not-yet-reporting client is assumed healthy by design); §39 seek
  prediction and §40+ network fingerprinting deliberately absent.

## 14. M7(下) boundary

M7(下) owns: E2E behavioural matrix (slow-network scenarios §92 in
real playback), debug-panel polish, config persistence UI depth, and
the final M7 DoD. M7(上) alone is not M7-complete (spec split note).

## 15. Commits

core → runtime+enforcement → persistence/IPC/frontend.
