# M4(上) Range Gateway

The ONE Tide streaming gateway (§37–§50/§71), promoted from the M0.5
spike to production (`core/streaming/gateway.rs`). Hand-rolled HTTP/1.1
over a raw TCP listener — full control over status codes, headers and
framing, no web-framework surface area.

## Lifecycle & sessions

- Binds `127.0.0.1:0` (random loopback port) once per app, at startup.
- Sessions are UUIDv4 tokens in the path: `/stream/{token}`. Tokens are
  runtime-only (§70) and map to a registered `StreamingResource` —
  the HTTP layer never sees paths, it sees tokens (§59/§60).
- Shutdown: the accept loop watches a `watch` channel; dropping the
  gateway stops it. Per-connection tasks die with the process.

## Request validation (order matters)

1. Header size cap 16 KiB.
2. **Host** header must be loopback (`127.0.0.1`, `localhost`, `[::1]`)
   else 403 (§57) — DNS-rebinding and cross-origin probing rejected.
3. Token path filter: exactly one segment, no `/`, `\`, `:`, `%`, `..`
   — traversal, encoded traversal and UNC-ish shapes all 404 (§56).
4. Unknown/expired token → 404, empty body.

## Range semantics (§61/§62)

Single implementation: M1's `parse_range_header` (re-exported from
`core/streaming/range.rs` — no duplicate Range code).

| Request | Response |
|---|---|
| no Range | `200`, full body |
| `bytes=a-b` valid | `206` + `Content-Range: bytes a-(b-1)/len` (internal model `[start, end)`) |
| `bytes=a-` / `bytes=-n` | `206`, clamped to EOF |
| start ≥ len | `416` + `Content-Range: bytes */len` |
| malformed | `400` |
| multi-range | NOT parsed as multipart — full `200` body (controlled simplification, §62) |

`HEAD` returns the exact headers with no body. `Accept-Ranges: bytes`
is always advertised. All bodies `Connection: close` — one request per
connection, like the M1 test server; WebView2 re-opens as needed.

## Streaming, backpressure, memory (§93–§95)

- `STREAM_CHUNK` = 256 KiB: the gateway reads a chunk, writes it,
  advances. No whole-file buffering, ever.
- Backpressure is structural: the next chunk is read only after the
  previous `write_all` completes — a slow player paces the reads.
- Bounded memory per connection: one ≤256 KiB chunk in flight; no RAM
  buffering of missing ranges.

## Timeouts (§96/§97)

| Timeout | Value | Covers |
|---|---|---|
| resource wait / per-read | 120 s default (`set_read_timeout_ms` tunable) | torrent pieces that never arrive, dead HTTP origin — the read aborts, the connection closes early (bounded, structured abort) |
| session idle | 12 h TTL, reaped by `PlaybackManager::reap_expired` (§55) |
| shutdown | gateway task stops on the watch signal at process exit |

Connect timeout: loopback TCP connect is immediate; the per-read bound
covers the observable hang cases. The suite proves the bounded-wait
behaviour (`torrent_read_without_peers_is_bounded`).

## Security posture

Loopback-only bind + host check + opaque tokens means the gateway
exposes ONLY what the app explicitly registered for playback. The
integration security matrix (§56) covers: no host, hostile host, wrong
token, empty token, `/`-bearing token, `%2e`-encoded traversal,
`..`, non-`/stream` path — all rejected with empty bodies.
