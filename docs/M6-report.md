# M6(上) Report

## 1. Executive Summary

M6(上) delivers the local library: explicit roots, a partial-scan-safe
scanner, one-transaction scan commits, download-completion and
torrent-completion imports, backend-side search/filter/sort/paging,
and library-item playback through the UNCHANGED M4 gateway into the
M5 player. 253 Rust + 25 frontend tests pass. The suite caught four
real defects — one P0-class — all fixed at the layer that owned them.

## 2. Audit conclusions (spec §1/§2)

FACTS: M0–M5 chains verified complete (downloads finalize atomically,
recovery works, torrents produce stable files, gateway/session/player
all real); three repositories share one versioned SQLite file (WAL);
the Library was an M0 placeholder. GAPS: no library schema/scanner,
no completion detection for torrents (nothing ever marked a torrent
completed — P1, fixed in this milestone), no download→library hook.
P0 blocking M6: none. ARCHITECTURAL CONFLICTS: none — M4/M5 APIs were
reused as-is (§91: reuse first).

## 3. FACTS

- 253/0 Rust (`cargo test --locked`, incl. 11 new `m6_library`
  tests), 25/0 frontend (`pnpm test`), `tsc` clean, fmt/clippy clean
  in M6 code.
- Scan idempotency: second scan adds 0, marks 0 missing.
- A real NTFS junction pointing outside the root: contents never
  indexed (errors recorded, prefix uncovered).
- Forged out-of-root item row: play AND delete both refuse with
  SECURITY; the outside file stays untouched.
- Library item → gateway → byte-exact 206 at arbitrary offsets.

## 4. HYPOTHESES

- The single-transaction scan commit stays fast enough at 50k+ files
  (the temp seen-table + set-based UPDATE is designed for it; the
  recorded benchmark lands with M6(下)).
- Explorer-side moves are correctly classified as Missing on the next
  scan (state machine fully covered by tests; the Explorer interaction
  itself is the manual pass).

## 5. M6(上) scope delivered

Roots (add/remove/enable via Settings), scanner + manual scan +
startup light verification + download-completion import +
torrent-completion import, items query UI (Library view), item
actions (Play/Open/Folder/Refresh/Remove/Delete-file), playback
integration via `create_playback_session(library_item)`, docs.

## 6. DEFECTS found and fixed (all by the M6 test suite)

1. **P0-class**: `fs::canonicalize` returns `\\?\` verbatim paths on
   Windows — canonical paths compared UNEQUAL to every plain path
   (env SystemRoot, user input), silently disabling containment and
   protected-location checks. Fixed by stripping the prefix in
   `canonicalize` (UNC form preserved).
2. The Missing-marking pattern used GLOB `*` inside a LIKE — the §69
   uncovered-prefix safety valve was INERT. Fixed (`%`).
3. Junction detection: modern Rust reports junctions with neither dir
   nor file bits — they were silently skipped as non-files. Now
   classified as reparse (uncovered + error), plus a
   canonicalize-moved-the-path check for the general case.
4. rusqlite errors mapped to an opaque string — the error now carries
   the numeric sqlite code (no SQL text, safe for IPC).

Also: the security token-mutation test from M4 could be a no-op
(fixed in the M5 stage of this session's history); `classify` now
case-folds its input defensively.

## 7. M6(上) boundaries kept

No resource discovery, no cloud, no NAS/SMB, no media probing, no
poster/metadata scraping (§4). Task/Resource/LibraryItem/Playback
remain four concepts (§3). The gateway gained ZERO new surface (§58/
§59). No watcher (§36) — manual scan + startup verification +
completion imports form the closed loop (§71). No FTS (§76) — LIKE is
enough at this stage.

## 8. What M6(下) owns

Performance benchmarks at scale (§74–§76), any watch-history/poster
decisions, additional UX polish, and the final M6 DoD/audit — the 上
half alone is explicitly not M6-complete (spec split note).

## 9. P0 / P1

- P0: 0 open (the canonicalize defect was found and fixed before
  completion; re-audited against §12's list).
- P1 (recorded): scale benchmark pending (下); Explorer-move manual
  pass; junction handling refuses instead of following (by design,
  documented); Open/reveal uses Explorer spawn (no new plugin/capability
  added — documented per §93).

## 10. Commits

schema/path/repo → scanner/service → wiring (hooks/monitor/IPC) →
test suite (with defect fixes) → frontend → docs (this set).
