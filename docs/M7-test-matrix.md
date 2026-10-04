# M7 Test Matrix & Simulation Results

## Automated coverage (M7 final)

- Rust 301 tests / 0 failures: 188 unit (incl. 16 M7上 policy/
  bandwidth/adapter tests, 4 RangeSet tests, stress-escalation,
  hysteresis, paused-release, determinism, aging cap, user-priority
  decomposition) + `m7_scheduler` integration/simulation suite
  (12 tests + 1 benchmark) + all M0–M6 suites.
- Frontend 25/0; `tsc` clean; fmt clean; clippy zero warnings in
  scheduler code; `pnpm tauri build` PASS.

## Spec section → test

| Requirement (M7下) | Where proven |
|---|---|
| §103 priority order + same-class fairness + aging/caps | M7上 unit tests + `same_class_tasks_fair_share`, `aging_is_capped` |
| §104 demand create/merge/cancel/expire/replace | `seek_and_pause_reports_are_idempotent` + M4 registry suite (`identical_pending_demand_merges` etc.) |
| §105 range ops (merge/subtract/intersect/difference) | `range_operations` |
| §106 availability hole EXACT | `availability_hole_exact` |
| §107 buffer edges (0/unknown/large) | hysteresis tests + classify "unknown → healthy" branch; large offsets in `missing_handles_edges` |
| §108 fairness (no starvation) | `same_class_tasks_fair_share` + `sim4_background_fairness_over_ticks` |
| §109/§110 aging monotonic + capped below Emergency | `aging_is_capped` + weight table (aging only on Normal/Background) |
| §111 bandwidth sum ≤ cap | `sim5_adapts_to_bandwidth_changes_without_overflow` + property invariant |
| §112 task limit | M1 `MAX_CONCURRENT_DOWNLOADS` semaphore (pre-existing) + config knob recorded |
| §113 capability degrade, no panic | `http_adapter_applies_and_degrades`, `torrent_adapter_reports_capability_limitation` |
| §114 property (sum ≤ cap, no dup, non-running = 0) | `property_invariants_hold_over_random_snapshots` (100 cases) + `sim_random_invariants` (80 cases) |
| §115–§121 simulation scenarios 1–6 | `sim1`–`sim6` |
| §122 HTTP integration | `per_task_rate_actually_paces_the_download` + `runtime_tick_diffs_and_applies_to_the_data_plane` |
| §123/§124 torrent & combined | `torrent_session_rate_smoke` + swarm coverage in M3/M4 suites (combined three-way via provider test pending real swarm — recorded OPEN) |
| §125/§126 seek/rebuffer | `sim3_seek_reroutes_allocation` + stress-escalation tests (real-player soak remains manual) |
| §127/§128 paused/closed player | `paused_playback_releases_capacity` + M4 close revocation suites |
| §129 restart reconstruction | `vanished_task_loses_all_runtime_state` (re-registered → fresh allocation) + M2 suites |
| §132 idempotency | `seek_and_pause_reports_are_idempotent` |
| §134 benchmark | see below |

## Performance benchmark (§134, dev machine, debug test binary)

| Task count | compute (first) | per tick (avg 20) |
|---|---|---|
| 10 | 63 µs | 7.7 µs |
| 100 | 129 µs | 70.7 µs |
| 500 | 301 µs | 405 µs |
| 1000 | 1.06 ms | 807 µs |

5000 M4 demand submissions: ≈ 210–250 µs total. Reproduce:
`cargo test --locked --test m7_scheduler bench_tick -- --ignored
--nocapture`. §135 discipline: no optimization was done — numbers
recorded as the baseline (well under the 500 ms tick budget).

## Manual / deferred items (honest)

- §111(前端)/§111 real-WebView soak: the scheduler responds to REAL
  element telemetry (verified at the API layer); the visual
  click-through of Demos A–I (§164) remains the standing manual pass —
  same disclosure as M4/M5/M6 reports.
- §123 torrent-over-real-swarm WITH playback simultaneously: the
  swarm machinery is proven in M3/M4 suites and the scheduling layer
  is engine-agnostic (provider views); a combined three-way live test
  is deferred (OPEN).
