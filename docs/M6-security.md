# M6(上) Library Security

Path safety is THE M6(上) P0 area (§11/§12/§25). Everything here is
enforced in `core/library/path.rs` + the service and covered by
tests.

## Trust model

A path enters the database ONLY through `canonicalize` (resolves
`.`, `..`, separators, junctions/symlinks, drive-letter casing). A
frontend-supplied directory string (from the picker) is treated as
untrusted input: validated + canonicalized in the service before
anything is stored (§11). Every later USE (play, delete, reveal)
re-canonicalizes and re-validates — the DB row is metadata, never a
permission proof (§10/§48).

## Containment, done right (§12)

Containment is COMPONENT-WISE and case-insensitive: `C:\Library-Evil`
is NOT inside `C:\Library` (the string `starts_with` check would
wrongly accept it — the spec's exact trap, unit-tested). A forged
row whose `canonical_path` points outside its root is refused by BOTH
play and delete with `SECURITY` (integration-tested with a real
forged row).

## Windows specifics

- Reserved device names rejected as items/roots: CON PRN AUX NUL
  COM1–9 LPT1–9, including with extensions ("CON.txt") and any case.
- Protected locations: the Windows directory and Program Files
  variants can never be roots or items.
- Whole drives cannot be library roots (§13 — no automatic C:\ scans).
- Junctions/symlinks are never traversed nor indexed; empirically,
  modern Rust reports junctions with neither dir nor file bits, so
  detection = "neither dir nor file" OR canonicalization moved the
  path. A real NTFS junction pointing OUTSIDE the root is proven
  non-indexed by an integration test.

## delete_item_from_disk (§25)

Backend-only flow: item id → DB canonical path → canonicalize → is
regular file → inside its (existing) root → not protected/reserved →
`remove_file` → row removed. The renderer can never pass a path.
Any playback session owned by the item is invalidated first (§102
policy from M4).

## Gateway boundary (§58/§59)

The library adds NO HTTP surface. Playback goes item id → validated
path → PlaybackSession → the M4 gateway, unchanged (loopback, tokens,
host check, range validation). There is no `/library?path=...` — the
gateway cannot be turned into a file server by URL manipulation.

## Logging & privacy (§64/§65)

Library logs carry event names + counts + item ids ONLY — never
absolute paths, URLs, magnets, or hashes (e.g. `library.play` logs
the item id, not `C:\Users\...\Movie.mp4`). Everything stays in the
local SQLite; no telemetry, no cloud indexing (§65).
