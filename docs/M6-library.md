# M6(上) Library Architecture

The local library: filesystem → scanner → validated items → SQLite →
query/playback. Built strictly ON TOP of M4/M5 — the library adds no
media data path of its own (§101-style boundary kept from §3).

## Four concepts, never merged (§3)

| Concept | Answers |
|---|---|
| Task | "one download operation" (M1/M2/M3) |
| Resource | "readable bytes" (M4 StreamingResource) |
| LibraryItem | "a local entity the user manages long-term" |
| Playback | "one session" (M4/M5 PlaybackSession) |

## Module map (`core/library/`)

```text
model.rs       LibraryRoot / LibraryItem / enums / ItemQuery
path.rs        Windows path safety (P0): canonicalize + containment
classifier.rs  ONE extension → category/MIME mapping (§77)
repository.rs  ALL library SQL; scan commit = one transaction (§32/§67)
scanner.rs     filesystem enumeration, partial-scan safe (§15/§35/§68)
service.rs     validation + lifecycle policies; the only IPC facade
```

## Data model

`library_roots` + `library_items` (migration 0004). UNIQUE
`(root_id, canonical_path)` — scanning ten times yields the same rows
(§21). `task_id` is NULLABLE: imported files have no task (§31).
Availability is an ENUM (`available | missing | inaccessible`), not
booleans (§86). No scan-audit tables — runtime status +
`root.scan_status/last_scan_at` (§70).

## Scanner semantics (§68/§69)

Enumerate → validate/classify → ONE transaction: upsert candidates,
then mark previously-available unseen items Missing — EXCLUDING any
prefix the scan could not cover (unreadable dir, junction, IO error
→ recorded as *uncovered*, items under it keep their state). An
unreadable ROOT aborts the scan with NO database write at all.

Enumeration rules: reparse entries (junctions/symlinks) never
traversed nor indexed (§12 — detection is empirical: modern Rust
reports junctions with neither dir nor file bits, plus a
canonicalize-moves-the-path check); Tide artifacts (.part/.tmp/
.crdownload/.download/.torrent) never indexed (§16); directories are
never items (§28).

## Import chains (§26/§27)

- HTTP download completes → `on_complete` hook (fired AFTER the
  atomic rename + persisted state) → `import_download`: file exists →
  destination dir seeds a root if none contains it (§14) → item with
  `origin=download`, task_id, and the M1-VERIFIED checksum reused
  (§20 — never recomputed). Idempotent.
- Torrent task finishes → completion monitor (every 5 s) persists the
  terminal state — a pre-M6 gap: NOTHING detected torrent completion
  before — and imports each selected file as its OWN item (mode A,
  §27).
- Startup: light background verification (exists/metadata only, no
  hashing — §71/§73); the UI never waits for it (§72).

## Task ↔ Library lifecycle (§52/§53)

Task deletion NEVER deletes library items or files. Library item
removal NEVER touches the task. `remove_item` (index only) and
`delete_item_from_disk` (file + row, backend re-validated) are
separate IPC commands (§24).

## Playback chain (§46/§80–§83)

`play(item_id)` → service re-validates the filesystem RIGHT NOW
(exists, within root, not protected/reserved, size drift → refresh,
§48/§49) → StreamingResource::File (fully available, ranged) → the
M4/M5 PlaybackSession → the ONE gateway → the M5 player. A completed
HTTP file, a finished torrent file and an imported local file are the
SAME resource — one player, no per-origin branches (§80).
