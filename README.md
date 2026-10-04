# ifuyo Tide

**Tide — Download. Watch. Flow.**

A calm, local-first downloader and progressive media player for Windows.

> **Current status: 1.0.0 — stable release.**
> The M0–M11 feature set is frozen and validated end-to-end on real
> installers: fresh install, upgrade, uninstall, crash recovery,
> data preservation, security regression. P0 = 0; open items with
> dispositions are tracked in `KNOWN_ISSUES.md`. See
> `CHANGELOG.md`, `docs/release-1.0.0.md` and `docs/M12-report.md`.

---

## What Tide is

Tide is a local Windows application (Tauri 2 + Rust + React + SQLite) built
around one idea: **you provide the source, Tide takes care of downloading,
managing, and — eventually — playing it progressively.**

Tide is **local-first**: no account, no cloud backend, no telemetry, no
upload of your data.

## Planned features

## Usage

1. **Add a source**: paste an HTTP(S) URL in Downloads, or drop a
   .torrent file / magnet link (or use `tide://add`).
2. **Download**: pause/resume anytime; completed files are verified
   (checksum) and imported into the Library automatically.
3. **Watch while downloading**: press Play on a running task —
   Tide streams progressively and seeks into not-yet-downloaded
   regions on demand.
4. **Library**: browse, re-watch, reveal files in Explorer.
5. **Settings**: download directory, bandwidth limits, task
   priorities, Windows behaviors (tray, autostart, notifications).

## Milestones

| Milestone | Feature | Status |
|---|---|---|
| M0 | Project skeleton, task domain, SQLite persistence, IPC, security baseline | **done** |
| M0.5 | Technology spike (streaming, WebView2 playback, librqbit feasibility) | **done** — full chain validated |
| M1 | HTTP/HTTPS download core | **done** — pause/resume/retry/checksum validated |
| M2 | Crash-safe persistence & resume | **done** — validated with crash-injection tests |
| M3 | Torrent engine (librqbit) | **done** — loopback-swarm tested |
| M4 | Progressive streaming, Range gateway, demand scheduler | **done** — unified gateway, progressive play + seek validated |
| M5 | Player | **done** — controller/state machine/UI, reload recovery, honest buffering |
| M6 | Local media library | **done** — scanner/reconciler/imports/query UI, benchmarked at 100k |
| M7 | Smart scheduler | **done** — deterministic policy, real enforcement, simulations + benchmarks |
| M8 | Windows integration | **done** — single instance, deep links, tray, autostart, notifications |
| M9 | Product polish | **done** — error UX, boundaries, loading states, strings centralization |
| M10 | Security hardening | complete |
| M11 | Release Candidate 0.2.0-rc.1 | complete (RC CONDITIONAL) |
| M12 | 1.0.0 stable release | **complete** |

## Requirements

- Windows 10 or 11, x64.
- WebView2 Runtime (auto-installed by the setup EXE if missing; it is
  preinstalled on up-to-date Windows 10/11).
- No admin rights required (per-user install). No Node/Rust/Python
  or other developer tools.

## Supported media

Playback uses the WebView2/Edge media stack of the host machine:

| Container | Codec | Status |
|---|---|---|
| MP4 | H.264 / AAC | Supported |
| WebM | VP9 / Opus | Supported |
| WebM | AV1 / Opus | Conditionally supported (varies by machine) |
| MKV | HEVC etc. | Conditionally supported (varies by machine) |

Unsupported codecs produce a structured error, never a crash.

## Installing, upgrading, uninstalling

- **Install**: run `ifuyo-tide_0.2.0-rc.1_x64-setup.exe` (NSIS,
  per-user, no admin rights; WebView2 bootstrapped if missing).
- **Data locations**: app data in `%LOCALAPPDATA%\ifuyo\Tide\`
  (database, logs, torrent metadata); downloaded files in the
  directory you choose. The three never share a root.
- **Upgrade**: close Tide, run the new installer over the old one —
  tasks, settings, library and downloaded files are preserved.
- **Uninstall**: Windows "Apps & features" or the Start-menu
  uninstaller — application files are removed, your app data and
  downloads are preserved by default (explicit checkbox to also
  remove app data).
- **Known limitations**: unsigned installer (SmartScreen prompt),
  no built-in updater — see `KNOWN_ISSUES.md`.

## Troubleshooting

- **App does not open after an interrupted install or corrupt app
  data**: reinstall over the existing installation; if it still
  fails, back up `%LOCALAPPDATA%\ifuyo\Tide\database	ide.db`
  and remove the `%LOCALAPPDATA%\ifuyo\Tide` folder (your
  downloaded files are elsewhere and are not affected).
- **A video fails to play**: check the codec row above — Tide shows
  a structured decode error for unsupported codecs; try MP4 (H.264).
- **Downloads stall**: check the global/per-task bandwidth limits in
  Settings, and the task priority in the Downloads view.
- **Installer warning (SmartScreen)**: the 1.0 installer is not yet
  code-signed; verify the SHA-256 in `release/SHA256SUMS.txt`.

## Architecture overview

```text
Frontend (React + TypeScript + Vite, WebView2)
        ↓ Tauri IPC (typed commands, structured errors)
Application Core (Rust)
        ↓
Domain (Task / Source / Resource / PlaybackSession / LibraryItem)
        ↓
Repository boundary
        ↓
SQLite (migrations, %LOCALAPPDATA%\ifuyo\Tide\database)
```

The download/playback chain
(`HTTP Engine / Torrent Engine → Resource → Availability → Scheduler →
Gateway → Player`) is **implemented as of M4(上)** — one gateway, one
player, every source kind. Details:
[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md),
[docs/M4-streaming-architecture.md](docs/M4-streaming-architecture.md).

## Development setup

Prerequisites: Windows 11 x64, Node ≥ 20, pnpm, Rust (stable, MSVC),
VS Build Tools with C++ workload, WebView2 Runtime.

```powershell
pnpm install
pnpm tauri dev      # run the desktop app in dev mode
```

Build, typecheck and tests:

```powershell
pnpm build          # frontend typecheck + production build
pnpm test           # frontend render tests (vitest)
cd src-tauri
cargo check --locked
cargo test --locked
```

More detail: [docs/development/GETTING_STARTED.md](docs/development/GETTING_STARTED.md).

## Security

M0 ships a minimal security baseline: least-privilege Tauri capabilities
(core + directory picker only), a strict CSP, no arbitrary filesystem /
shell / process permissions, Rust-side validation of every IPC input, and
structured `{code, message}` error responses. See [SECURITY.md](SECURITY.md).

## Privacy

Local-first. Tide does not send download URLs, magnets, info hashes, file
names, paths, content, history or behaviour anywhere. Tide provides no
anonymous network service and makes no such claims. See [PRIVACY.md](PRIVACY.md).

## Content policy

Tide is a general-purpose tool. It does not provide, search, index or
recommend content, and it does not bypass DRM or paywalls. Users are
responsible for having the necessary rights to the content they use with
Tide. See [CONTENT_POLICY.md](CONTENT_POLICY.md).

## License

The final license is **not yet decided**; see [LICENSE](LICENSE). All
third-party dependency licenses are tracked in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
