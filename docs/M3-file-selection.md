# M3 — File Selection

- Selection is per `(task_id, info_hash, file_index)` — never by name
  (M3 §11): names can change between metadata versions.
- `Select All / None / Individual` map to librqbit's
  `Session::update_only_files(handle, &HashSet<usize>)` (verified
  against 9.0.1 — not guessed).
- Selection persists in `torrent_files.selected` at add time and after
  every change.
- Partial selection CAN complete the task (M3 §82): Completed =
  "all SELECTED data complete"; unselected files are not downloaded and
  do not block the task (`multi_file_selection_downloads_selected_only`).
- The UI shows `Files (N)` with per-file state; the selected count
  summary comes from the engine snapshot, never fabricated.
