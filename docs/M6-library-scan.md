# M6 Scanner & Reconciliation

The scanner's exact behaviour, its safety valves, and how the library
detects and repairs filesystem/database drift (M6(下) §108/§117/§118/
§130).

## Scan algorithm (M6上 §68 + M6下 §109/§130)

```text
begin_scan (single-flight, §131)          → BUSY if already scanning
  ↓ enumerate root (BFS, reparse-safe)
  ↓   · cancellation checked between directories and batches
  ↓   · injected faults possible at §109 points (test-only)
  ↓ collect candidates
  ↓ cancelled? → return Cancelled, commit NOTHING
  ↓ injected before-DB fault? → return Err, commit NOTHING
  ↓ ONE transaction: upsert candidates + Missing-mark unseen
     (uncovered prefixes excluded)
  ↓ injected after-commit fault? → return Err, data STAYS committed
end_scan (release single-flight)
```

Status transitions on the root: `scanning → ok | partial | failed`,
cancel → `idle`. A cancelled or aborted scan can never produce false
Missing verdicts: Missing marking only runs inside the successful
commit, and only for directories the scan actually covered.

## Missing-safety valve (§69)

Every directory whose enumeration fails (permissions, reparse point,
IO error, injected fault) is recorded as an UNCOVERED prefix. The
Missing UPDATE excludes everything under those prefixes — items in
folders the scan could not read keep their availability. The empirical
junction handling: modern Rust reports junctions with neither dir nor
file bits → classified as reparse → uncovered, never traversed.

## Reconciliation (§117/§118)

SQLite and the filesystem cannot commit atomically together; drift is
expected and DETECTABLE. The reconciler owns the four cases:

| DB | Filesystem | Verdict |
|---|---|---|
| Available | exists (file) | Available |
| Available | missing | Missing |
| Missing | exists again | Available |
| Available | exists, metadata changed | Available + stale noted; next full scan refreshes metadata |

Entry points: startup light verification (§71/§73 — existence only,
background, UI never waits), `refresh_item` (§49 play-time check),
full scans. NOT automated: path guessing — a missing file is never
re-bound to a same-named file elsewhere (§119, determinism first).

## Cancellation (§130)

`library_cancel_scan(root_id)` sets the root's flag; the scan stops
between directories/file batches, commits NOTHING, and the root
returns to `idle`. No zombie scanners (single-flight registry released
via `end_scan` in every path), no false Missing, UI state restored via
the `library://scan` event with status `cancelled`.

## Concurrency (§131)

Same-root scans are single-flight — a second `begin_scan` gets a
structured `BUSY`. Different roots MAY scan concurrently (independent
directories; each commits its own transaction). The registry is
per-service state, not a global lock — normal reads/writes never
contend with it.

## Performance (§106/§107, measured — see M6-report §21)

Scan is one transaction per root; progress events are throttled to
~300 ms; per-file cost is dominated by enumeration, not DB writes.
Measured on the dev machine: 1k files ≈ 57 ms, 10k ≈ 627 ms, 100k-row
index build ≈ 9.1 s, search p50 ≈ 50 ms at 100k rows.
