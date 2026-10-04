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

## Manual verification items

- Explorer-driven moves while Tide is closed → next scan marks
  Missing → move back → restored (automated equivalent covers the
  state machine; the Explorer interaction itself is manual).
- 50k/100k-file scale benchmark (§74/§75): the algorithmic shape
  (single transaction, temp seen-table, indexed queries) is built for
  it; the recorded benchmark run belongs with M6(下)'s performance
  section.
