# M8 Lifecycle: tray, background mode, exit semantics

(§13/§14/§22/§23/§28/§132 — the states that Windows makes easy to
conflate, kept strictly separate.)

## Runtime modes (§28)

`Foreground` (window visible) / `Background` (window hidden, process
running) / `TrayOnly` (== Background with a tray icon present) /
`Exiting`. These are RUNTIME LIFECYCLE states — deliberately not
merged with Task/Download states (§28: no second task state machine).

## Window hidden (§14.1)

Downloads, torrents, the scheduler, persistence and recovery
checkpoints all CONTINUE. The frontend stops being the center of
attention: it only re-renders on its existing throttled event streams
(§14.2 — no new timers were added; the tray updates are event-driven
string swaps, §53).

## Close requested (§13.3/§22.2)

`win:close_to_tray` (default ON): the close is prevented, the window
hides, the app keeps running. Setting it OFF restores native behavior
— close really exits. Taskbar minimize is untouched (minimize never
tray-hides).

## Quit (§22.1)

Tray → Quit Tide calls `app.exit(0)`: the runtime loop, engines and
tray die with the process; M2's durable persistence means state was
ALREADY safe at every checkpoint (§24: graceful shutdown is an
optimization, never a recovery requirement). Windows
shutdown/logout/restart exercises the same path via the OS
(§23 — no long synchronous waits were added).

## Startup order (§73)

```text
process start
→ single-instance check (2nd instance exits here)
→ core state (db, recovery, engines, scheduler, tray)
→ window create/restore (window-state plugin)
→ frontend ready
→ drain pending launch requests (§74)
```

Requests arriving before "frontend ready" wait in the pending queue —
none are lost (§74 tested). Multiple simultaneous requests are all
kept and confirmed one by one (§75).
