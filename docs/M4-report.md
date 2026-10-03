# M4(上) Completion Report

## 1. Executive Summary

M4(上) delivers progressive streaming: `task/file/torrent →
StreamingResource → DemandScheduler → ONE loopback Range Gateway →
WebView2 <video> → play/seek`. A download's `.part`, a finished file
and a torrent file stream through the same gateway with identical
range semantics; the player is one real component with zero fake
state. 222 Rust tests (incl. 7 new integration tests over a real
loopback swarm) and 3 frontend suites pass.

## 2. FACTS

- Rust: 165 unit + 57 integration tests, 0 failures (`cargo test
  --locked`).
- Frontend: `pnpm test` 3/3; `tsc --noEmit` clean.
- New integration tests run a REAL seeder + production engine; the
  bounded-wait test proves no infinite hang when pieces never arrive.
- All M0/M1/M2/M3 suites pass unchanged (no regressions).

## 3. Shipped

| Component | Location |
|---|---|
| StreamingResource (File/Torrent/Http) | `core/streaming/resource.rs` |
| DemandScheduler (4 classes, dedup/cancel) | `core/streaming/demand.rs` |
| Range Gateway (ONE per app) | `core/streaming/gateway.rs` |
| PlaybackManager (sessions, TTL, revocation) | `core/streaming/playback.rs` |
| Range parser reuse | `core/streaming/range.rs` → M1 parser |
| IPC (create/stop/get session, notify_seek) | `commands/playback.rs` |
| ONE VideoPlayer + Play buttons | `src/features/player`, Downloads/Torrents views |

## 4. Architecture decisions

See the companion docs: M4-streaming-architecture (enum-not-trait,
independent stream per read), M4-demand-scheduler (minimal by design),
M4-range-gateway (security, range policy, timeouts, backpressure),
M4-playback-session (lifecycle, IPC minimality, no-fake-UI),
M4-persistence-boundaries (nothing new persisted; RangeWriter deferred
with recorded constraints; rqbit owns torrent integrity).

## 5. Doc-drift rulings (doc vs code, code wins)

- §68 lists pause/resume playback IPC — NOT built: §68's own principle
  ("能由 WebView2 原生事件直接完成的，不要多造 IPC") rules them out;
  the media element handles pause/resume natively. `notify_seek` IS
  built (demand invalidation is backend business).
- §10–§12 interval AvailabilitySet: coarse availability shipped;
  intervals deferred with the Range-Writer feature they depend on
  (constraints recorded in M4-persistence-boundaries).
- §81 playback-priority throttling of background downloads: not wired
  into engines in M4(上) (scheduler tracks/cancels demands; bandwidth
  policy is M7). Playback is never queued behind background work —
  torrent reads wait directly on librqbit pieces.

## 6. Security

Loopback-only bind; Host-header validation; opaque runtime UUID
tokens; path/traversal-encoded token rejection; range parsing via the
M1 parser (negative/overflow/multi-range hardened there). No CORS
headers (§58: WebView2 needed none in M0.5; unchanged). New attack
surface covered by the §56 matrix integration test.

## 7. What M4(上) intentionally does NOT close

Per the spec split note, the following live in M4(下): structured
error model pages for streaming (§97's structured errors get their
codes there), CORS/WebView2 re-validation pages, performance
benchmark + chunk-size tuning data, the full §82 behavioural matrix
soak, and the final M4 DoD checklist. M4(上) alone is explicitly NOT
M4-complete (spec header).

## 8. Commits

1. `feat(streaming)` — core: resource/demand/gateway/playback.
2. `feat(playback)` — IPC commands + app wiring (+ invoke_handler
   dedup fix).
3. `feat(player)` — frontend: one player, Play buttons, real state.
4. `test(streaming)` — M4 integration suite + test fixes.
5. `docs(streaming)` — this documentation set + README status.
