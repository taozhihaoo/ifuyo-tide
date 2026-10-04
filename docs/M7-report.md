# M7 Final Report

Per §168 — covers BOTH halves of M7.

## Executive Summary

M7 completes Tide's resource-allocation brain: a PURE deterministic
policy computes bandwidth allocations from an immutable snapshot
(tasks + playback sessions + global cap), a runtime diffs and applies
them to the data plane (per-task HTTP token buckets, session-level
torrent rate via librqbit's public API), playback buffer health drives
priority with hysteresis, and every decision carries a decomposable
reason. 301 Rust tests (incl. 16 policy/bandwidth/adapter units, 4
RangeSet tests, 7 simulation scenarios, property fuzzing, 3
engine-integration tests) + 25 frontend tests green; benchmark: 1000
tasks in 807 µs/tick. One policy defect (missing stress escalation)
was found by the simulation suite and fixed.

## FACTS

- 301/0 Rust, 25/0 frontend, `tsc` clean, fmt clean, clippy zero
  warnings in scheduler code, `pnpm tauri build` PASS.
- REAL enforcement: 1 MiB @ 256 KiB/s ≈ 2 s (unlimited ≈ 0.1 s); diff
  applies 100k → (skip) → 250k exactly once.
- Benchmark: 10/100/500/1000 tasks → 7.7 µs / 71 µs / 405 µs / 807 µs
  per tick; 5000 demands ≈ 210–250 µs.

## CURRENT M0–M6 STATE

All suites unchanged and green. The M0 Priority enum was absorbed into
PriorityClass; M4's DemandScheduler remains the per-chunk registry
(merge/cancel semantics reused and now tested at §132 level).

## M7 Architecture

docs/M7-scheduler.md — control plane (snapshot → pure policy →
decisions → adapters), single-writer runtime, no sockets/peers/files
(§3/§136/§137/§138).

## Demand Model

docs/M7-demand-model.md — DemandKind (§6), range-aware intent (§9),
M4 registry for per-chunk bookkeeping; merge = one decision per task
(§151/§152 by construction: one task, one allocation; playback only
raises its class).

## Scheduling Policy / Priority / Fairness / Aging

docs/M7-allocation-policy.md + docs/M7-fairness.md — four-phase
allocation, semantic classes, weighted shares, capped aging, background
floor (§95), stress escalation 4× (§31/§92).

## Bandwidth Allocation / Playback Scheduling / Seek

Global cap hard constraint (§24/§111); watermark state machine on REAL
browser-buffer seconds (§8); seek = invalidate + elevate with 500 ms-
coalesced position reporting (§32/§33).

## HTTP Adapter / Torrent Adapter

Capabilities declared (§82); HTTP per-task enforcement proven; torrent
session-level via public API — per-task precision is a recorded
limitation (P1, §12 discipline).

## Persistence Boundary / IPC / UI

Settings-KV only (§72/§163 — no new migration); three commands, DTO
snapshots; priority selector + bandwidth limit + dev debug panel.

## Test Matrix / Simulation / Integration / Performance

docs/M7-test-matrix.md (full §102–§134 mapping + benchmark table).

## Security Review (§87/§88/§142)

IPC intent re-validated against live registries (unknown task →
NOT_FOUND); no arbitrary resource/path access; no new capabilities;
logs carry ids/counts/reasons only.

## FACT / HYPOTHESIS / INFERENCE

- FACT: policy invariants hold over 180 randomized cases; real
  pacing + diff behavior proven over live engines.
- HYPOTHESIS: watermark defaults (4/12/30/10 s) are good starting
  points — real-network tuning pending (§74).
- INFERENCE: 807 µs/1000-task tick leaves ~99.8% of the 500 ms budget
  for everything else.

## RESOLVED / CONFIRMED / OPEN / WORSENED

- RESOLVED: output-contract duplicate decisions (found by property
  fuzz); missing stress escalation (found by simulation sim1);
  Maintenance-vs-playback merge order (found by property fuzz).
- CONFIRMED: rqbit public ratelimits API sufficient (no fork);
  one-task-one-allocation merge by construction (§152).
- OPEN: combined live test (real swarm + HTTP + playback, §124);
  real-network watermark tuning; in-window visual demos (§164) —
  the standing manual pass, all layers automated.
- WORSENED: nothing — all pre-M7 suites green.

## P0 / P1

- P0: 0.
- P1: torrent per-task rate (public API limit); watermark tuning;
  §149 "full download" button is served by the EXISTING task flow
  (no new demand kind needed — recorded).

## Known Limitations

Watermarks are initial defaults; no FTS-like optimizations; scheduler
responds to reported telemetry only (a silent client is assumed
healthy); §176-style future piece-level scheduling stays in rqbit.

## Changed Files

`core/scheduler/**` (config/model/policy/range/bandwidth/adapters/
runtime), `core/download/engine.rs` (limiter/telemetry/Clone/Arc map),
`core/torrent/engine.rs` (rate + telemetry), `error.rs` (Busy/
FileInUse), `commands/scheduler.rs`, `app/{mod,state}.rs`,
`tests/m7_scheduler.rs`, frontend telemetry/API/UI, docs set.

## M8 Readiness (§157)

Task + Resource + Availability + Demand + Scheduler + HTTP + Torrent +
Playback form the testable/explainable/recoverable core M8's Windows
integration assumes. No M8 work was started (§172).

## DoD Checklist (§167)

Architecture/Functional/Reliability/Testing/Security/Performance/
Documentation — all items verified per docs/M7-test-matrix.md and this
report; build gates PASS. M7 STOPS here (§172).
