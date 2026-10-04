# M4 Test Matrix

## Results (M4(下) final)

- Rust: **235 tests, 0 failures** (`cargo test --locked`) — 166 unit +
  integration suites `m1`/`m2`/`m3`/`m4_streaming` (7) + `m4_lower` (10)
  + persistence/spike suites.
- Frontend: `pnpm test` 3/3, `tsc --noEmit` clean.
- `cargo clippy --locked` and `cargo fmt --check` pass (see report
  §27 for the exact verification run).
- `pnpm tauri build` produces the release exe.

## Test architecture (§177)

| Layer | Where |
|---|---|
| unit | `core/streaming` (demand ordering/dedup/cancel, resource capability, range-unsupported, error codes), M1 range parser, task/state machines |
| integration | `tests/m4_streaming.rs` (7), `tests/m4_lower.rs` (10), `m1`/`m2`/`m3` regression suites |
| property-style | seeded deterministic loops over range resolution × Content-Range consistency (parser suite), byte-pattern assertions at arbitrary offsets |
| fuzz/abuse | `fuzz_abuse_battery_never_panics` (§136) + `security_matrix_full` (§100/§101/§125) |
| E2E | WebView2 visual playback measured end-to-end in M0.5 (documented); M4 ships the production wiring of the same shape — see report §18 for the honest E2E statement |

## Coverage map (spec requirement → test)

| Requirement (M4(下)) | Where proven |
|---|---|
| §98 error taxonomy, stable codes | `error_codes_are_stable` (streaming codes asserted) |
| §100 arbitrary-file-read P0 | `security_matrix_full` (all listed shapes denied, empty bodies) |
| §101 token P0 | `security_matrix_full` (expired/guessed/foreign/closed) |
| §102 task deletion | `task_delete_invalidates_its_sessions` |
| §103/§104 isolation/capability | docs + capability file audit (see report) |
| §105/§106 gateway-not-IPC, loopback media | architecture (ADR-8) + M0.5 measured E2E |
| §107 status correctness | 200/206/416/400/405/503 assertions across both M4 suites |
| §108/§109 Content-Length/Range consistency | `critical_9` (+ parser unit suite) |
| §110 range parser robustness | M1 parser unit tests + §136 battery |
| §111/§112 availability ordering | engine fires watch AFTER flush; `concurrent_read_while_writing_never_tears` |
| §113/§114 demand lifecycle, in-flight merge | `identical_pending_demand_merges` + ordering tests |
| §115 reader wakeup, no busy loop | event-driven watch (`wait_for_durable_len`); no polling loops in streaming code |
| §116/§137 cancellation | `session_close_aborts_blocked_read_promptly` |
| §126 Critical #1 | `critical_1_http_progressive_playback_waits_at_frontier` |
| §127 Critical #2 | `critical_2_http_seek_beyond_frontier_reads_through_origin` |
| §128/§129 Critical #3/#4 | `torrent_read_waits_for_pieces_then_serves` + `seek_cancels_stale_demands` (m4_streaming) |
| §130 Critical #5 | `seek_cancels_stale_demands` |
| §131 Critical #6 | `security_matrix_full` |
| §132 Critical #7 | `critical_7_restart_recovers_resource_and_playback` |
| §133 Critical #8 | `unified_gateway_serves_file_http_and_torrent` — one player, no source branches |
| §134 Critical #9 | `critical_9_byte_preservation_across_random_ranges` |
| §135 property-style | parser/round-trip suite + demand invariants |
| §136 fuzz | `fuzz_abuse_battery_never_panics` |
| §138 no busy loop | code audit: waits are watch/deadline driven; reaper is a 10-min task |
| §140/§185 log safety | docs/M4-security.md + code audit |
| §143 network classification | `classify_reqwest` + `origin_that_ignores_range_is_rejected` |
| §145 Range-unsupported fallback | `RESOURCE_RANGE_UNSUPPORTED` never serves wrong bytes |
| §146/§147 capabilities | `part_capability_reflects_origin`, DTO + UI gating |
| §148/§149 UI/buffer honesty | VideoPlayer dual metrics; vitest suites |
| §169/§196 performance baseline | `performance_baseline_recorded` (numbers in report §19) |

## §124 player matrix status

load/play/pause/seek forward-backward/rapid seek/waiting/resume/
ended are element-driven and verified by M0.5 measured behavior + the
integration layer behind them; session stop and app shutdown are
covered by `session_close_aborts_blocked_read_promptly` and the
runtime-only session model. The in-browser visual run of this matrix
against the FINAL production gateway remains a manual step (report
§18).
