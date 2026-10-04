# Release Notes — ifuyo Tide 0.2.0-rc.1

Status: **Release Candidate** (channel: rc, profile: release).
This is the first RC toward 1.0. It freezes the M0–M10 feature set:
no new features were added in M11 — M11 validated, packaged and
documented what exists.

## What Tide is

Tide is a local-first, Windows-native download manager with
progressive media playback: you provide the source (HTTP URL,
.torrent or magnet link), Tide downloads it, and you can start
watching while the download is still in progress. Tide does **not**
provide resource discovery, does not recommend sources, does not
bypass DRM or paywalls, and does not provide anonymity.

## Highlights

- HTTP downloads with pause/resume, crash-safe `.part` recovery and
  content validation (M1/M2).
- BitTorrent downloads via librqbit: metadata, file selection,
  pause/resume, restart recovery (M3).
- Progressive streaming: play while downloading, with seek into
  available and not-yet-available regions (M4/M5).
- Local library: scan roots, import, query, reveal, playback history
  (M6).
- Deterministic scheduler with bandwidth limits and per-task
  priorities; streaming demand is prioritized (M7).
- Windows integration: single instance, `tide://` deep links,
  .torrent file association, tray (show / pause-all / resume-all /
  quit), autostart opt-in, download notifications (M8).
- Product polish: first-run welcome flow, centralized error UX,
  honest loading states (M9).
- Security hardening: capability audit, SSRF protections
  (scheme/IP-literal/redirect re-validation), loopback-only stream
  gateway with session tokens, fail-closed debug surface, security
  regression corpus (M10).

## Security

- No telemetry, no accounts, no cloud services, no update checks.
  The app makes network connections only to: download sources you
  provide, torrent peers/trackers/DHT for your tasks, and a
  loopback-only (127.0.0.1, random port, session token) gateway the
  player uses. See `docs/M10-network-surface.md`.
- Frontend is never a security authority; every IPC input is
  re-validated in Rust. Security regressions are covered by a
  dedicated test corpus (`tests/security_corpus.rs`).
- The debug/experiment IPC surface is compiled out (fail-closed
  stubs) in release builds.

## Compatibility

- Windows 10/11 x64. Per-user NSIS installer (no admin rights
  required).
- WebView2 Runtime is required; the installer bootstraps it if
  missing (Tauri default download bootstrapper).
- No developer tools (Node, Rust, Python, FFmpeg) are required on
  the target machine.

## Upgrade notes (0.1.0 dev builds → 0.2.0-rc.1)

There is no built-in updater in the RC (see Known Issues). To
upgrade: run the new installer over the existing installation. The
in-place upgrade preserves the database, settings, library and all
downloaded files under `%LOCALAPPDATA%\ifuyo\Tide\` and your chosen
download directory. Close the running app before upgrading.

## Data preservation

| Data | Location | Uninstall |
|---|---|---|
| Database (tasks, library, settings) | `%LOCALAPPDATA%\ifuyo\Tide\database\` | Preserved by default |
| Logs | `%LOCALAPPDATA%\ifuyo\Tide\logs\` | Preserved by default |
| Torrent metadata | `%LOCALAPPDATA%\ifuyo\Tide\torrent-metadata\` | Preserved by default |
| Downloaded files | Your chosen download directory | Never touched |

The uninstaller offers an explicit app-data removal option; leaving
it unticked preserves everything listed above.

## Known issues

See `KNOWN_ISSUES.md` at the repository root. Summary: P0 = 0;
P1 = 2 (documented, non-blocking: ringbuf advisory without upstream
fix; DNS-rebinding TOCTOU window). Notable limitations: no built-in
updater, unsigned build (no certificate yet).

## Testing coverage

Verified (evidence in `docs/M11-report.md` and the M11 document
set): 317 Rust tests + 38 frontend tests, release build with locked
dependencies, automated release smoke (isolated launch, migrations,
task-row persistence across restart, idle resource budget), real
install / upgrade / uninstall / reinstall passes with data
preservation checks, security regression corpus, SQLite migration
idempotency.

Not verified in this RC: third-party clean machines outside the
development host (the install/upgrade/uninstall pass ran on the
development machine against a pristine per-user state), multi-monitor
DPI walkthrough, packet-capture verification of the no-cloud claim.
These are recorded as manual items in `docs/M11-test-matrix.md`.
