# M4 Architecture Decision Records

The ten decisions M4(下) §152 requires. Each states the decision, the
alternatives rejected, and the consequence.

## ADR-1 — `Resource.read(range)` semantics

**Decision**: `read_range(range) -> Result<Vec<u8>>` returns EXACTLY
the requested bytes or a structured error — never a short read, never
zeros for missing data. Reads WAIT for availability, bounded by
`DEFAULT_READ_TIMEOUT` (120 s) or the frontier grace window.

**Rejected**: returning partial buffers (pushes protocol complexity
into every caller); returning zeros for missing ranges (silently wrong
bytes — P0 class).

## ADR-2 — Availability representation

**Decision**: coarse `Availability { Complete, Partial{available_bytes},
Unknown }` plus `available_len()` for the exact durable prefix. The
interval-based `AvailabilitySet` (spec §10–§12) is deferred until a
feature needs non-contiguous local availability (random-range HTTP
storage); today all sources are contiguous-prefix or live-origin.

**Rejected**: `downloaded_bytes` disguised as full availability
(assumes `0..X` is always the only available interval); full
AvailabilitySet now (no consumer, storage + merge + persistence cost
with zero callers).

## ADR-3 — HTTP random-range storage strategy

**Decision**: NO local random-range writes in M4. In-progress
downloads serve their contiguous `.part`; beyond-frontier ranges are
read through to the origin (not persisted). A `RangeWriter` becomes
mandatory the day random ranges must survive restart; constraints are
recorded in M4-persistence-boundaries.md.

**Rejected**: implementing a sidecar range map speculatively (§202:
no scope expansion without a blocking need).

## ADR-4 — Torrent adapter strategy

**Decision**: the librqbit `FileStream` IS the torrent reader: opens
an independent stream per read (WebView2 issues concurrent ranges;
a shared seeked stream corrupts them — M0.5 measured), waits for
missing pieces internally, and returns exact byte ranges regardless of
piece boundaries (piece mapping stays inside librqbit, §163–§165).
Tide keeps no second bitfield; rqbit is the integrity authority.

## ADR-5 — Gateway binding/security

**Decision**: raw TCP HTTP/1.1 server bound to `127.0.0.1:0` (random
loopback port, per app run). Host-header must be loopback; opaque
UUIDv4 path tokens; GET/HEAD only. No web framework.

**Rejected**: binding `localhost`-resolvable wildcard (LAN exposure
risk); fixed port (collisions, predictable); Tauri custom protocol
(couples media bytes to the IPC surface; harder to reason about
range semantics — see ADR-8 for why a local HTTP gateway at all).

## ADR-6 — Session token strategy

**Decision**: UUIDv4 tokens generated at session creation, held in
memory only, revoked on stop/delete/reap (close signals a per-session
cancellation watch). Nothing about sessions persists to SQLite;
restart regenerates everything.

## ADR-7 — Scheduler priority model

**Decision**: four fixed classes (Critical > High > Normal > Low),
reason-based defaults (seek/initial = Critical, buffer = High,
recovery = Normal, background = Low), oldest-first within a class;
dedup/merge of identical pending demands; eager removal of terminal
demands. No scoring, no ML — M7's territory.

## ADR-8 — Browser/player integration model

**Decision**: WebView2 `<video src=http://127.0.0.1:{port}/stream/{token}>`
— media bytes flow over HTTP Range to the media element, never over
Tauri IPC (§105: no `invoke(read_bytes)` pulling). The M0.5 spike
measured this end-to-end (play/seek/buffer events all working through
a loopback gateway) — M4 productionizes exactly that shape.

**Rejected**: IPC byte-pulling (IPC overhead + JS memory copies +
serialization per chunk); custom URI scheme handlers (unproven range
semantics in WebView2).

## ADR-9 — Single-range vs multi-range

**Decision**: single byte range only. Multi-range requests are NOT
parsed as multipart — they serve a full 200 body (a controlled,
honest degradation the M1 parser classifies as `Unsupported`).
M0.5 showed WebView2 does not require multipart for playback.

## ADR-10 — Cancellation semantics

**Decision**: three cancel axes, all explicit: (1) a DEMAND is
cancelled/merged by seek or session close; (2) a SESSION close
revokes gateway access, cancels queued demands AND aborts in-flight
reads via a per-session watch; (3) task deletion invalidates every
session owned by that task (§102 policy: "session invalidated").
Shutdown order: gateway stop → sessions revoked → demands dropped →
resource waits abort via the same watch → process exit closes
sockets.
