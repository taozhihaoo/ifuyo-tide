# M11 Install / Upgrade Validation

Evidence types: TEST RESULT (scripted, repeatable — the matrix script
re-runs the whole matrix), FACT (code/config), MANUAL REVIEW,
ASSUMPTION. The executed matrix is `scripts/install-matrix.ps1`;
final evidence run: **58 PASS / 0 FAIL** on the RC commit, non-admin
session (`elevated=False`).

## 1. Installer identity (§6)

| Field | Value |
|---|---|
| File | `ifuyo-tide_0.2.0-rc.1_x64-setup.exe` |
| Kind | NSIS, per-user (no elevation) |
| Size | 6,738,922 bytes |
| SHA-256 | `0915478d7c9cc46f89b81b3c555fdfd2154cc8c1d8847e7975ffa2e329992c63` |
| Signature | unsigned — Production Signing = Pending (KI-3) |
| MSI | not shipped (rationale: docs/M11-release.md §3) |

## 2. Path separation (§11) — TEST RESULT

- Install dir: `%LOCALAPPDATA%\ifuyo-tide\` (registry `InstallLocation`
  — NSIS writes it quoted; scripts normalize).
- App data: `%LOCALAPPDATA%\ifuyo\Tide\{database,logs,torrent-metadata}`.
- Download root: user-chosen directory (never under install/app-data
  by default).
- Matrix check `S1-install-path-not-appdata` PASS: install dir ≠ app
  data dir; download root is user policy, not installer policy.

## 3. First install on a machine without dev tools (§14)

The installed app was smoke-launched inside an isolated
`LOCALAPPDATA` (§81): directories created on first run, SQLite
migrated to `user_version = 4`, `app.started` logged, no panic lines,
no dev-environment dependency (no Node/Rust/Python/FFmpeg; the only
runtime prerequisite is WebView2, bootstrapped by the installer).
TEST RESULT — `release-smoke.ps1` against the installed exe.

## 4. Executed matrix (§48/§77) — TEST RESULT, `install-matrix.ps1`

| Scenario | Expected (§48) | Actual | Result |
|---|---|---|---|
| Clean Install (0.1.0 = previous release, "Machine B" start state) | Pass | Registry `DisplayName=ifuyo-tide DisplayVersion=0.1.0`, exe + start-menu shortcut created | PASS |
| First launch of installed build | Initialize + reach UI | `app.started`, dirs + DB created, no panic | PASS |
| Upgrade With Active Task (app running during silent upgrade) | Defined | Silent NSIS installer terminates the running app, then completes; 0 processes remain afterwards | PASS (defined: force-close) |
| Upgrade Older Version (0.1.0 → 0.2.0-rc.1) | Pass | exe `ProductVersion` and registry `DisplayVersion` both 0.2.0-rc.1 | PASS |
| Upgrade With `.part` | Pass | No active download existed in the test window; `.part` semantics are enforced by M2 validator/`file_sink` suites (see M11-recovery-validation.md §3), not by the installer | PASS / inference from engine tests |
| Upgrade With Library (DB/settings/library) | Preserve | Seeded task row + `user_version=4` survived the upgrade (same isolated data root across the upgrade boundary) | PASS |
| Uninstall | Defined | Install dir, registry entry, start-menu shortcut removed; **app data preserved** | PASS |
| Download Preservation | Pass | Files outside app data untouched by uninstaller | PASS |
| Reinstall (same version) | Defined | Reinstall succeeds; data still present and app healthy | PASS |
| DB Migration | Pass | `user_version=4` before and after upgrade; no migration events re-run (idempotent) | PASS |
| Corrupt App Data | Recoverable/Error | Corrupted SQLite header → process terminates with structured stderr (`SQLITE_ERR: NotADatabase` → `fatal: failed to open database`); no silent corruption, no data destruction | PASS (fail closed) |
| Interrupted Install | Defined | **NOT TESTED** — half-running an NSIS install is flaky to simulate reliably; recorded as a manual item | NOT TESTED |

## 5. Data lifecycle (§13)

| Type | Example | Install | Upgrade | Uninstall | Reinstall |
|---|---|---|---|---|---|
| Application binary | `ifuyo-tide.exe` | created | replaced | deleted | recreated |
| SQLite DB | `tide.db` | created | preserved | preserved (explicit checkbox to remove) | reused |
| Logs | `tide.log` | created | preserved (appends) | preserved | appended |
| Torrent metadata | `torrent-metadata\` | created | preserved | preserved | reused |
| User downloads | user-chosen root | not installer-controlled | preserved | never touched | preserved |
| `.part` | partial downloads | n/a | preserved (M2 recovery validates) | preserved | resumed |

(FACT: install dir contents; TEST RESULT: preservation behavior.)

## 6. Privileges (§16) — TEST RESULT

The entire matrix ran in a non-elevated session. No step required
administrator rights: per-user install, per-user registry (HKCU),
app data under `%LOCALAPPDATA%`, downloads under user-writable roots.
No "Run as Administrator" fallback exists or is needed.

## 7. Machine scope (§77) — honest statement

Machine A (clean install) and Machine B (upgrade) were simulated on
the development host against a pristine per-user state and an
isolated app-data root. A third-party physical/virtual clean machine
was NOT available; this is recorded as KI-5 (P3). The matrix is a
single script and re-runnable on any machine.

## 8. Final user workflow (§78)

Automated coverage: install → launch → (seeded task row standing in
for created task) → restart/recovery → upgrade → data preserved →
uninstall → policy confirmed → reinstall. UI-interactive steps
(create task via form, playback, seek, tray interactions) remain on
the manual smoke checklist (M11-test-matrix.md §4) — not claimed as
automated.
