# M4(上) Demand Scheduler

The minimal scheduler of M4(上) §15–§19/§75 — deliberately NOT smart.
Scoring, EWMA throughput optimization, predictive models: M7 scope
(§75 says so explicitly).

## Model

A Demand = "which consumer needs which bytes, how urgently":

```rust
Demand { id, resource_id, range, priority, reason, created_at, state }
```

- `reason` → default priority: PlaybackSeek/PlaybackInitial = Critical,
  PlaybackBuffer = High, Recovery = Normal, BackgroundDownload = Low.
- states: Pending → Serving → Satisfied | Cancelled.

## Algorithm (§75)

`next_for(resource)`: highest priority first, oldest first within a
priority (oldest = `created_at`, a stand-in for "earliest deadline" —
M4 has no deadlines yet). `submit` dedupes identical
(resource, range, reason) entries. `cancel` / `cancel_resource` /
`reprioritize` / `complete` round out the surface.

## Who uses it today

| Caller | Behaviour |
|---|---|
| Gateway chunk reads | submit before read, complete on success, cancel on failure/client abort |
| `stop_playback_session` / session close | `cancel_resource` — queued demands are obsolete |
| `notify_seek` (player seeked) | `cancel_resource` — old-position demands dropped (§80), new range re-queued and wins |
| TTL reaping | same as close |

## Honest limits

The scheduler is bookkeeping + invalidation, not a byte-flow throttle:
it does not pause background downloads while playback is blocked
(§81's strict-priority provision is satisfied differently for now —
torrent downloads continue in librqbit; playback reads wait on pieces
independently and are never queued BEHIND background work). Wiring
demand priorities into the download engines' bandwidth decisions is
M7's job. §82's interaction matrix is exercised by the M4(上)
integration tests at the semantics level (stop/seek/pause), with the
full behavioural matrix completed in M4(下).
