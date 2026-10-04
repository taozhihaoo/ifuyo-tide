# M11 Test Matrix

Scope: §40–§42 (visual/DPI/window), §48–§52 (matrix, smoke,
categories, core-chain regression), §53–§54 (performance/idle).
Every row carries an evidence type; "应该没问题" is not a result.

## 1. Automated layers (§50/§51)

| Layer | Coverage | Evidence |
|---|---|---|
| Unit / integration (Rust) | 317 tests / 0 failed (`cargo test --locked`), incl. M1 download+resume, M2 persistence/recovery, M3 torrent, M4 streaming+gateway, M6 library, M7 scheduler, M9 soak, security corpus | TEST RESULT |
| Frontend (vitest) | 38 tests / 0 failed; `tsc --noEmit` clean | TEST RESULT |
| Release smoke (`release-smoke.ps1`) | version resource, isolated launch, required dirs, migrations (`user_version=4`), log health (no panic), seeded task-row persistence across kill+restart, idle CPU budget, warm relaunch | TEST RESULT (12 checks × 3 runs in matrix) |
| Installer matrix (`install-matrix.ps1`) | §48 scenarios S1–S8 | TEST RESULT — 58 PASS / 0 FAIL |
| Packaging (`package-release.ps1`) | SHA-256, manifest, SBOM, §9 hygiene gate | TOOL RESULT |
| Clippy | ~26 legacy warnings (pre-M6 modules/test targets; none from M8–M11) — tracked KI-7 | STATIC ANALYSIS |

## 2. Installer matrix results (§48)

See `M11-install-upgrade.md` §4 for the full executed table
(58 PASS / 0 FAIL at the RC commit). NOT TESTED row: Interrupted
Install (flaky to simulate — manual item).

## 3. Core-chain regression (§52)

| Chain | Status | Evidence |
|---|---|---|
| Task State Machine | PASS | M2/M7 suites green |
| Persistence | PASS | M2 suites + matrix S2/S5/S7 (row survives restart+upgrade) |
| Recovery | PASS | `M11-recovery-validation.md` §3 (crash/finalization/missing-part/1 000-task bound) |
| HTTP Download | PASS | M1 suites (resume, ranges, redirects) |
| HTTP Resume | PASS | M1 suites + validator/f_sink tests (§25 table) |
| Torrent | PASS | M3 suites (metadata, selection, pause/resume, recovery) |
| Availability | PASS | M3/M4 suites |
| Resource | PASS | M4 unified gateway suite |
| Scheduler | PASS | M7 suites (frozen semantics) |
| Progressive Streaming | PASS | M4 suites (play-while-downloading, range into available/unavailable) |
| Range Gateway | PASS | `range_head_and_full_semantics`, `gateway_security_matrix` |
| Player | PARTIAL | Backend session/range suites green; actual WebView2 playback interactions are manual items |
| Library | PASS | M6 suites (scan/import/query/reveal-path) |
| Windows Integration | PARTIAL | single-instance/deep-link/tray/autostart from M8 suites + matrix (installed exe launches, single-instance is code FACT); tray menu interaction = manual |
| Security | PASS | security corpus + gateway matrix + `M11-security-regression.md` |

## 4. Manual checklist (carried; not claimed as automated)

- [ ] Real-UI smoke walk (§49 steps 3–14): create HTTP task via form,
      pause/resume/complete, open file, torrent partial → stream →
      seek, tray pause-all/resume-all, tray Quit.
- [ ] Clean third-party machine install (KI-5).
- [ ] Multi-monitor / DPI 100–200% layout walkthrough (§40/§41).
- [ ] Window lifecycle: minimize/maximize/restore/reopen from tray
      (§42) — single-instance and close-to-tray are code-verified;
      visual confirmation is manual.
- [ ] Packet capture: no unexpected egress at idle (KI-6).
- [ ] Memory footprint reading (script WorkingSet unreliable — §54).
- [ ] Interrupted-install simulation.
- [ ] Offline live-toggle / DNS-failure / 429-5xx behavior (§29).
- [ ] Fault-injected disk-full / permission-denied (§30).
- [ ] Zombie port / locked-file audit across cancel/exit/crash (§55).

## 5. Performance baseline (§53) — measured subset (TEST RESULT)

| Metric | Value | Note |
|---|---|---|
| Cold start → `app.started` | 1 088–1 755 ms (installed, 3 runs) | 2 704 ms dev-tree run |
| Idle CPU (8 s sample) | ~0% | threshold < 5% |
| Task-list load / DB open / migration time | not instrumented | manual item |
| HTTP / torrent throughput | not instrumented in M11 | M7 bandwidth enforcement tested at engine level |
| 1 000-task recovery bound | PASS (engine level) | `thousand_task_recovery_is_bounded` |
| Memory (idle/downloading/streaming) | not measured reliably | manual item (script WS readout unreliable) |
| Scale runs (10/100/500/1 000 UI tasks) | not instrumented | engine-level 1 000-task bound covered |

## 6. Visual regression (§40) — MANUAL REVIEW scope

No layout automation exists. Shipped-string review (M9
centralization) found no placeholder/fake data: all displayed
numbers come from backend state (§39 — FACT: UI reads projections
via IPC only). DPI/long-name/zh-CN visual walk: manual checklist
§4.
