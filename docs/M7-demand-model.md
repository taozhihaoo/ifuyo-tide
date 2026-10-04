# M7 Demand Model

A Demand is "谁需要哪个字节范围、多急" (§5). Two layers:

1. **Runtime registry** (M4 `core/streaming/demand.rs`): per-chunk
   bookkeeping — submit/merge/cancel/complete with (resource, range,
   reason) dedup. The gateway, seek and session-close flows drive it
   (§132 idempotency tested).
2. **Policy intent** (M7 `core/scheduler/model.rs`): aggregated views —
   TaskView (engine/state/user-priority/smoothed bps/waited) and
   PlaybackView (REAL browser-buffer seconds). DemandKind (§6) maps
   onto PriorityClass by the policy, never by callers.

Range awareness (§9/§105): `RangeSet` (half-open, normalized) provides
merge/subtract/intersect/`missing_from` — Demand − Availability =
Missing = what actually needs scheduling (§106 exact example tested).

Lifecycle (§144–§146): vanished tasks lose registry entries, stamps and
rates on the next tick (garbage-guard tested); ended playback →
Maintenance class → zero allocation; a re-appearing task is scheduled
from scratch (restart/rebuild semantics, §129).

No implicit downloads (§89): a playback demand NEVER creates a task;
"full download" stays the user's explicit M1 task (§149/§150) — and a
task carrying BOTH (playback + full download) is one entry with one
allocation (§151/§152: no duplicate transfers, by construction).
