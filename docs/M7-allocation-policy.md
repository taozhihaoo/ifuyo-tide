# M7 Allocation Policy

`compute(snapshot, config) -> Vec<AllocationDecision>` — pure,
deterministic (§51): now_ms travels INSIDE the snapshot; no clock, no
globals, no engine calls.

## Four phases (§55)

- **Phase A** classify: playback sessions → watermarked state machine;
  tasks → user-priority class or Normal; non-running → Maintenance (0).
  A task carrying a session takes the session's class (§54); a
  paused/ended session on a stopped task ends the demand.
- **Phase B** mandatory: Emergency minimums (2 MiB/s each) inside the
  global cap.
- **Phase C** target top-up: Emergency/Playback entries are topped to
  target (6 MiB/s); a CRITICAL/seek-stressed entry keeps absorbing up
  to 4× target — background is left "极低配额" (§31/§92).
- **Phase D** weighted fair split of the leftover: effective weight =
  base(class) × user multiplier + capped aging (Normal/Background
  only); background keeps a floor (20%) of the leftover when playback
  is fully healthy (§95) and drops to spare-only when stressed.

## Invariants (§166, property-tested)

sum ≤ cap; every allocation ≥ 0; one decision per task; non-running →
0; determinism (identical snapshot+config → identical output);
aging ≤ cap; background never permanently starved; emergency playback
obtains its minimum whenever the cap allows.

## Why decomposable reasons (§16)

Every decision carries reason code + detail (e.g. "buffer 8.0s, target
allocation") so "为什么 A > B" is always answerable in words — the debug
panel and tests render exactly these.
