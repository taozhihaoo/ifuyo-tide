# M9 Performance

Method first (§104: no fake benchmarking) — every number below is from
a named, reproducible command on the dev machine (Windows 11, debug
test binaries unless noted).

## Backend (reproduced from M7 §134 benchmark)

| Scenario | Command | Result |
|---|---|---|
| Scheduler compute, 1000 tasks | `cargo test --test m7_scheduler bench_tick -- --ignored --nocapture` | 807 µs/tick (avg of 20) |
| 5000 demand submissions | same | ≈ 210–250 µs |
| Library scan, 10k real files | `cargo test --test m6_library_lower bench -- --ignored` | 627 ms |
| 100k library rows insert | same | 9.1 s |

## Frontend (§103)

| Scenario | Command | Result |
|---|---|---|
| 1000-row format/render | `pnpm test -- polish.test.ts` | passes a loose 500 ms budget (actual: single-digit ms) |

## Long-run stability (§47/§105/§106)

Soak harness: `tests/m9_soak.rs`
(`TIDE_SOAK_SECONDS=3600 cargo test --test m9_soak -- --ignored`).
Verified 60 s run: 502 churn iterations through start/pause/resume/
delete + 25k demand submit/cancel — the M4 registry stayed bounded
(pending 0 after cancels) and the engine map growth tracked the
deliberate churn pattern (every 4th iteration deleted).

Cleanup audit (§106): every listener/timer has a matched teardown —
PlayerPage controller `dispose()` (§87 test), tray menu commands are
one-shot spawns, the scheduler loop holds only Arcs, downloads delete
removes their registry entries via the tick retain (§144).

## Idle quietness (§48/§49)

No polling timers exist outside: the 500 ms scheduler tick, the 1 s
player availability poll (only while the Player view is mounted), the
Downloads 1 s dev-only diagnostics poll (DEV builds only), and the
Library 1.5 s refresh. Progress flows event-driven from the backend's
throttled emitter (M1 §24). No animation runs continuously (reduced-
motion + finite keyframes only).

## Known hotspots / honest notes

- Startup cost is dominated by database open + M2 recovery, by design
  (§109: correctness first) — M2 recovery tests cover the behavior.
- The scheduler tick recomputes from the snapshot each pass; at
  current scale this is microseconds and was not optimized further
  (§104 no premature optimization).
