# M11 Report — ifuyo Tide 0.2.0-rc.1 (Release Candidate)

Evidence types: FACT (code/build/observed state), TEST RESULT
(scripted run at the RC commit), TOOL RESULT (script/binary output),
STATIC ANALYSIS (audit of code/config), MANUAL REVIEW (human
inspection), ASSUMPTION (not directly evidenced). Evidence-type
mixing is avoided; open items are labeled.

## 1. Executive Summary

M11 froze the M0–M10 feature set and produced a real, installable,
traceable Release Candidate: version unified at 0.2.0-rc.1, release
build with locked dependencies (317 Rust + 38 frontend tests green),
NSIS installer built, **executed** install/upgrade/uninstall/reinstall
matrix (58 checks PASS / 0 FAIL) with data-preservation and
fail-closed-corrupt-DB evidence, automated release smoke, packaged
`release/` artifacts with SHA-256/manifest/SBOM, and the M11
document set. P0 = 0. One P1 release blocker was **found and fixed**
during M11 (release profile did not compile — KI-8). Verdict:
**RC CONDITIONAL** — all core flows PASS; non-blocking pendings are
code signing (KI-3), built-in updater (KI-4), and third-party
machine validation (KI-5).

## 2. Release Candidate Identity

- version **0.2.0-rc.1** (package.json = tauri.conf.json =
  Cargo.toml = exe resource — TEST RESULT)
- commit: recorded in `release/RELEASE-MANIFEST.json`; tag
  `v0.2.0-rc.1`
- branch `main`; build ID = commit + toolchain block in the manifest
- platform windows / arch x64 / profile release
- toolchain (TOOL RESULT): rustc/cargo 1.99.0, node v24.19.0,
  pnpm 12.8.1, tauri-cli 2.12.1, target x86_64-pc-windows-msvc

## 3. M0–M10 Completion Audit

Audit at M11 start (FACTS/HYPOTHESES block in the M11 session
record): all ten milestones' deliverables present in code and docs;
317-test suite green; versions aligned; capabilities minimal;
migrations transactional; spike surface cfg-gated. Drift found
during M11: the M10 spike fail-closed stub did not compile under the
release profile (KI-8) — M10's "release build verified" claim was
stale relative to the final tree. Fixed in c6a238c and re-verified.

## 4. M11 Changes

No features. Additions: RC version alignment (7d8afb1),
`scripts/release-smoke.ps1` (921210c + fixes),
`scripts/package-release.ps1` (15114d6 + fixes),
`scripts/install-matrix.ps1`, RELEASE-NOTES.md, KNOWN_ISSUES.md, six
`docs/M11-*.md`, root-doc release states. One fix: c6a238c (KI-8).
No migration touched; no test deleted or weakened; no warning
suppressed.

## 5. Build System

`cargo fmt --check` clean; `cargo test --locked` 317/0; `cargo build
--release --locked`; `tsc --noEmit` clean; `pnpm test` 38/0;
`pnpm tauri build` → exe + NSIS bundle. Locks committed
(`Cargo.lock`, `pnpm-lock.yaml`). Reproducibility: version, commit,
locks, commands pinned and recorded (§8 traceability level; bit-for-
bit identity not pursued — ASSUMPTION documented in M11-release.md).

## 6. Installer

NSIS per-user, `ifuyo-tide_0.2.0-rc.1_x64-setup.exe`, 6 738 922
bytes, SHA-256 `0915478d7c9cc46f89b81b3c555fdfd2154cc8c1d8847e7975ffa2e329992c63`,
WebView2 bootstrapped if missing. MSI not shipped (decision in
M11-release.md §3). Install/upgrade/uninstall behaviors: see §8–§10
and `M11-install-upgrade.md`.

## 7. Signing

**Pending** (§46): no certificate; unsigned RC; signing is a release
gate before 1.0; no private key exists in the repository (hygiene
gate — TOOL RESULT). No pseudo-production signing fabricated.

## 8. Upgrade

0.1.0 → 0.2.0-rc.1 in-place upgrade executed (TEST RESULT): exe and
registry version updated; seeded task row + `user_version=4`
survived; silent upgrade force-closes a running app (defined
behavior, recorded). `.part` upgrade semantics: engine-level TEST
RESULT (`M11-recovery-validation.md` §2); UI-level upgrade with a
live download = manual item.

