# Tide 1.0.0 Release Notes

Status: **Stable 1.0** (channel: stable, profile: release).
Audience: Windows users without a development environment.

## Release Overview

Tide 1.0 is a local-first Windows application for downloading and
progressively watching media. You bring the source — an HTTP/HTTPS
URL, a .torrent file, or a magnet link — and Tide downloads it,
lets you start watching before the download finishes, keeps tasks
safe across crashes and restarts, and archives finished media into
a local library.

Tide is a **local tool, not a content-discovery platform**: no
search, no resource indexes, no recommendations, no DRM/paywall
bypass, no anonymity claims.

## Major Features

- HTTP/HTTPS downloads with pause/resume, retries, ETag/Last-Modified
  revalidation and SHA-256 verification.
- BitTorrent (Download-Only by default: uploads disabled) with
  per-file selection and restart recovery.
- Progressive playback: play while downloading, seek into
  unavailable regions (demand-driven scheduling fetches what the
  player needs first).
- Local library with automatic import of completed downloads.
- Deterministic scheduler with bandwidth limits and per-task
  priorities.
- Windows integration: tray, single instance, deep links, .torrent
  association, notifications, optional autostart.

## Supported Sources

| Source | Supported | Notes |
|---|---|---|
| HTTP/HTTPS URL | Yes | scheme allow-list; private addresses require explicit opt-in |
| Magnet link | Yes | structured parsing; Download-Only |
| .torrent file | Yes | file association + `tide://add` |
| Local file | Yes | via Library / player |

## Playback

Player is WebView2-based. Playback and seek work for what the
host's WebView2/Edge codec pack supports:

| Container | Codec | Status |
|---|---|---|
| MP4 | H.264 / AAC | Supported |
| WebM | VP9 / Opus | Supported |
| WebM | AV1 / Opus | Conditionally supported (environment-dependent) |
| MKV | HEVC etc. | Conditionally supported (environment-dependent) |

MKV/HEVC/AV1 support varies by Windows machine — Tide reports a
structured decode error instead of pretending.

## Persistence

- SQLite database, transactional migrations; the `.part` file is
  the source of truth for resume points.
- Crash, force-kill and restart recovery are covered by dedicated
  integration tests and an executed installer matrix.
- App data lives in `%LOCALAPPDATA%\ifuyo\Tide\`; downloads live in
  the directory you choose. The uninstaller removes the app, not
  your data (explicit checkbox to also remove app data).

## Scheduler

Playback demand is prioritized (emergency → playback → seek), then
weighted-fair background downloads with anti-starvation floors.
Global and per-task bandwidth limits are enforced with real token
buckets.

## Windows Integration

Single instance, `tide://` deep links, .torrent association, tray
controls, close-to-tray (default on, configurable), autostart
(opt-in), download/torrent notifications.

## Security / Privacy

- No telemetry, accounts, cloud services, or update checks.
- Loopback-only playback gateway with session tokens; no
  path-based file access.
- SSRF defenses on download sources; hostile-input regression
  corpus in CI-test suite.
- Minimal Tauri capabilities; debug surface compiled out of
  releases.
- Details: SECURITY.md, PRIVACY.md, docs/M11-security-regression.md.

## Known Limitations

See CHANGELOG.md "Known Limitations" and KNOWN_ISSUES.md: unsigned
installer (SmartScreen prompt), no built-in updater, WebView2 codec
dependence, per-session torrent rate limiting, DNS-rebinding
validate→connect window, silent (stderr-only) fatal startup
reporting.

## Upgrade Notes

From any previous dev/RC build: close Tide, run this installer over
the old installation. Database, settings, library and downloaded
files are preserved. There is no in-app updater in 1.0.

## Verification Summary (evidence)

- 317 Rust + 38 frontend automated tests green at the release
  commit.
- Installer matrix (fresh install / upgrade / uninstall / reinstall
  / corrupt-DB fail-closed / data preservation): 58 checks PASS on
  real NSIS installers, non-admin (docs/M11-install-upgrade.md,
  re-run for 1.0.0 — docs/M12-report.md §22).
- Release smoke on the installed artifact: isolated launch,
  migrations, task-row persistence across kill+restart, idle
  resource budget (docs/M12-report.md §24/§25).
