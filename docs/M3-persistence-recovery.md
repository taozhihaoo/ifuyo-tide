# M3 — Persistence & Recovery (Torrent)

Three stores, one recovery coordinator (M3 §14/§15):

| Store | Owns |
|---|---|
| SQLite `torrent_tasks` | Tide task identity, source (magnet URI / metadata copy path), display name, storage root, lifecycle state, phase, progress snapshot, selection count, errors, recovery reason |
| SQLite `torrent_files` | per-file identity (`task_id + info_hash + file_index`, M3 §11) with `selected` flag |
| librqbit session | piece bitfield, fast resume, peers, tracker state |
| Filesystem | downloaded files under the storage root; metadata copy `torrent-metadata\<task-id>.torrent` |

## Startup flow (M3 §15/§84)

```text
App start → open DB → migrate → init TorrentEngine session
  → torrent recovery (core/torrent/recovery.rs):
      for each torrent_tasks row:
        torrent_file source → re-validate metadata-copy paths (M3 §12)
        magnet source       → reuse stored URI
        completed/cancelled → restore WITHOUT re-adding (terminal)
        otherwise           → re-add to session PAUSED (M3 §16/M2 §68:
                               startup never auto-resumes)
```

Idempotency (M3 §16): re-adding to the same session returns
AlreadyManaged — the session dedupes by info hash, so repeated starts
never duplicate downloads (proven by `duplicate_add_reuses_managed_torrent`).

## Integrity (M3 §85)

The stored metadata copy's SHA-256 is recorded at import; recovery
re-validates paths before re-adding. On resume after crash, librqbit
re-verifies pieces against the hashes inside the metadata (M3 §22) and
re-downloads corrupted pieces — proven by
`restart_with_corrupted_piece_redownloads`.

## Resume semantics

- Paused → stays Paused after restart; user resumes (M2 §68 rule).
- Ready → stays Ready (resumable, idle).
- Running at crash → recovery reclassifies to Ready (same as M2 HTTP
  tasks); the storage state survives in the librqbit-managed files.
- Completed → task restored WITHOUT re-adding to the engine.