## 9. Uninstall

Install dir, HKCU registry entry, start-menu shortcut removed; app
data (database/logs/torrent-metadata) **preserved** unless the
explicit app-data option is ticked; files outside app data never
touched (TEST RESULT).

## 10. Data Preservation

Full lifecycle table in `M11-install-upgrade.md` §5. Verified across
first launch → upgrade → uninstall → reinstall (TEST RESULT).

## 11. Migration

4 transactional `user_version` migrations, idempotent runner; no
migration history rewritten (§23 FACT). Cross-version migration
drill not applicable (no schema divergence exists between the
previous dev build and RC — FACT, honest scope note in
`M11-recovery-validation.md` §1).

## 12. Download Regression

M1 suites green: resume/range/retry, redirect re-validation,
checksum paths (TEST RESULT). UI-interactive flows: manual
checklist.

## 13. Torrent Regression

M3 suites green: metadata, file selection, pause/resume, restart
recovery, multi-file path safety (TEST RESULT). Live legal-seed
swarm: manual item.

## 14. Streaming Regression

M4 suites green: play-while-downloading, seek into available and
unavailable regions, gateway serving file/HTTP/torrent uniformly
(TEST RESULT).

## 15. Player Regression

Backend session/range suites green; WebView2 interactive playback
(seek bar, buffer states) is a manual checklist item — PARTIAL, not
claimed otherwise.

## 16. Library Regression

M6 suites green (scan/import/query/refresh/delete/reveal-path)
(TEST RESULT). Visual review: manual item.

## 17. Security Regression

Security corpus + gateway matrix green; capabilities minimal and
feature-mapped; trust boundary intact; logging/error policies hold;
DNS-rebinding TOCTOU still NOT GUARANTEED (KI-2). Details:
`M11-security-regression.md`.

## 18. Privacy

No telemetry/cloud/accounts/update-check/crash-reporting (STATIC
ANYSIS of config + deps); only task-driven outbound connections +
loopback gateway (FACT). PRIVACY.md states this without absolute
"collects nothing" phrasing. Packet-capture proof: manual item
(KI-6).

## 19. Third-party Licenses

deny.toml license allow-list enforces MIT/Apache-2.0/BSD/ISC/
Unicode/Zlib/0BSD/Unlicense/CC0/BSL/MPL-2.0/OpenSSL/CDLA (TOOL
RESULT: `cargo deny check licenses` pass at M10; inventory shipped
as `release/DEPENDENCIES.txt`, 518 entries). No FFmpeg/mpv/VLC/
GStreamer runtime is bundled (FACT — player is WebView2). THIRD_
PARTY_NOTICES.md consistent with the shipped set.

## 20. Clean-machine Validation

Dev-host simulation: first-launch zero state in isolated
`LOCALAPPDATA` (TEST RESULT); full installer matrix non-elevated
(TEST RESULT). Third-party machine: NOT done (KI-5, P3). No dev-
tool dependency exists (WebView2 is the only runtime prerequisite —
FACT).

## 21. Performance

Measured subset (TEST RESULT): cold start 1 088–1 755 ms; idle CPU
~0%/8 s; 1 000-task recovery bound PASS (engine level). Not
instrumented: throughput, memory, task-list load time, DPI-scale
runs — recorded in `M11-test-matrix.md` §5. RC-level goal "no
obvious anomaly" is met by the measured evidence.

## 22. Test Matrix

`M11-test-matrix.md`: automated layers, §48 results, core-chain
statuses (13 PASS, 2 PARTIAL: Player/Windows-UI interactions),
manual checklist (10 items).

## 23. FACT / HYPOTHESIS / INFERENCE

- FACT: all build/test/packaging/matrix results above; version
  alignment; capability set; data-path separation.
- TEST RESULT: 317+38 tests, 12-check smoke ×3, 58-check matrix,
  fail-closed corrupt-DB behavior.
- HYPOTHESIS: toolchain cache state does not alter artifact
  behavior (not evidenced either way — documented).
- INFERENCE: an interrupted download under RC resumes under tested
  M2 rules because the upgrade preserves app data verbatim and
  byte-level semantics are suite-covered.

