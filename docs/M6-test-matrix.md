# M6(上) Test Matrix

## Automated results (M6(上) final)

- Rust: **253 tests, 0 failures** (`cargo test --locked`), including
  the new `m6_library` suite (11 tests) and the path/classifier unit
  tests.
- Frontend: **25 tests, 0 failures** (`pnpm test`), including 4 new
  LibraryView tests; `tsc --noEmit` clean.
- `cargo fmt --check` clean; clippy clean in M6-owned code.

## Coverage map (spec section → test)

| Requirement (M6(上)) | Where proven |
|---|---|
| §11/§12 canonical paths, component-wise containment, `Library-Evil` trap, reserved names | `path.rs` unit tests |
| §12 junctions never traversed/indexed | `junctions_are_not_traversed` (real NTFS junction via mklink) |
| §13 no drive roots / system dirs | `root_validation_rejects_system_locations` |
| §16 artifact exclusion | `scan_is_idempotent_and_classifies` (.part/.tmp absent) |
| §17/§18/§77/§78 classification central, extension ≠ playability | classifier unit tests; playability stays with the player |
| §21 scan idempotency | `scan_is_idempotent_and_classifies` (second scan: 0 added, 0 missing) |
| §22/§23 missing → restore | `scan_marks_missing_and_restores` |
| §24 remove ≠ delete | `remove_keeps_file_delete_removes_it` |
| §25 backend re-validation for delete | `delete_rejects_paths_outside_the_root` (forged row → SECURITY, file untouched) |
| §26 download → library, §20 checksum reused, §14 root seeding | `download_import_is_idempotent_and_keeps_origin` |
| §27 torrent mode A | `torrent_import_one_item_per_file` |
| §41–§45 search/filter/sort/paging in SQL | `queries_page_filter_and_sort` |
| §46/§59 item id → capability URL, no paths to the renderer | playback command + `library_item_plays_through_the_gateway` (byte-exact 206) |
| §48/§49 play-time filesystem re-validation | service `play_source` + forged-row test |
| §68/§69 uncovered prefixes never produce Missing | `uncovered_directory_is_never_marked_missing` |
| §80–§83 one resource for all origins, Range, full availability | `library_item_plays_through_the_gateway` + M4 File-resource suites |
| §38–§40 background scan + throttled raw-count events | command spawns `spawn_blocking`; events carry counts only; UI shows "Scanning… N files" (frontend test asserts no fake percentage path) |
| §54–§56 UI structure/actions | LibraryView vitest suite |

## M6(下) additions (`tests/m6_library_lower.rs`, 15 tests + benchmark)

| Requirement | Where proven |
|---|---|
| §96 duplicate path → exactly one item | m6上 idempotency suite |
| §97 Missing ⇄ Available via refresh | `refresh_restores_state_both_directions` |
| §98/§69 partial scan safety | m6上 uncovered test + `failure_injection_metadata_read_is_partial` |
| §99 download completed ×N → one item | `download_import_is_idempotent_and_keeps_origin` |
| §100 task deletion keeps items | `task_lifecycle_never_touches_library` |
| §101 remove keeps file | `remove_keeps_file_delete_removes_it` |
| §102 delete = confirm + backend validation + file gone + row gone | m6上 delete suite + frontend confirm test |
| §103 path escape (forged row, junction) | `delete_rejects_paths_outside_the_root` + `junctions_are_not_traversed` |
| §104 playback security (fake id / id-shaped path) | `playback_with_fake_item_id_fails_safely` |
| §105 >4 GiB sparse file, u64/offsets/range intact | `large_sparse_file_beyond_4gib` |
| §108 filesystem race → structured failure, no stale truth | reconciler + play_source re-stat; injection tests |
| §109 failure injection (3 points) | `failure_injection_*` (before-DB untouched, after-commit keeps, partial meta) |
| §110 property fuzz: nothing escapes containment | `fuzzed_paths_never_escape_containment` (500 seeded hostile paths through the real validator) |
| §130 cancellation commits nothing | `cancelled_scan_commits_nothing` |
| §131 same-root single-flight | `same_root_scan_is_single_flight` |
| §135 locked file → structured failure | `locked_file_delete_fails_structured_not_panic` |
| §136 playing → FILE_INUSE refusal | `playing_file_refuses_deletion_with_file_in_use` |
| §95 duplicate root / FK / corrupted DB fails safe | `duplicate_root_returns_existing`, FK error observed in injection setup, `corrupted_database_fails_safe` |
| §145 migration regression v3→v4 | `migration_regression_v3_to_v4` |

## §151 matrix status

Every row of the spec's matrix is covered by the mapped tests above;
"Performance 100k measured" = the §106 benchmark (below).

## Performance benchmark (§106/§107, dev machine, recorded)

| Measurement | Result |
|---|---|
| Scan 1,000 real files | 57 ms |
| Scan 10,000 real files (nested) | 627 ms |
| Insert 100,000 rows (batched transactions) | 9.1 s |
| Search "movie" at 100k, p50 / p95 (50 samples) | 50 ms / 59 ms |
| Filter + sort at 100k | 276 ms |
| Idempotent re-commit of 100k | 9.6 s (still exactly 100k rows) |

Conditions: debug-profile test binary, dev NVMe SSD, LIKE search per
§76 (no FTS — not yet justified by these numbers). Reproduce with
`cargo test --locked --test m6_library_lower benchmark -- --ignored
--nocapture`.

## Manual verification items

- §111 in-window scenarios (A–G) against the real WebView2 UI: the
  automated layers cover every step; the visual click-through remains
  the standing manual pass (same disclosure as M4/M5 reports).
- Explorer-driven move out + back while Tide runs (§152 D/E).
