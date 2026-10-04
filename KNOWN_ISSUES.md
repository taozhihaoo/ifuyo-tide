# Known Issues — ifuyo Tide 1.0.0

Severity levels: P0 (release blocker) / P1 (strong release concern —
each item must carry an explicit "why it does not block 1.0") /
P2 (post-1.0) / P3 (minor).

**P0 open: 0.**
**1.0 disposition:** both P1 items ship with documented
non-blocking rationales below; P2/P3 are scheduled for 1.1+.

| ID | Severity | Area | Reproduction | Expected | Actual | Workaround | Release impact | Status |
|---|---|---|---|---|---|---|---|---|
| KI-1 | P1 | Dependencies | `cargo deny check advisories` flags RUSTSEC-2026-0293 (ringbuf 0.4.x double-free) | Advisory-free dependency tree | ringbuf is pinned to 0.4.x by librqbit-utp 0.7.0; no upstream fix exists yet | None needed for users | **Does not block 1.0:** ringbuf is used only for the scheduler decision ring; the vulnerable API path is unreachable from attacker-controlled input (docs/M10-supply-chain.md reachability analysis); no fix exists upstream to adopt | OPEN — revisit on librqbit-utp update |
| KI-2 | P1 | Security | DNS rebinding: hostname resolves to a public IP during validation, then to a private IP at connect time | Private addresses are unreachable for download sources | Validate→connect window is NOT GUARANTEED against rebinding (docs/M10-security-audit.md §43 URL/SSRF = PARTIALLY VERIFIED) | None needed for users | **Does not block 1.0:** exploiting it requires the user to add an attacker-controlled DNS name as a download source; IP literals and redirects are already tested; private-address policy default-denies | OPEN — pinned-resolution connector slated for 1.1 |
| KI-3 | P2 | Installer | Inspect the 1.0 setup exe signature | Signed installer | Unsigned — no code-signing certificate exists; honestly recorded per M12 §58 as Code Signing: NOT AVAILABLE | Users see the standard SmartScreen prompt for unsigned installers | Post-1.0 gate | OPEN — 1.1 (pending certificate decision) |
| KI-4 | P2 | Updater | Check for updates in the app | — | No built-in updater in 1.0 (docs/M11-release.md §4) | Manual upgrade: run the new installer over the old one (data preserved — re-verified for 1.0.0) | Documented limitation | OPEN — 1.1 |
| KI-5 | P3 | Validation | Run install/upgrade/uninstall on machines other than the development host | Evidence from ≥2 distinct machines | The M11/M12 install/upgrade/uninstall passes ran on the development machine only, against a pristine per-user state with an isolated app-data root | — | Low — all flows are automated and re-runnable | OPEN — first external test opportunity closes this |
| KI-6 | P3 | Validation | High-DPI / multi-monitor visual walkthrough; packet-capture verification of the no-cloud claim | No layout breakage; no unexpected egress | Not instrumented in M11 (manual items carried from M10 §51) | — | Low | OPEN — manual checklist in docs/M11-test-matrix.md |
| KI-7 | P3 | Code quality | `cargo clippy --all-targets` on the workspace reports ~26 legacy warnings (pre-M6 modules, test targets) | Zero warnings | Legacy warnings inventoried in prior milestone reports; none introduced by M8–M12 | — | Cosmetic | OPEN — 1.1 backlog |
| KI-9 | P3 | UX | Corrupt app data (or any fatal startup failure) in a release build: process exits(1) fail-closed, but stderr is invisible — the user sees "app does not open" with no dialog | Structured startup-failure dialog | stderr-only reporting (`fatal_startup_raw`) | Reinstall/clean app data per docs/M11-install-upgrade.md §5 | Low — data-safe failure, rare trigger | OPEN — 1.1 backlog |

Fixed during M11 (kept here for the release trail, not open):

| ID | Severity | Area | Finding | Fix |
|---|---|---|---|---|
| KI-8 | P1 (found→fixed) | Release build | The M10 spike fail-closed stub macro compiled `spike_autorun_enabled => bool` stubs as `Err(...)` — release profile failed with E0308 (application could not build → RC blocker per M11) | Stub macro split by return kind; `spike_autorun_enabled` fail-closed to `false` (commit c6a238c); release build re-verified |
| KI-10 | P1 (found→fixed) | UI freeze | The M0.5 spike prototype view ("Spike (M0.5)" sidebar entry, autorun jump, view render) shipped in release navigation — prototype UI as a 1.0 feature | Gated to `import.meta.env.DEV` in Sidebar/App (commit a3365d7); release navigation is Downloads/Torrents/Library/Player/Settings only |
