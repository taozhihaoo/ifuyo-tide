# M10 Windows Release Notes (§22–§25/§83/§84)

## Installer

- Type: NSIS (per-user by default — no admin required, §56).
- Registers: Start-menu shortcut, `.torrent` association (Editor →
  opens Tide's confirm flow), `tide://` protocol handler.
- WebView2: bootstrapper download strategy (Windows 11 ships WebView2
  by default; older systems fetch the Evergreen runtime).

## Upgrade

NSIS in-place upgrade preserves `%LOCALAPPDATA%\ifuyo\Tide\` (database,
settings, logs) and all user-chosen download directories. Migrations
run transactionally at first launch (M2 §119: no migration history is
rewritten; v1→v4 tested in `migration_regression_v3_to_v4`).

## Uninstall

Removes: install directory, shortcuts, .torrent/tide:// registrations,
autostart registration (installed by the plugin when enabled).

Preserves (by design, §48/§84): user downloads in the chosen download
directory, torrent data directories, library-imported files, and —
unless the uninstaller's app-data option is explicitly ticked — the
app-data directory itself.

## Verified

- Installer builds (NSIS single exe).
- Debug/release IPC surface split verified by release-binary symbol
  scan (no spike commands).
- Upgrade-data preservation covered by migration regression tests.

## Manual installed-build smoke (§83 checklist carrier)

Install → Launch → Settings → HTTP download → Pause → Restart →
Resume → Complete → Playback → Seek → Torrent → Pause → Restart →
Resume → Playback → Close → Reopen → Uninstall — each step recorded
with duration/logs/network/filesystem/security observations. This run
requires a clean machine and is the M10 manual acceptance carrier.
