# M11 Recovery & Reliability Validation

Scope: §22–§30 (migrations, upgrade-in-progress, crash recovery,
finalization, restart/shutdown, offline/network, disk failure) plus
the resource-budget evidence (§54–§57). Evidence types as per M11
rules. The 317-test Rust suite ran green at the RC commit (TEST
RESULT, `cargo test --locked`).

## 1. SQLite migration (§22/§23)

- 4 versioned migrations (`0001`–`0004`), each applied inside a
  transaction that also bumps `PRAGMA user_version` — a crash
  mid-migration cannot leave a half-applied schema (FACT,
  `persistence/migration.rs`).
- Idempotency: runner applies only `version > current`. TEST RESULT:
  across matrix runs on a reused data root, `user_version` stays 4
  and zero re-application events are logged; fresh roots log exactly
  4 `db.migration_applied` events.
- No existing migration was modified, renumbered or rewritten for
  M11 (§23 — FACT, M11 diff audit).
- Old-version-DB upgrade: no pre-RC release schema divergence exists
  (0.1.0 dev builds already carry the 4-migration schema — FACT), so
  the "old DB → RC" path is exercised as in-place upgrade with
  preserved data (TEST RESULT, install matrix S5) rather than a
  cross-version migration drill. Honest scope note, not a claim of
  multi-version migration coverage.

## 2. Upgrade with in-progress download (§25)

The three DB-vs-`.part` relations have deterministic, tested
decisions (TEST RESULT):

| Relation | Decision | Test |
|---|---|---|
| DB `downloaded_bytes` < `.part` size | recover forward to file position | `m2_persistence::db_behind_part_recovers_to_file_position` |
| DB `downloaded_bytes` > file size | never resume past data (truncate stale tail) | `m2_persistence::db_ahead_of_file_never_resumes_past_data`, `file_sink::resume_offset_truncates_stale_tail` |
| equal / zero | resume exactly at offset | `validator::resume_decision_zero_vs_partial` |

Installer-level `.part` upgrade: no live `.part` existed in the
matrix window; the engine-level suites above cover the byte-level
semantics, and the upgrade preserves the app-data directory
verbatim (install matrix S5) — INFERENCE: an interrupted download
under RC resumes under the same rules; UI-verified upgrade-with-
active-download remains a manual item.

## 3. Crash recovery regression (§26/§27)

Dedicated suites (TEST RESULT, all green):

- `crash_before_completion_recovers_by_finalizing` — `.part` with
  all bytes written finalizes to the final file on recovery; nothing
  is marked complete without its artifact.
- `completed_without_artifact_fails_on_recovery` — DB says complete
  but the final file is missing → structured failure, never a fake
  completion (§27 "DB complete, file missing").
- `missing_part_restarts_from_zero_with_reason` — `.part` gone →
  restart from zero with a recorded recovery reason.
- `db_ahead_of_file_never_resumes_past_data` — no skipped/missing
  bytes (§26 "never continue beyond the safe persisted position").
- `thousand_task_recovery_is_bounded` — recovery cost is bounded at
  1 000 tasks (§53 scale point covered at the engine level).

## 4. Restart / shutdown (§28)

- Graceful exit: `RunEvent::Exit` handler logs `app.shutdown`;
  every engine state transition is persisted before exit (FACT).
- Process kill: smoke script kills the process mid-session and
  requires a healthy warm relaunch (`app.started`, `user_version=4`,
  seeded row intact) — TEST RESULT, 3/3 matrix runs.
- Window close to tray, OS shutdown/restart, force termination via
  OS: manual checklist items (M11-test-matrix.md §4) — the kill test
  above is the automated worst-case; graceful shutdown is an
  optimization on top of crash safety, never the only guarantee
  (design FACT from M2).

## 5. Offline / network failure (§29)

- Engine-level: pause/resume/retry semantics, bounded retry counts,
  structured error codes (M1/M2 suites, green — TEST RESULT).
- Live offline toggling, DNS failure, 429/5xx handling on real
  networks: NOT instrumented in M11 — manual checklist item (§51
  honesty rule). No busy-loop evidence at idle: smoke idle CPU
  sample ~0% over 8 s (TEST RESULT).

## 6. Disk failure (§30)

- Corrupt DB: fail-closed startup (matrix S8 — TEST RESULT).
- Insufficient disk / write failure / permission denied: structured
  `TideError` codes on the persistence boundary (FACT); a dedicated
  injected-failure suite does not exist — recorded as NOT TESTED at
  fault-injection level; app-level guarantee is: no panic path on
  user input (M10 panic audit + hostile-string corpus, green).

## 7. Idle resource budget (§54) — TEST RESULT (measured subset)

- Idle CPU ~0% over 8 s samples (3/3 runs; threshold < 5%).
- Cold start to `app.started`: 1 088–1 755 ms across installed runs
  (2 704 ms for a dev-tree run) — baseline recorded, no regression
  budget defined yet.
- WorkingSet readout via the script is unreliable (reports 0 MB for
  GUI-subsystem processes started from Start-Process) — memory
  figures are a manual Task-Manager item, not claimed here.
- No high-frequency timers write SQLite at idle (scheduler design
  FACT, M7; corroborated by idle CPU sample).

## 8. Resource leaks / gateway cleanup (§55/§56)

- Playback session stop invalidates the session token and frees the
  gateway route (TEST RESULT: `m4_streaming::gateway_security_matrix`
  — closed sessions are rejected).
- `m9_soak` covers extended-run behavior (green).
- Zombie port / locked-file audit across Task Cancel / Exit / Crash
  is NOT instrumented in M11 — manual checklist item (§55 honesty
  rule).

## 9. Task concurrency (§57)

Scheduler semantics are frozen from M7 (deterministic four-phase
policy, token-bucket bandwidth); `m7_scheduler` + `m9_soak` suites
green (TEST RESULT). Scale points 1/2/4/8 concurrent tasks are
covered by the suite's parameterized scenarios; UI-side concurrency
behavior is a manual item.
