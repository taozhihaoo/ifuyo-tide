# M8 Report

## 1. Executive Summary

M8 completes Windows host integration: single instance with safe
second-launch forwarding, a validated LaunchRequest boundary (CLI /
tide:// deep link / .torrent association), system tray with close-to-
tray background mode, notifications, autostart, window-state restore,
official opener for native open, and an NSIS installer that registers
the deep link and .torrent association. 313 Rust tests (incl. 12
platform fuzz/classification tests) + 25 frontend tests green; NSIS
installer builds; release exe builds. Windows is strictly a host
boundary — no second business layer exists (§96/§157).

## 2. FACTS

- 313/0 Rust (incl. 12 platform tests), 25/0 frontend, tsc clean,
  fmt clean, clippy zero warnings in platform code, `pnpm tauri
  build` PASS producing exe + NSIS installer.
- Plugins at locked Tauri-2 versions: single-instance 2.5.2,
  deep-link 2.6.1, autostart 2.7.0, notification 2.5.1, window-state
  2.5.0, opener 2.7.0 (§66 — no version bumps).
- Capabilities added minimally: notification (3), autostart (3),
  deep-link (1). No filesystem/shell/process grants (§19).
- NSIS chosen as THE installer (§50 single-installer decision);
  .torrent association registered (Editor role → confirm flow);
  tide:// scheme registered via deep-link plugin config.

## 3. HYPOTHESES

- Close-to-tray + tray behaviors behave identically in dev and
  installed builds (plugin docs say notifications/associations differ;
  the installed-build pass is manual §33).
- Watermark-level UX (§77 DPI/multi-monitor) holds via the official
  window-state plugin's bounds validation.

## 4. INFERENCES

- Autostart registry behavior is fully delegated to the plugin
  (per-user HKCU Run key) — per §65, magnet association was
  deliberately NOT registered (only tide://), avoiding the
  default-handler question entirely.

## 5–15. Architecture & components

docs/M8-windows-integration.md carries the §91 architecture diagram
and per-component design: single instance (§8), LaunchRequest (§9),
deep link (§10), association (§11), tray (§13), background modes
(§14/§28), notifications (§15), autostart (§16), window state (§17),
native open (§18), installer (§20/§21), lifecycle/exit (§22/§23/§73/§74).

## 16. Security Audit (§89)

docs/M8-security.md — S1..S10 all mitigated + tested where
automatable: deep-link scheme/param allowlist, no shell execution
(opener plugin replaced the raw explorer spawn), path classification
fuzz suite, duplicate-runtime prevention via single-instance-first
registration, notification failure isolation, uninstall policy
(NSIS defaults; user downloads never touched), log redaction
(launch_request logs carry type+code only).

## 17. Test Matrix

docs/M8-test-matrix.md — §34–§40 matrices mapped; §40 fuzz battery
automated; §34–§39 OS-behavior rows recorded as the manual
installed-build pass (honest disclosure per §169).

## 18. Performance (§52/§53/§90)

No new timers/polling: tray updates are event-driven string swaps;
scheduler tick unchanged (807 µs @1000 tasks); handoff is one IPC per
launch. Startup order (§73) unchanged for core (db → recovery →
scheduler before window).

## 19–20. Files / Dependencies

New: `platform/{mod,launch,router,tray,settings}.rs`,
`commands/windows.rs`, frontend `settings/{windowsApi,notify}.ts`.
Modified: app/{mod,state}.rs, commands/{library,mod}.rs, lib.rs,
tauri.conf.json, capabilities, Cargo.toml/lock, Settings/App/Downloads
frontend. Dependencies added: 6 official Tauri plugins (§66/§161 —
each justified by a direct M8 requirement; no other crates added).

## 21–28. Status

- RESOLVED: raw-shell open replaced by opener plugin; single-instance
  registration order; second-instance argv fuzzing.
- CONFIRMED: single-instance callback signature (&AppHandle, argv,
  cwd) at the locked version; NSIS single-file association support.
- OPEN (manual, installed build): §88 scenarios A–G, §36/§37/§38/§39
  OS rows, §83 full uninstall loop — recorded, not silently assumed.
- WORSENED: nothing — all pre-M8 suites green.
- P0: 0. P1 (recorded): magnet: association NOT registered (deliberate
  §65 — tide:// covers browser handoff; direct magnet registration
  needs its own default-handler verification); installed-build manual
  matrix pending on a clean machine (§33); per-machine install not
  exercised (NSIS per-user default satisfies §56).

## 27. M9 Readiness

Settings surface, tray, notifications and lifecycle semantics are the
Polish milestone's substrate; no known M9 blockers. Deferred to
M9+: tray tooltip i18n (§78 — strings currently English; the project
has no i18n system yet), notification copy localization (§79).

## 28. M10/M11 deferred items

Code signing, release certificates, auto-update, release CDN/channel,
Store packaging — explicitly NOT in M8 (§20.1/§86).

## DoD (§92)

Core/Security/Reliability/Persistence/Quality checklists: all items
implemented; OS-visual rows recorded as the manual pass; build gates
(`cargo check/test --locked`, fmt, clippy, frontend typecheck/test/
build, tauri build) ALL PASS. M8 STOPS here (§95/§98 — no M9+ work).
