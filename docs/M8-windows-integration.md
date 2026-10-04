# M8 Windows Integration

M8 turns Tide into a first-class Windows desktop citizen — WITHOUT
becoming a second business layer (§1/§3/§157). Everything here is a
native ENTRY POINT into the existing M0–M7 core.

## Architecture (§91)

```text
Windows OS
 ├── CLI / .torrent association      ⟍
 ├── tide:// deep link                ⟍
 ├── Single-instance handoff ──────→ LaunchRequest parse+validate
 ├── Tray (Show/Pause-all/Resume/Quit)  ⟍
 └── (notifications, autostart)          ⟍
                              WindowsIntegration (src/platform/)
                                       ↓  validated LaunchRequests only
                              Application Layer (commands/)
                              ┌────┼────────┐
                            Task Scheduler  Playback/Library
                                       ↓
                                 Persistence
```

Hard rule: NOTHING in `platform/` touches engines, scheduler internals
or the filesystem beyond the validated launch-path checks — Windows is
the host environment, not a business layer.

## Single Instance (§8)

Official `tauri-plugin-single-instance` 2.5.2, registered FIRST
(plugin doc requirement). The second process never reaches the
builder: its callback receives the forwarded argv, classifies every
argument through the LaunchRequest validator (§57 untrusted), routes
valid requests and activates the main window; the second process then
exits. Failure to classify = structured warning + window activation,
never a crash.

## LaunchRequest model (§2.2/§9)

`platform/launch.rs` — `parse_args(argv) -> Vec<LaunchRequest>`:

| Input | Classification | Validation |
|---|---|---|
| empty argv | Focus | — |
| http(s)://… | OpenHttpUrl | M1 `validate_download_url` (REUSED, §9.1) |
| magnet:… | OpenMagnet | librqbit `Magnet::parse` structural check (§9.2) |
| tide://add?magnet=… / ?url=… | recursive | strict percent-decode, ONE param, 64 KiB cap (§10.3/§58) |
| *.torrent path | OpenTorrentFile | canonicalize + regular file + 0 < size ≤ 16 MiB (§9.3) |
| media path | OpenMediaFile | canonicalize + extension allowlist (§9.4) |
| anything else | REJECTED | structured error (§40) |

LaunchRequest ≠ Task (§46): the frontend shows a confirm banner; only
"Add" routes to the existing create commands (§45 unified Add-Source
flow). The pending queue (§74) holds requests that arrive before the
frontend is ready; drain delivers each exactly once (§75: multiple
requests all kept).

## Tray (§13/§26/§62)

Show / Pause-all / Resume-all / Quit — actions go through application
commands (§62, no direct engine calls). Left click activates the
window. Tray creation failure = warning + main window keeps working
(§72 degraded mode). Tooltip from real counts via `update_tray_state`.

## Close semantics (§13.3/§22)

`windows.closeToTray` (default ON): window close hides to tray,
process keeps running (§14: window hidden ≠ app stopped); tray Quit
exits. The M2 durable-persistence guarantee is untouched — a hard kill
still recovers (§24).

## Autostart (§16)

Official plugin, default OFF (§16.1); Settings toggle drives
`autolaunch().enable/disable/is_enabled`. Registry truth is read back
and persisted for the UI. Autostart is a startup POLICY only — no
scheduler changes (§16.3).

## Notifications (§15)

Official plugin; completed/failed download notifications fired from
the frontend's existing progress stream, gated by the user's three
toggles and swallowed on any failure (§39: notification failure ≠
download failure). No marketing content (§15.2); body = friendly
title + file name only (§32).

## Window state (§17)

Official window-state plugin restores position/size/maximized; the
window constraint (min 860×560) plus the plugin's own validation keep
the window recoverable; multi-monitor/DPI scenarios are the standing
manual matrix (§77).

## Native open (§18/§41/§42/§43)

Library Open/Folder use the OFFICIAL opener plugin
(`open_path`/`reveal_item_in_dir`) — the previous raw `explorer`
spawn was REPLACED. Paths are backend-resolved from item ids
(§42/§43: no frontend paths).

## Capabilities (§19)

Added only: notification (notify + permission queries), autostart
(enable/disable/is-enabled), deep-link (get-current). NO
filesystem/shell/process grants.

## Installer (§20/§50/§83)

NSIS (single-installer decision, §50); `.torrent` association
registered at install (role Editor — opens Tide's confirm flow, §11);
`tide://` scheme via deep-link config. WebView2 strategy unchanged
(§51). The generated `ifuyo-tide_0.1.0_x64-setup.exe` is the smoke-test
carrier (§83); uninstall keeps user downloads (§48/§84 — NSIS
defaults do not touch %LOCALAPPDATA% app data unless asked, and never
the user-chosen download root).

## degraded mode (§72)

Tray failed → window works. Notification failed → downloads work.
Autostart failed → app works. Window state corrupted → plugin
resets to defaults. Every integration initializes in a warning-on-fail
path (see app/mod.rs setup).
