# Getting Started

## Prerequisites (Windows 11 x64)

| Tool | Version | Notes |
|---|---|---|
| Node.js | ≥ 20 | tested with 24.x |
| pnpm | ≥ 10 | tested with 12.x |
| Rust | stable, **MSVC** target | `rustup default stable-x86_64-pc-windows-msvc` |
| VS Build Tools | 2022 | workload "Desktop development with C++" (link.exe) |
| WebView2 Runtime | evergreen | preinstalled on Windows 11 |

Make sure `cargo` is on your PATH (`%USERPROFILE%\.cargo\bin`).

## Install

```powershell
pnpm install
```

## Run in development

```powershell
pnpm tauri dev
```

This starts Vite (port 5173, strict) and the Rust backend, then opens the
Tide window. First compile takes several minutes.

## Build

```powershell
pnpm build          # frontend: tsc --noEmit + vite build
pnpm tauri build    # full release build (bundle targets currently "none")
```

## Test

```powershell
pnpm test                      # frontend render tests (vitest + jsdom)
cd src-tauri
cargo check --locked           # typecheck Rust
cargo test --locked            # Rust unit + integration tests
```

## Directory structure

```text
src/                  frontend (React)
  app/                shell + layout
  components/         shared UI
  features/           tasks / library / player / settings
  lib/                api + ipc + wire types
  test/               frontend tests
src-tauri/
  src/
    app/              startup, state, app dirs
    commands/         Tauri IPC commands
    core/             domain: task / source / resource / scheduler / playback / library
    persistence/      SQLite open, migrations, repositories
    security/         input validation (trust boundary)
    diagnostics/      structured logging
  migrations…         embedded in persistence (0001_initial.sql)
  capabilities/       Tauri permission sets
tests/fixtures/       legal test media only (see its README)
docs/                 architecture / development docs
```

## Troubleshooting

- **`cargo: command not found`** — add `%USERPROFILE%\.cargo\bin` to PATH
  (Git Bash: `export PATH="/c/Users/Gray/.cargo/bin:$PATH"`).
- **`error: linker link.exe not found`** — install VS Build Tools with the
  C++ workload.
- **pnpm blocks esbuild build scripts** — `pnpm-workspace.yaml` carries
  `allowBuilds: esbuild: true`; don't remove it.
- **Port 5173 in use** — Vite uses `strictPort`; free the port or change
  `vite.config.ts` + `tauri.conf.json devUrl` together.
- **App data location** — `%LOCALAPPDATA%\ifuyo\Tide\` (database + logs).
  Deleting that folder resets app state; it never touches your downloads.
