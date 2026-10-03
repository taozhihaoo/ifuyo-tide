# M2 — Test Matrix and Results

All results from real runs (2026-10-04, Windows 11 x64):

```text
cargo test --locked     # 190 passed, 0 failed
pnpm test               # 3 passed
pnpm build / pnpm tauri build   # PASS
```

## Persistence (unit + integration)

| Test | Result |
|---|---|
| record insert/get roundtrip, duplicate id rejected | PASS |
| list ordering by created_at | PASS |
| checkpoint updates progress+validators atomically | PASS |
| multi-field state update (state+bytes+error) atomic | PASS |
| delete removes record; unknown total stays NULL | PASS |
| migrations idempotent + ordered (M2 §17) | PASS |
| 1000-task recovery bounded (`thousand_task_recovery_is_bounded`) | PASS |

## Crash / recovery matrix (M2 §89, integration)

| Scenario | Result |
|---|---|
| create → persist → restart → task listed, DB reconciled to part size, stale `running` → `ready` | PASS |
| pause → restart → **still paused** (M2 §68) → manual resume → complete (Demo A) | PASS |
| mid-download terminate simulation → recovery → resume (Range observed) → complete (Demo B) | PASS |
| Crash A (part > DB) → resume point follows FILE → completes, checksum verified | PASS |
| DB ahead of file (forged +512 KiB) → resume point moves BACK to file → completes, checksum verified | PASS |
| Crash C / AfterFinalizeBeforeDBUpdate (final complete, DB says running) → recovery verifies + marks Completed → second scan no-op (idempotent) | PASS |
| Crash D (DB completed, artifact deleted) → recovery fails the task honestly | PASS |
| Missing `.part` → restart from zero with `artifact_missing` reason → completes | PASS |
| Remote changed / validator safety | covered in M1 (`remote_changed_restarts_from_zero`) — same code path, exercised through persistence in the restart tests |
| Destination unavailable | decision logic unit-tested (`destination_unavailable_blocks`); E2E needs a removable drive — NOT TESTED end-to-end |

## Invariants (M2 §56) — where each is proven

| # | Invariant | Proof |
|---|---|---|
| 1 | DB never causes resume past physical data | `db_ahead_of_file_never_resumes_past_data` + reconcile unit tests |
| 2 | Completed ⇒ validated final artifact | checksum gate before rename (engine + recovery) |
| 3 | Cancelled never auto-resumes | `decide` unit test (Keep) |
| 4 | Retry preserves task id | retry reuses the record id (engine) |
| 5 | Recovery idempotent | double-scan test in crash_before_completion |
| 6 | Source/destination never lost | record columns NOT NULL, restored entries rebuilt from them |
| 7 | Restart doesn't corrupt partial bytes | sink truncates to verified offset; resume tests byte-exact |
| 8 | Validators survive restart | persisted etag/last_modified; If-Range on resumed requests (server-observed) |
| 9 | UI reconstructable from persistence | restored entries listed via `list_downloads` |
| 10 | Crash doesn't require recreating the task | same task id resumed in every M2 test |

## Performance (M2 §104)

- 1000-task startup recovery: bounded well under 10 s (asserted);
  measured ≈ sub-second on the reference machine (1000 fs stats + 1000
  small UPDATEs).
- Checkpoints: 1 s / 1 MiB throttle — a 1 MiB loopback download emits
  1–3 checkpoints; throughput unchanged vs M1 (loopback saturates).
