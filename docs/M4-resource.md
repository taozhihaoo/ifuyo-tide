# M4 Resource Architecture

How the streaming layer is put together: one readable-resource model,
one demand scheduler, one range gateway, one playback manager — the
same path for a finished file, an in-progress HTTP download (including
its un-downloaded region), and a torrent file (M4(上) §72/§74).

Companions: M4-availability (availability semantics), M4-demand,
M4-stream-gateway, M4-playback, M4-http-progressive,
M4-torrent-progressive, M4-adr, M4-security.

## Module map

```text
core/streaming/
├── resource.rs   StreamingResource: File / Part / Torrent / Http
├── demand.rs     Demand model + minimal priority scheduler
├── gateway.rs    THE loopback Range gateway (one per app)
├── playback.rs   PlaybackSession manager (owns the gateway)
├── range.rs      re-exports the M1 Range parser (single source of truth)
└── mod.rs        content_type_for_name()
```

Wiring: app startup creates `DemandScheduler` → `PlaybackManager`
(gateway binds 127.0.0.1:0 here) → `AppState.playback`. IPC commands in
`commands/playback.rs` resolve resource IDs to resources through the
trusted repositories and open sessions.

## StreamingResource — enum, not trait

The M0.5 spike documented that async-trait objects would need
`async_trait` boxing for no benefit; M4(上) keeps the spike's enum
shape, promoted to production:

- **File** — a completed download's final file or any authorized
  local file. Reads use physical file size (M2 invariant: the durable
  file is the truth, never the DB number).
- **Part** — the `.part` of an IN-PROGRESS HTTP download: dynamic
  durable length, event-driven frontier wait (durable-len watch fired
  after every flushed chunk, M4(下) §112/§115), and origin
  read-through for beyond-frontier seeks (M4(下) §127). Full model in
  M4-http-progressive.md.
- **Torrent** — resolved via `TorrentEngine` (Clone shares the librqbit
  `Arc<Session>`). Every `read_range` opens an INDEPENDENT
  `FileStream`: WebView2 issues concurrent range requests (head +
  moov-at-end) and a shared seeked stream would corrupt them — the
  M0.5 measured finding. Missing pieces WAIT, bounded by the read
  timeout (120 s default).
- **Http** — ranged GET against the origin (`Accept-Encoding: identity`
  per M1 semantics), used for play-while-downloading from the source
  itself.

## Read model

`read_range(range)` returns exactly the requested bytes or an error —
never short silent reads. Hard caps: 8 MiB per read (gateway chunks at
256 KiB, so the cap is never hit through the gateway), bounded waits
everywhere. Gateway reads are byte-preserving (§92): resource bytes =
gateway response bytes = player input bytes; Tide never transcodes.

## Availability (honest scope)

M4(上) ships the coarse `Availability { Complete, Partial{available_bytes},
Unknown }`. The spec's interval-based `AvailabilitySet` (§10–§12) is
designed for and deferred with the Range-Writer work (see
M4-persistence-boundaries.md): today no consumer needs sub-range
availability — the gateway simply reads and WAITS, and torrent piece
availability stays librqbit's business (§88, no second bitfield).

## The §71 chain, as actually built

```text
WebView2 GET Range → Gateway → (Demand.submit for the chunk)
  → resource.read_range → [torrent: FileStream WAITS on pieces]
  → swarm delivers bytes → read completes → Demand.complete
  → 206 bytes to the player
```

The scheduler records/cancels demands (stop/seek drop obsolete ones,
§80); the blocking wait itself lives in librqbit's FileStream, which
the integration test `torrent_read_waits_for_pieces_then_serves`
proves end-to-end (tail-of-file range served byte-exact after the
swarm catches up).
