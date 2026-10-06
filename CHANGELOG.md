# Changelog

All notable changes to ifuyo Tide. This project keeps an honest,
capability-based changelog — no marketing language.

## 1.0.5 — 2026-10-06

Patch release.

### Fixed

- The "N files selected" counter stayed at its creation-time value
  after changing checkboxes — it now reflects the actual selection.

## 1.0.4 — 2026-10-06

Patch release from real-use validation of torrent actions.

### Fixed

- **Pause/Resume/File-selection could hang silently**: the IPC awaited
  librqbit session operations that can stall indefinitely on a torrent
  stuck mid-add — the click did nothing, with no error anywhere.
  Session mutators are now bounded (15 s) and return a structured
  error, which the UI displays instead of swallowing.
- **Phantom "Running" over a paused engine**: startup recovery re-adds
  torrents paused and marks rows ready/resumable, but the completion
  monitor "corrected" rows to running on its first tick with zero
  bytes flowing. The monitor now writes only on real byte progress;
  only the user's resume (or actual flow) can say Running.
- Startup recovery bounds each engine re-add (60 s per task) so one
  stale-partial torrent can no longer wedge recovery for the whole
  session (the row records `readd_timeout` and stays resumable).

### Improved

- Torrents view shows **Stalled** when a task has been running with
  zero bytes flowing for over a minute — distinguishing "downloading"
  from "the swarm has nothing for us" (honest-state rule, §39).

## 1.0.3 — 2026-10-06

Patch release: fixes the most user-visible launch defect.

### Fixed

- **Double-clicking the exe opened a console window with log output
  instead of (in front of) the app**: release builds were compiled as
  a console-subsystem program — `main.rs` was missing the
  `windows_subsystem = "windows"` attribute. Release builds are now
  GUI-subsystem (no console window; the app window opens directly).
  Debug builds keep the console. A PE-subsystem check was added to
  the release smoke harness so this cannot regress silently.

## 1.0.2 — 2026-10-05

Second patch release from real-use validation.

### Added

- Torrents view now shows **live download speed and ETA** while a
  task is running (real engine stats; "—" when idle, per the honest-
  data rules).
- **File selection**: torrents no longer grab every file by default.
  A .torrent task is added paused (pick files, then Resume); a magnet
  auto-pauses the moment its metadata resolves without a user
  selection; the file list has per-file checkboxes driving the
  existing selection IPC.

### Fixed

- Scheduler preferences (global bandwidth limit, per-task
  priorities) never loaded after a restart: the settings prefix
  query shipped an empty `ESCAPE` expression (a Rust string-escape
  mistake, visible as `SQLITE_ERR: ... ESCAPE expression must be a
  single character` at startup). Fixed and covered by a regression
  test, including wildcard-containing prefixes.

## 1.0.1 — 2026-10-05

Patch release from real-use validation of the installed 1.0.0.

### Fixed

- **Torrent pause did not persist**: the pause command wrote
  `phase='paused'`, which the database schema rejects (phase is an
  engine phase, pause is a task state) — every pause failed its
  write and a restart resumed the task. Pause now keeps the row's
  engine phase and flips only the state.
- **Magnet tasks froze at their creation-time shape**: when the DHT
  delivered the metadata, the real display name, total size and
  file list were never written back — the UI showed the raw info
  hash, 0 B and "Files (0)" forever. The completion monitor now
  backfills metadata (and file rows) when it resolves.
- The completion monitor no longer resurrects paused tasks back to
  running, writes the honest `metadata_pending` phase while metadata
  is unresolved, and no longer clobbers freshly backfilled values
  with a stale row snapshot.
- UI: the torrent badge shows "Resolving metadata…" instead of the
  internal enum; a Ready task shows only Resume (Pause requires a
  running task).

### Added

- CI: a GitHub Actions workflow running `cargo fmt --check`,
  `cargo test --locked` and the frontend typecheck/tests on
  windows-latest for every push/PR.

