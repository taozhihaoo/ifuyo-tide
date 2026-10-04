# M6 Final Report

Per M6(下) §155 — covers BOTH halves of M6 (上 §1–§91 implementation,
下 §92–§160 tests/acceptance).

## 1. Executive Summary

Tide now has a reliable, persistent, recoverable, searchable, playable
local resource index: explicit roots, a partial-scan-safe scanner with
cancellation and single-flight, one-transaction scan commits, download-
and torrent-completion imports, backend-side search/filter/sort/paging,
a reconciler for filesystem/index drift, and library-item playback
through the UNCHANGED M4 gateway into the M5 player. 268 Rust + 25
frontend tests pass; a recorded benchmark covers 100k-item scale. Five
real defects (one P0-class) were found by the M6 suites and fixed at
their owning layers.

## 2. FACTS

- Rust 268/0 (`cargo test --locked`), frontend 25/0 (`pnpm test`),
  `tsc` clean, fmt clean, clippy clean in M6-owned code,
  `pnpm tauri build` PASS.
- Benchmark (dev machine, debug test binary): scan 1k=57 ms, 10k=627 ms;
  100k-row index build 9.1 s; search p50 50 ms / p95 59 ms at 100k;
  filter+sort 276 ms; idempotent re-commit 9.6 s (row count stays
  exactly 100k).
- A real NTFS junction pointing outside a root is never traversed nor
  indexed; forged out-of-root rows are refused by play AND delete.
- A >4 GiB sparse file plays ranged reads at 4.5 GiB with exact
  Content-Range.
- Cancelled scans commit nothing; before-DB-upsert faults leave the DB
  byte-identical; after-commit faults keep committed data.

## 3. HYPOTHESES

- The §111 in-window visual scenarios (A–G) hold in the real UI: every
  step is covered by automated layers; the click-through itself is the
  standing manual pass (disclosed, not assumed).
- 50k/100k real-FILE scans behave like the 10k measurement (the 100k
  index-build numbers are synthetic-row; file enumeration cost was
  measured separately at 10k).

## 4. Existing M0–M5 state

Verified unchanged and green. M6 added one enabling fix outside the
library: a torrent completion monitor (nothing detected torrent
completion before — rows stayed "downloading" forever). The gateway,
player, and engines gained no new media-path responsibilities.

## 5. M6 Architecture

docs/M6-library.md — Task/Resource/LibraryItem/Playback stay four
concepts; module map model/path/classifier/repository/scanner/
reconciler/service; no万能 Manager (§127), LibraryService stays a
thin facade (§128).

## 6. LibraryRoot Model

Explicit directories only (no drives, no system dirs), enabled flag +
scan_status + last_scan_at; duplicate path dedupes to the same root.

## 7. LibraryItem Model

Canonical path + relative path + display name + classification +
size/mtime + availability enum + origin + nullable task_id + reused
checksum. Stable random UUID ids (§137 — never array index or
filename). UTC in persistence, local in UI (§138).

## 8. SQLite Schema / Migration

Migration 0004 (library_roots/library_items), UNIQUE(root_id,
canonical_path), CHECK-constrained enums, FK to roots, indexes for the
query paths. v3→v4 regression test keeps every pre-existing table;
tampered schema fails with structured PERSISTENCE errors (§144).

## 9. Scanner Design

docs/M6-library-scan.md — BFS enumeration, artifact exclusion,
reparse-safe, uncovered-prefix safety valve, single transaction,
cancellation, same-root single-flight, test-only fault injection at
the §109 points.

## 10. Filesystem Reconciliation

docs/M6-library-scan.md — the four-case reconciler; drift is expected,
detected and repaired (startup light verification, refresh_item, full
scans); no smart path guessing (§119).

## 11. Task → Library Integration

Download completion hook fires AFTER atomic finalize + persisted
state; import is idempotent (×N completions → one item, §99); task
deletion never touches items (§100, tested).

## 12. Torrent → Library Integration

Completion monitor reconciles engine stats every 5 s, persists the
terminal state (pre-M6 gap fixed) and imports each selected file as
its own item (mode A, §122).

## 13. Resource → Library Integration

Library file → StreamingResource::File (complete availability) →
`read(range)` verified byte-exact at arbitrary offsets, including
4.5 GiB into a sparse file (§123/§105).

## 14. Playback Integration

item id → filesystem re-validation → PlaybackSession → gateway →
M5 player; missing file → structured error; fake/id-shaped ids →
NOT_FOUND (§104); playing file blocks deletion with FILE_IN_USE
(§136).

## 15. Search / Filter / Sort

All backend-side in SQL (LIKE per §76); pagination limit/offset;
measured latencies at 100k rows in §21. No FTS (numbers don't justify
it yet).

## 16. Delete / Remove Semantics

Remove = index only; Delete = confirmation (UI) + backend
re-validation + FILE_INUSE refusal while playing + file removal + row
removal. Task deletion never removes items; item removal never
touches tasks (§52/§53).

