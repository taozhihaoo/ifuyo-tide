# M5 Playback Session

How the M4 session became the player's backend contract — by
EXTENDING it, not redesigning it (M5 §91).

## Fields (M5 §8, as actually kept)

`session_id, url (gateway capability), size, content_type, source_kind,
available_bytes (live), capability, created_at_ms, last_position_ms`.

Duration is NOT on the backend session: the media element owns real
duration (§25 — the backend would have to parse containers to know
it, which Tide must not do). Unknown duration renders `--:--`.

## Additions in M5

1. **`active_session()` + `get_active_playback_session` IPC** — the
   most recent still-alive session. Frontend reload recovery (§37):
   React state is a projection; the backend registry is the truth.
2. **`report_playback_position` IPC + `last_position_ms`** — the
   player reports its position every 5 s while playing (§36).
   Session-scoped only: it dies with the session, it is not history,
   it never leaves the process (telemetry-wise: local diagnostics
   only, §58).
3. **Position resume** — when a session is adopted (reload, or return
   to the Player view), the controller seeks to `last_position_ms`
   once metadata is loaded, if it is inside the duration.

## Lifecycle & cleanup (§70)

- Opened per Play action; ONE session at a time (the app stops the
  previous on switch — Test 7 proves the old gateway URL dies).
- Stopped by: explicit Stop, source switch, owning task deletion
  (M4 §102), 12 h TTL reaping, app exit (runtime-only by design).
- "Frontend reload / close page" deliberately does NOT kill the
  session — that is the reload-recovery feature; zombie risk is
  bounded by TTL reaping and owner invalidation.
- Controller cleanup: dispose removes every media listener and timer
  (§87); stop releases the element source.

## Security unchanged (§52/§53/§15)

The player can only open backend-created sessions (resource id →
authorize → capability URL). It has no path-shaped API at all; the
gateway/session security model is byte-identical to M4 (M4 security
tests still green).