## 1.0.0 — 2026-10-05

First stable release. The M0–M11 feature set is frozen: M12 added no
features; it fixed release blockers, re-validated install/upgrade/
recovery/security behavior on real installers, and finalized the
documentation and artifacts.

### Added

- HTTP/HTTPS downloader: pause/resume, bounded retry, ETag /
  Last-Modified revalidation, SHA-256 checksum verification,
  >4 GiB files validated.
- Crash-safe persistence: transactional SQLite schema
  (`PRAGMA user_version` migrations), `.part`-based resume where the
  file is the source of truth, deterministic startup recovery.
- BitTorrent downloads (librqbit): magnet and .torrent sources,
  metadata fetch, per-file selection, pause/resume, restart
  recovery, corrupted-piece redownload; Download-Only default
  (uploads disabled).
- Progressive streaming: watch while downloading — local loopback
  Range gateway (random port, session token, host/range validation)
  serving HTTP, torrent and file resources uniformly; seek into
  not-yet-downloaded regions with demand-driven piece scheduling.
- Player: WebView2-based playback with honest buffering states,
  seek/volume/speed/fullscreen/keyboard controls, reload recovery
  (backend session is the source of truth).
- Local library: scan roots (junction-safe path handling), import on
  download completion, query/filter/sort, refresh, reveal in
  Explorer; benchmarked at 100k items.
- Smart scheduler: deterministic four-phase allocation, streaming
  demand prioritization (emergency/playback/seek), global and
  per-task bandwidth limits, per-task priorities, observable metrics.
- Windows integration: single instance, `tide://` deep links,
  .torrent file association, tray (show/pause-all/resume-all/quit),
  optional autostart, completion notifications, close-to-tray.
- Settings: download directory, bandwidth, scheduler preferences,
  Windows behaviors; persisted in SQLite.
- Security hardening: minimal Tauri capabilities, CSP without
  remote origins, SSRF defenses (scheme allow-list, private-address
  classification, per-hop redirect re-validation), hostile-input
  regression corpus, fail-closed debug surface (dev-only spike UI
  and IPC).
- Per-user NSIS installer (no admin rights), WebView2 bootstrapping,
  SHA-256-summed release artifacts with a traceable build manifest
  and dependency inventory.

### Improved

- Error UX: structured backend errors map to plain-language
  messages with recovery actions; internals stay folded in
  "Technical details".
- Long-name/number formatting: middle truncation, sensible byte and
  ETA formatting, zero-state "—" instead of fake zeros.
- Reduced-motion support and double-submit guards.

### Fixed

- Release builds compile fail-closed stubs for all debug commands
  (found and fixed during M11; E0308 in the stub macro).
- M0.5 spike prototype surface (sidebar entry, autorun, view) is
  now compiled out of release navigation (M12 blocker fix).
- The launch-queue drain command requested a mismatched Tauri state
  type, aborting real launches seconds after startup (latent since
  the Windows-integration milestone; found by 1.0 artifact
  self-testing and fixed).

### Security

- No telemetry, no accounts, no cloud services, no update checks,
  no hidden analytics (audited per milestone; see
  `docs/M10-network-surface.md` and `docs/M11-security-regression.md`).

### Known Limitations

- Media support depends on the WebView2/Edge codec pack of the
  host machine (H.264/AAC, VP9/Opus generally available; HEVC and
  AV1 vary by environment — conditionally supported).
- Installer is unsigned; SmartScreen will show an unknown-publisher
  prompt (code signing pending certificate).
- No built-in updater in 1.0; upgrade by running the new installer
  over the old installation (data preserved).
- BitTorrent rate limiting is per-session, not per-task.
- DNS-rebinding validate→connect window is not pinned (documented;
  low practical exposure).
- Startup-failure reporting goes to stderr (invisible to the user
  in release builds); data-safe but silent.

See `KNOWN_ISSUES.md` for the full tracked list.