## 17. Security Review (§142)

Path: traversal/reparse/junction/UNC/device names — canonicalize +
component-wise containment + empirical reparse detection, fuzz-tested
(500 hostile paths). DB: tampered path/foreign root → SECURITY;
corrupted schema → structured failure. IPC: ids only, no path-shaped
APIs. Gateway: unchanged M4 surface (loopback, tokens, host, range).

## 18. Privacy Review

Local SQLite only; no telemetry/upload; library logs carry event
names + counts + item ids, never paths/URLs/magnets (§64/§65/§141).

## 19. Test Matrix

docs/M6-test-matrix.md — the full §151 table mapped to 26 M6 tests
(11 上 + 15 下) plus unit suites.

## 20. Failure Injection (§109)

Three test-only fault points wired through `ScanFaults`; each proves
a distinct consistency property (before-DB untouched / after-commit
persists / partial keeps uncovered availability).

## 21. Performance Benchmark

Recorded numbers in §2 and docs/M6-test-matrix.md; reproduce with
`cargo test --locked --test m6_library_lower benchmark -- --ignored
--nocapture`. Startup impact: none — library loads from SQLite and
verifies in the background (§72).

## 22. Changed Files

`core/library/**` (model/path/classifier/repository/scanner/
reconciler/service), `core/download/engine.rs` (completion hook),
`core/torrent/monitor.rs`, `error.rs` (Busy/FileInUse),
`commands/library.rs`, `commands/playback.rs`, `app/{mod,state}.rs`,
migrations/0004, `tests/m6_library*.rs`, frontend `features/library/
**` + Settings + App wiring, styles, docs set.

## 23. RESOLVED

- P0-class: `\\?\` verbatim canonical paths broke every plain-path
  comparison (containment/system-path checks inert).
- Missing-marking LIKE used GLOB `*` (§69 valve inert).
- Junctions silently skipped as "non-files" instead of refused.
- Torrent completion never detected (rows stuck "downloading").
- Security token-mutation test could be a no-op (M4 suite).

## 24. CONFIRMED

- Library → Resource → PlaybackSession → gateway → player chain is
  byte-exact and stable (§157's M7 prerequisite).
- Scans are idempotent, partial-safe, cancellable, and crash-consistent.

## 25. OPEN

- §111 in-window visual run (manual pass; layers automated).
- 100k REAL-file scan (100k measured via index-build path; real-file
  enumeration measured to 10k).
- i18n of library strings (project has no i18n system yet — M9 owns
  localization; §139 decision recorded).

## 26. WORSENED

Nothing — all pre-M6 suites remain green.

## 27. P0

0 open (the canonicalize defect was fixed before completion).

## 28. P1 (recorded)

- Search at 100k is ~50 ms p50 via LIKE — fine per §76; FTS remains
  available if usage proves otherwise.
- 100k real-file scan not directly measured (synthetic-row path
  measured; enumeration measured to 10k).

## 29. Known Limitations

No watcher (§36: manual scan + startup verification + completion
imports); junctions are refused rather than followed (determinism
over cleverness, §119); Open/reveal uses Explorer spawn (no new
plugin/capability, §93); subtitles/posters out of scope (§49/§50/
§113).

## 30. M7 Readiness (§157)

`LibraryItem → Resource → Availability → PlaybackSession` is stable
and tested. M7 can schedule against: what the user HAS (library), what
can be READ (resources), what the user is WATCHING (sessions) — with
demand/priority untouched by M6 (no scheduler code was added).

## 31. Final DoD Checklist (§154)

- Architecture: separation kept, PlaybackSession reused,
  Resource.read(range) intact, no new managers — DONE.
- Persistence: roots/items/migration/repository/restart survival/
  duplicate-path uniqueness — DONE.
- Scanner: nested/temp-exclusion/missing/restored/partial/cancellation/
  background — DONE.
- Integration: HTTP ✓ torrent ✓ multi-file ✓ task-delete safe ✓
  library-delete never cancels tasks — DONE.
- Playback: session ✓ gateway ✓ player ✓ no arbitrary paths ✓ missing
  fails safely — DONE.
- Filesystem: canonicalization ✓ traversal ✓ junction ✓ UNC defined
  (canonicalized, containment-checked) ✓ permissions ✓ disappearance ✓.
- Search/UI: search/filter/sort/pagination/progress/real-data — DONE;
  i18n deferred to M9 (recorded).
- Delete: remove≠delete ✓ confirmation ✓ backend validation ✓
  active-playback behaviour defined (FILE_INUSE) — DONE.
- Tests: unit/integration/filesystem/database/security/playback/
  injection/large-file ✓; benchmark RECORDED; locked build/test PASS.
- Documentation: architecture/scan/security/test-matrix/report — DONE.

Per §160: M6 STOPS here — no M7, no media-centre extras.
