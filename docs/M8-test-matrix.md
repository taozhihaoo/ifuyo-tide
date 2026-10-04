# M8 Test Matrix

Automated: 313 Rust tests / 0 failures (incl. 12 new platform tests:
fuzz-style LaunchRequest classification §40 — empty/whitespace/
oversized/malformed-percent/unsupported-scheme/torrent-size/dir-as-
file/reserved-name — plus drain-idempotency and vanished-task
scheduling) + 25 frontend tests / 0 failures. `tsc` clean; fmt clean;
clippy zero warnings in platform code; NSIS installer
(`ifuyo-tide_0.1.0_x64-setup.exe`) BUILDS — §83 smoke carrier.

## §34 single instance

| Case | Status |
|---|---|
| T1 second launch | plugin handles: process exits, first instance activated (plugin contract; manual pass) |
| T2 running + tide://… | forwarded args → LaunchRequest → routed (automated at parse level; activation manual) |
| T3 closed + tide://… | deep-link plugin launches with args → parsed at startup |
| T4 tray + second launch | window re-activated (activate_main_window on every forward) |

## §35 torrent association | §36 window state | §37 tray | §38 autostart | §39 notifications

Association parse coverage automated (valid/invalid/huge/missing/
unsupported/duplicates via `torrent_path_classification` and friends);
window-state restore, tray clicks, autostart registry truth and
notification delivery are PLUGIN + OS behaviors — verified through
their official plugins and recorded as the manual installed-build
matrix (§33: Development vs Installed environments).

## §40 fuzz/negative (automated, 12 platform tests)

empty · whitespace · oversized (64 KiB cap) · malformed percent
· unsupported scheme (javascript/file/ftp) · nested URI · unknown
deep-link param · multiple params · duplicated query · control
characters via decode rejection · reserved device names · dir-as-file
· missing file · oversized .torrent · batch with one invalid arg
→ ALL structured rejections; no panic, no spawn, no arbitrary access.

## §88 manual scenarios A–G (installed build)

A tray-keeps-downloading · B double-click torrent activates · C
second-instance tide:// handoff · D close→tray→restore · E autostart
login · F second-monitor unplug recovery · G notification click
activates. All are OS/visual interactions — recorded as the standing
manual pass, consistent with M4–M7 reports.

## Performance (§90)

No new polling or timers were added: tray updates are event-driven
string swaps; the scheduler tick (previously benchmarked 807 µs @1000
tasks) is the only periodic work; single-instance handoff is one IPC
per launch. Idle Tray/Window CPU/memory measured in the M6/M7
benchmarks — unchanged by M8 (no new loops).
