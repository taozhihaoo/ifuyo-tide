# M3 — librqbit Integration Notes

## Exact version & features

- **librqbit 9.0.1** (crates.io), locked in Cargo.lock since M0.5 and
  re-validated against the vendored source for M3.
- Features enabled: `disable-upload` (DownloadOnly guarantee, M3 §28).
- API verified against the vendored crate source — NOT docs or blog
  posts. Key facts that shaped the adapter:
  - `Session::add_torrent` takes `AddTorrent` + `AddTorrentOptions`
    (`initial_peers`, `output_folder`, `overwrite`, `list_only`,
    `only_files`, `paused`).
  - `SessionOptions.disable_upload: bool` exists only with the
    `disable-upload` cargo feature.
  - `ManagedTorrentHandle` is NOT publicly nameable (private module) —
    the adapter uses `Arc<ManagedTorrent>` (re-exported at root).
  - `add_torrent` on an already-managed info hash returns
    `AlreadyManaged` — the duplicate invariant (M3 §17) maps to this.
  - `list_only: true` turns `add_torrent` into a metadata probe
    (`ListOnlyResponse`) — used for inspection and pre-add path
    validation.

## API used

- `Session::new_with_opts(output_folder, SessionOptions)`
- `session.add_torrent(...)` → Added / AlreadyManaged / ListOnly
- `session.pause/unpause(handle)`, `session.delete(info_hash, delete_files)`
- `session.update_only_files(handle, &HashSet<usize>)`
- `session.with_torrents(|iter| ...)` for handle lookup
- `handle.with_metadata(|meta| ...)` — file list (path/len/offset)
- `handle.stats()` → progress/upload/error; `handle.name()`
- `session.create_and_serve_torrent` (TEST SEEDER only)

## Configuration / limitations

- Listener: TCP-only (uTP left off — M3 §27), UPnP disabled (M3 §26).
- DHT enabled in production; disabled in tests (loopback seeder).
- Known gotcha (from M0.5, re-verified): re-adding a torrent whose
  storage contains stale state with `overwrite: false` can stall —
  M3 always re-adds with `overwrite: true` (re-verify + resume).
- `piece_length` is not publicly readable from `Lengths` in 9.0.1 —
  read via `ValidatedTorrentMetaV1Info::info().piece_length` instead.
- File paths inside metadata are librqbit-parsed but Tide re-validates
  every one of them before accepting a torrent (M3 §12 — Tide's
  security boundary does not lean on third-party internals).

## Upgrade constraints

Re-run the M0.5/M3 verification battery when bumping: the 9.0.1 module
layout is private in several places (session/torrent_state/type_aliases)
and re-exports have changed across versions before.