## 24. RESOLVED / CONFIRMED / OPEN / WORSENED

- RESOLVED: KI-8 release-build compile blocker (fixed c6a238c);
  version-source alignment; missing installer/install evidence.
- CONFIRMED: M10 P1s stand as documented (ringbuf advisory KI-1,
  DNS-rebinding window KI-2); no new attack surface from M11
  (packaging scripts are dev-side only).
- OPEN: KI-1..KI-9 (two P1, two P2, five P3) — see KNOWN_ISSUES.md.
- WORSENED: none (all suites green throughout; no regression
  observed).

## 25. P0 / P1 / P2 / P3

P0 = **0**. P1 = 2 (KI-1 ringbuf advisory without upstream fix,
documented reachability analysis; KI-2 DNS-rebinding TOCTOU). Both
are documented-and-scheduled, neither is a data-loss/corruption/
security-bypass of shipped functionality. P2 = 2 (KI-3 signing,
KI-4 updater). P3 = 5 (KI-5/6/7/9 + manual-item tracking).

## 26. Known Issues

KNOWN_ISSUES.md (9 tracked + fixed-trail KI-8). Release-blocking
P1 = 0.

## 27. Release Artifacts

`release/`: setup exe, `SHA256SUMS.txt`, `RELEASE-MANIFEST.json`
(commit, branch, dirty flag, toolchain, locks, signing status,
per-artifact size/SHA-256), `DEPENDENCIES.txt` (SBOM-lite, 518
entries), `RELEASE-NOTES.md`. Traceability per §80: any artifact
answers commit/toolchain/locks/signing/SHA-256.

## 28. Checksums

`release/SHA256SUMS.txt`: installer
`0915478d7c9cc46f89b81b3c555fdfd2154cc8c1d8847e7975ffa2e329992c63`.
(hash generated at packaging; verification = re-run
`Get-FileHash`, TOOL RESULT.)

## 29. Limitations

Single-machine validation (KI-5); unsigned (KI-3); no built-in
updater (KI-4); startup-failure dialog gap (KI-9); throughput/
memory not instrumented; UI-interactive workflows manual. All
documented, none concealed.

## 30. M12 Readiness

| Area | Status | Evidence |
|---|---|---|
| Product | READY | M0–M10 feature set frozen, RC flows pass (matrix) |
| Engineering | READY | 317+38 green, locks pinned, hygiene gates |
| Security | CONDITIONAL | corpus green; KI-2 TOCTOU open; KI-9 UX gap |
| Installer | READY | §48 matrix 58/58 PASS |
| Upgrade | READY | executed upgrade with preservation evidence |
| Reliability | CONDITIONAL | recovery suites green; live-offline/fault-injection items manual |
| Documentation | READY | doc set + release notes + known issues complete |
| Legal/License | CONDITIONAL | allow-list pass + SBOM; signing (KI-3) outstanding |
| Performance | CONDITIONAL | measured subset healthy; full baseline pending |

**M12 READY overall: CONDITIONAL** — 1.0 gating items: signing,
updater, third-party machine validation, KI-1/KI-2 disposition.

## 31. Definition of Done

§89 checklist: audited ✓; versions consistent ✓; release build ✓;
locks ✓; automated tests ✓; artifacts ✓; installer ✓; clean-machine
(dev-host zero-state; third-party open) ✓/partial; launch ✓; HTTP ✓;
torrent ✓; persistence ✓; recovery ✓; streaming ✓; player
(backend) ✓; library ✓; restart ✓; upgrade ✓; uninstall ✓;
download-data lifecycle ✓; migration ✓; crash consistency ✓;
security/gateway/path/URL regressions ✓; capabilities ✓; logs ✓;
privacy ✓; licenses ✓; SHA-256 ✓; signing **PENDING (documented)**;
manifest ✓; release notes ✓; known issues ✓; **P0 = 0**; release-
blocking P1 = 0; clean-Windows validation **partial (KI-5)**; M12
readiness documented ✓.

Final gate (§91): **B — RC CONDITIONAL.** M11 = COMPLETE /
CONDITIONAL. Per §92, M11 stops here; no M12 feature work.
