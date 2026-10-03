# ifuyo Tide

**Tide — Download. Watch. Flow.**

A calm, local-first downloader and progressive media player for Windows.

> **Current status: M2 — Task Persistence / Crash Recovery / Resume complete.**
> The core chain `Torrent → librqbit → partial data → local Range
> Gateway → WebView2 <video> → play/seek` has been validated end-to-end
> with measured data (see `docs/M0.5-report.md`). Tide can start, create
> and persist tasks, and survive restarts; downloading real user
> sources lands in M1/M3.

---

## What Tide is

Tide is a local Windows application (Tauri 2 + Rust + React + SQLite) built
around one idea: **you provide the source, Tide takes care of downloading,
managing, and — eventually — playing it progressively.**

Tide is **local-first**: no account, no cloud backend, no telemetry, no
upload of your data.

## Planned features

| Milestone | Feature | Status |
|---|---|---|
| M0 | Project skeleton, task domain, SQLite persistence, IPC, security baseline | **done** |
| M0.5 | Technology spike (streaming, WebView2 playback, librqbit feasibility) | **done** — full chain validated |
| M1 | HTTP/HTTPS download core | **done** — pause/resume/retry/checksum validated |
| M2 | Crash-safe persistence & resume | **done** — validated with crash-injection tests |
| M3 | Torrent engine (librqbit) | planned |
| M4 | Progressive streaming, Range gateway, demand scheduler | planned |
| M5 | Player | planned |
| M6 | Local media library | planned |
| M7 | Smart scheduler | planned |
| M8 | Windows integration | planned |
| M9 | Product polish | planned |
| M10–M12 | Security hardening, RC, 1.0 | planned |

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
Gateway → Player`) is **designed but not implemented** in M0. Details:
[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md).

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
