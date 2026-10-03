# M4(上) Persistence & Data-Integrity Boundaries

Decisions for M4(上) §83–§92: what M4 stores, what it deliberately does
NOT store, and which layer owns data integrity.

## What M4(上) persists: nothing new

No new tables, no new sidecars. Playback sessions and their capability
tokens are runtime-only (§70); restart re-creates everything.

## §83/§84 Range Writer — not needed yet, decision recorded

M1's HTTP engine is sequential-append (`.part` = contiguous prefix).
M4(上)'s HTTP streaming does not require random-range WRITES:

- in-progress HTTP download playback serves the `.part` prefix
  (progressive playback of what exists), and
- origin-side ranges are read live via ranged GET (Http resource).

A `RangeWriter` (write/read/flush per range, no clobbering of
unrelated bytes) becomes necessary the moment HTTP tasks gain
random-range download persistence (e.g. "download the moov first").
That is M4(下)/M6 decision space; the design constraint recorded here:
write range must never overwrite unrelated bytes, and availability
must come from the same metadata that survives crashes (§87/§89).

## §85/§86 Sparse files — not adopted

No sparse-file usage anywhere in M4(上); the `.part` remains a plain
sequential file. If random-range HTTP storage lands later, the
sidecar-range-map option (§86) is the default candidate on Windows —
sparse files would need explicit validation of allocation/read-hole/
recovery behaviour (§85's checklist) at that time.

## §87 Consistency with the M2 model

Untouched and re-verified by the full suite: SQLite + `.part` + resume
point = PHYSICAL file size. The File resource reads `.len()` from the
filesystem, never the DB number — the same invariant M2 recovery uses.

## §88 Torrent availability — rqbit is the source of truth

Tide keeps NO second bitfield. Availability for streaming comes from
librqbit's live stats via the engine adapter; piece/fast-resume state
is rqbit's persisted business. Tide's SQLite keeps the product view
(selection, task metadata) exactly as M3 defined it.

## §89 HTTP availability

No random-range HTTP availability exists in M4(上) (see §83 above);
the sequential `.part` needs only `downloaded_bytes`-style prefix
truth, which M2 already guarantees and crash-recovery reconstructs.
When random ranges arrive, availability MUST move to a persisted
range map (§89's requirement recorded as a precondition).

## §90/§91 Checksums & piece verification — ownership unchanged

The gateway is a byte pipe and re-verifies nothing: HTTP bytes are
exactly what the origin sent (M1 semantics: checksum verification
happens at task completion), torrent bytes are piece-verified by
librqbit before they are ever readable (Tide never re-hashes SHA1
pieces). `read_range` returns origin-verified bytes or an error.
