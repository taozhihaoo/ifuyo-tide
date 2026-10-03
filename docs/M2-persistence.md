# M2 — Task Persistence

What SQLite stores, what it deliberately does not, and why.

## What is stored (`download_tasks` table, migration 0002)

Identity & source:
- `id` — UUIDv4, NEVER changes across pause/retry/restart (M2 §14)
- `source_url` — the request URL (re-validated on every resume)
- `allow_private` — the persisted SSRF opt-in

Target:
- `destination_dir` + `filename` + `part_path` + `final_path` — the
  resolved locations (M2 §35: root AND resolved name; the `.part` path
  is persisted so recovery finds the artifact regardless of
  AutoRename collisions)
- `overwrite_policy` — persisted with the task (M2 §34)

Progress:
- `total_bytes` — NULL when unknown (M2 §62: never 0-as-fake)
- `downloaded_bytes` — CHECKPOINT HINT (M2 §24); the `.part` file size
  is the physical truth and recovery reconciles to it
- no percentage stored (M2 §63 — always computed)

Validators (M2 §13/§73):
- `etag`, `last_modified` — survive restart for safe `If-Range` resume

Integrity:
- `checksum_algorithm`, `checksum_expected`

Lifecycle:
- `state` — CHECK-constrained M0 lifecycle string; a stale `running`
  row is reclassified to `ready` by startup recovery (M2 §8: "running"
  is not a durable fact)
- `created_at`, `updated_at`, `started_at`, `completed_at` — UTC epoch
  ms (M2 §64)

Errors & recovery:
- `retry_count` (M2 §50: count persisted, backoff timers are not)
- `last_error_code`, `last_error_message`
- `recovery_reason` — why the last recovery decided what it decided
  (M2 §96 reason codes)

## What is NOT stored

- speed / ETA / instantaneous progress (M2 §49 — recomputed)
- backoff sleep timers (M2 §50)
- any runtime object: HTTP client, tokens, stream state (M2 §41)
- percentages (M2 §63)

## How migrations run

`PRAGMA user_version`-gated, transactional per migration
(`persistence/migration.rs`). 0001 = M0 tables; 0002 =
`download_tasks`. The download repository re-runs `run_migrations`
idempotently on open (it may be the first connection to touch the
file). Repeated/interrupted migration covered by tests
(`migrations_are_idempotent`, M0 `migration regression`).

## Repository boundary

`DownloadEngine → DownloadRepository → SQLite`. The repository owns a
DEDICATED Mutex'd connection (rusqlite is !Sync; WAL gives one writer +
readers) so engine checkpoints never block app queries. The engine
never touches SQL. Recovery/business decisions live in
`core/recovery` (M2 §18/§19).

## Checkpoint frequency (M2 §20)

Time-or-byte, whichever first: **1 second OR 1 MiB**, plus a forced
write on every lifecycle transition. Measured: loopback 1 MiB download
completes with 1–3 checkpoints; a 4 GiB download ≈ 4000 checkpoints
(~one small UPDATE per second — no measurable throughput change vs M1).
