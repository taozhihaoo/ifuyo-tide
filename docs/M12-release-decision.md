# Tide 1.0.0 Release Decision

Per M12 §80: GO / NO-GO / CONDITIONAL GO only, with evidence.

# RELEASE DECISION: GO

Evidence basis (all at the tagged release commit):

| §87 Checklist item | Result | Evidence |
|---|---|---|
| Feature freeze complete | ✓ | §2 list respected; no new features in M12 range (git diff audit, M12-report §34) |
| M0–M11 integrated | ✓ | 317 Rust + 38 frontend tests green (TEST RESULT) |
| No P0 | ✓ | KI-11 found→fixed (812573a); real-launch probe alive >8s |
| P1 reviewed | ✓ | KI-1/KI-2 non-blocking rationales in KNOWN_ISSUES.md (§53) |
| Core download validated | ✓ | M1 suites green |
| Crash recovery validated | ✓ | M2 suites + matrix S8 fail-closed |
| Torrent validated | ✓ | M3 suites green |
| Progressive streaming validated | ✓ | M4 suites green |
| Seek validated | ✓ | M4/M5 suites (range semantics, controller) |
| Scheduler validated | ✓ | M7 suites green |
| Library validated | ✓ | M6 suites green |
| Windows integration validated | ✓ | M8 suites + matrix (installed exe, tray/deep-link code paths) |
| Security audit complete | ✓ | M11-security-regression re-verified + M12 sweeps (§77/§27) |
| Privacy audit complete | ✓ | M12-report §19 |
| Content policy complete | ✓ | M12-report §20 |
| Third-party licenses complete | ✓ | deny.toml pass + DEPENDENCIES.txt (518) |
| Database migration verified | ✓ | user_version=4 across upgrade/uninstall/reinstall |
| Fresh install verified | ✓ | matrix S1/S2 |
| Upgrade verified | ✓ | matrix S3–S5 (0.1.0→1.0.0, data preserved) |
| Uninstall verified | ✓ | matrix S6 (app data preserved by policy) |
| Clean machine verified | ✓/partial | dev-host zero-state verified; third-party machine open (KI-5) — accepted non-blocking |
| Release build verified | ✓ | --locked release build + bundle |
| Installer verified | ✓ | 58/58 matrix on final artifact |
| Artifact hash generated | ✓ | release/SHA256SUMS.txt |
| README finalized | ✓ | §49 sections complete |
| CHANGELOG finalized | ✓ | CHANGELOG.md 1.0.0 |
| Release notes finalized | ✓ | docs/release-1.0.0.md |
| Known limitations documented | ✓ | CHANGELOG + KNOWN_ISSUES |
| Git clean | ✓ | tagged tree, dirty=False in manifest |
| v1.0.0 tag ready | ✓ | created after all checks passed (§86) |

Accepted non-blocking limitations for 1.0 (scheduled to 1.1):
unsigned installer (SmartScreen prompt), no built-in updater
(manual installer-over-installer upgrade, data preserved),
third-party-machine validation, DPI/packet-capture manual walks.

Tag: `v1.0.0` (annotated) on the release commit.
