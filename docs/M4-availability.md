# M4 Availability

What "available" means in Tide, how it is updated, and why the
interval set is deferred (ADR-2).

## Model

```rust
Availability { Complete, Partial { available_bytes }, Unknown }
```

plus the exact durable prefix via `StreamingResource::available_len()`:

| Source | available_len | Notes |
|---|---|---|
| File | declared len | complete local file |
| Part | PHYSICAL `.part` size (`metadata().len()`) | the M2 invariant: the durable file is the truth, never the DB number |
| Torrent | rqbit `progress_bytes` (clamped to file len; file len when finished) | coarse global approximation — byte-accurate mapping lives inside the FileStream |
| Http (origin) | 0 | unknown until read; reads are live |

## Update order (M4(下) §111/§112)

```
engine receives bytes
→ write_chunk (flush per chunk)
→ durable_len watch fired
→ waiting readers wake
→ (availability is only ever claimed AFTER bytes are flushed)
```

The gateway never hears "available" before the bytes are on disk: the
watch fires after `write_chunk` returns, and `.part` readers re-stat
the file. A reader therefore cannot observe a torn or half-written
region — every byte a reader sees was flushed before its offset was
announced.

## Why no AvailabilitySet yet (spec §10–§12)

The interval representation earns its cost the moment local
availability becomes non-contiguous. M4's three sources are:

- contiguous prefix (`.part`),
- all-or-nothing per file with librqbit-owned piece state (torrent),
- live origin (Http read-through).

Non-contiguous local availability arrives with random-range HTTP
storage (ADR-3). At that point: `AvailabilitySet` with
`is_available / available_subranges / missing_subranges /
contiguous_available_from`, interval merge/compact (§162), a
persistence decision (§87/§89), and the §161 storage benchmark
(10/100/1k/10k ranges) all become acceptance criteria. Building it
earlier would be dead weight (§153).

## Piece alignment (§163–§165)

Internally everything is byte ranges. The torrent adapter rounds to
pieces where necessary — but that stays inside librqbit: a demand for
`[10, 100)` may cause whole pieces to download, yet the gateway
serves exactly `[10, 100)`. Proven byte-exactly at odd offsets
(`MIB+13..MIB+13+4999` in the M4 test suite). The scheduler never
sees piece indices.
