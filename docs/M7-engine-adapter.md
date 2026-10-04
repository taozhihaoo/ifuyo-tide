# M7 Engine Adapters

§81 contract: `capabilities() / apply_decision() /
observe_throughput_bps()`.

## HTTP adapter
per-task bandwidth = YES (token bucket read by the M1 chunk loop —
bounded sleep keeps pause/cancel responsive); range prioritization =
YES (M4 origin read-through already serves arbitrary ranges). The
adapter holds the bucket registry; the runtime writes rates through it
via `DownloadEngine::set_task_rate_bps`.

## Torrent adapter
per-task bandwidth = NO — librqbit 9.0.1's PUBLIC
`Session.ratelimits.set_download_bps` is session-level; piece
prioritization = rqbit's own streaming selector (§12/§35: no fork, no
private modules). The adapter records the limitation and applies the
session rate; the runtime sums all torrent allocations into it.

## Degradation (§83)
An unsupported decision NEVER panics: adapters return a degradation
note (e.g. "no limiter registered (not running?)", "session-level
limitation") and the policy's reasons explain the effective behavior.
