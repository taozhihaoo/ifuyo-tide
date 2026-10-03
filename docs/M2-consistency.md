# M2 — Crash Consistency Model

The question this document answers (M2 §100): how does Tide guarantee
that **the DB never guides a resume past physically durable data**?

## The three states (M2 §5)

- **Logical** — SQLite `download_tasks` row (intent + checkpoint hint)
- **Physical** — the `.part` file (truth for "how many bytes exist")
- **Runtime** — the spawned engine task (dies with the process)

## Write ordering (M2 §23)

```text
1. HTTP bytes arrive
2. write_all + flush to .part      ← bytes on disk (page cache)
3. inner.downloaded += chunk       ← runtime counter
4. every 1 s / 1 MiB: SQLite checkpoint UPDATE (step 5 of §23)
```

The DB is NEVER updated before the bytes are written. Therefore at
every instant:

```text
DB.downloaded_bytes ≤ physical .part size (modulo OS-crash page cache)
```

On process kill (not power loss) the page cache survives — the part is
exact. On power loss the OS may lose buffered tail bytes: recovery
TRUSTS THE FILE, whose length is whatever survived, so the resume point
is still valid. The DB hint being stale/behind is always safe.

## Reconciliation (M2 §57/§58)

Startup recovery sets `safe_resume_bytes = physical part size` and
rewrites the DB row to match. The recorded trade-off (M2 §58 format):

- **FACT**: part length is authoritative for "how many contiguous
  bytes exist".
- **TRADE-OFF**: bytes written after the last checkpoint are adopted
  (not truncated) even though they were never checkpointed. Rationale:
  they are a prefix of the same representation (guarded by If-Range on
  resume and by the optional whole-file SHA-256 at completion), and
  truncation would discard real received data.
- **DECISION**: follow the file; If-Range + checksum catch any
  inconsistency the length check cannot.

## Completion (M2 §24/§60)

A checkpoint is a RECOVERY HINT, never proof of completion. Completed
requires: all bytes (size check when total known) → whole-file SHA-256
(when configured) → fsync → atomic rename → DB row `completed`. Any
earlier crash leaves `running`/`ready` + part, and startup recovery
re-derives the state (Finalize path when part == total, checksum
verified before rename).

## SQLite + filesystem are not one transaction (M2 §79)

Every multi-field update is one SQL statement (M2 §78), but the
file/DB pair relies on: write ordering (above) + reconciliation on
startup. Both mismatch directions are tested:

- DB behind file → adopt file (`db_behind_part_recovers_to_file_position`)
- DB ahead of file → resume point moves BACK to the file
  (`db_ahead_of_file_never_resumes_past_data`)

## SQLite pragmas (M2 §76)

| Setting | Why |
|---|---|
| WAL | readers don't block the engine's checkpoint writer |
| synchronous=NORMAL | safe in WAL; fast checkpoints |
| foreign_keys=ON | task→source integrity (M0 tables) |
| busy_timeout=2 s (download repo) | engine + app connections coexist |
