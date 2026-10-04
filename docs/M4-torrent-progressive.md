# M4 Torrent Progressive Streaming

The torrent path: partial swarm data → playback, with librqbit as the
piece/integrity authority (ADR-4). Critical Acceptance #3/#4 chain.

## Flow

```
add torrent (M3) → metadata → file rows
→ create_playback_session(task_id, file_index)
   → repository lookup: info_hash + file_index + declared size
   → paused torrent is RESUMED (§52: Task=Paused + Playback=Playing
     is legal; §81: playback is strict priority)
→ Torrent resource (engine clone shares the librqbit Session)
→ gateway range read
   → independent FileStream per read (M0.5: concurrent head+moov
     ranges must not share a seeked stream)
   → seek to offset, read_exact — FileStream WAITS for missing pieces
   → byte-exact range regardless of piece boundaries
```

## Seek (Critical #4)

A seek to a distant position is a gateway read at that byte offset.
The FileStream blocks until the required pieces arrive — rqbit
receives the read pressure directly. `notify_seek` on the IPC side
cancels the stale queued demands (§80) so old-position bookkeeping
cannot pile up. Proven by the tail-range test: a read for the LAST
64 KiB of a just-started torrent blocks, the swarm delivers, the
gateway serves the exact bytes.

## Wait bounds (§97)

A torrent whose pieces can never arrive (no peers, no DHT) fails the
read after the bounded timeout — the connection closes with a
truncated body, the suite never hangs. `torrent_read_without_peers_is_
bounded` proves the bound.

## Integrity (§90/§91)

Tide never re-verifies SHA1 pieces: librqbit only exposes verified
bytes through the FileStream. The gateway is a byte pipe. Byte
preservation through the torrent path is asserted against the
seeder's known pattern.

## Task deletion (§102)

remove/delete-data invalidate every session owned by the task
(`close_for_owner`) — no session outlives its engine handle; a race
with an in-flight read surfaces as a structured failure, never a
dangling path.

## Known limitations (recorded)

- rqbit downloads pieces roughly front-to-back; there is no
  Tide-side piece-priority API — a far seek waits for the swarm (or
  local seeding) rather than jumping the internal queue. Measured
  behavior recorded; a smarter policy is M7 scope.
- `available_len()` for torrents is a coarse global-progress
  approximation; per-file byte-accurate availability would require
  re-deriving piece→file maps — not needed by any consumer today.
- Multi-file torrents stream individual files by index; cross-file
  byte spaces are not exposed (nobody asks for them).
