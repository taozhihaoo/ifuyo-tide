# M4(上) Test Matrix

## Results

- Rust: **222 tests, 0 failures** (165 unit + integration suites incl.
  the new `m4_streaming` 7/7).
- Frontend: **3/3** vitest suites pass (`pnpm test`).
- Full regression: M0–M3 suites unchanged and green.

## Coverage map (spec section → test)

| Requirement (M4(上)) | Where proven |
|---|---|
| §37–§50 gateway: loopback bind, tokens, single-range 200/206/416/400, bounded chunks | `m4_streaming::range_head_and_full_semantics`, `unified_gateway_serves_file_http_and_torrent`; spike gateway suite (12 tests) still green |
| §53/§55 stop/expire revokes access + demands | `stop_revokes_gateway_and_cancels_demands` |
| §56/§57 security matrix | `gateway_security_matrix` (+ spike gateway security tests) |
| §61/§62 range validation, multi-range policy | `gateway_security_matrix` (400/416/200-full cases), M1 parser unit tests |
| §64/§65 real player, no fake UI | `VideoPlayer.tsx` reads element events only; vitest suites guard the app shell |
| §68–§70 IPC surface, runtime-only tokens | `commands/playback.rs`; no session data in any migration |
| §71 demand chain runs end-to-end | `torrent_read_waits_for_pieces_then_serves` (tail range WAITS, then byte-exact 206) |
| §72/§74 HTTP + Torrent + File unified through ONE gateway/manager | `unified_gateway_serves_file_http_and_torrent` (same port, same session model, byte-identical semantics) |
| §75 priority ordering | demand unit tests (`playback_beats_background`, `seek_reprioritizes_to_critical`, `oldest_wins_at_equal_priority`, `resources_are_independent`) |
| §80 obsolete demand on seek | `seek_cancels_stale_demands` |
| §92 byte-preserving streaming | pattern assertions in every body check (`pattern_byte(i)` at exact offsets) |
| §96/§97 bounded waits, no one-global-timeout | `torrent_read_without_peers_is_bounded` (1.5 s timeout, truncated body, < 25 s) |

## §82 interaction semantics, current status

| Case | Status |
|---|---|
| A download + playback | covered (File `.part` path; gateway concurrent with engine) |
| B playback → pause | native element pause; session/gateway unaffected (no IPC by design) |
| C playback → seek | `seek_cancels_stale_demands` |
| D playback stop | `stop_revokes_gateway_and_cancels_demands` |
| E download pause while playback | download pause semantics unchanged (M1); session keeps serving what exists — behavioural soak in M4(下) |
| F shutdown while playback | sessions are runtime-only; process exit drops them — nothing to clean on next start |
