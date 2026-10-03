# Security Policy

## Status: M0 baseline

This document records the security boundaries that exist **today** (M0).
It does not claim to enumerate all future risks — the full threat model
and hardening pass happen in M10.

## Supported reporting

Report security vulnerabilities to **security@ifuyo.com** (planned
contact; until the project is published, use the repository's private
issue tracker). Please do not open public issues for exploitable bugs.

## Security boundaries

| Boundary | M0 measure |
|---|---|
| WebView → Rust | Tauri IPC only; explicit command allow-list; every input re-validated in Rust (`security::` module). Frontend validation is UX, never a security control. |
| Capabilities | Minimal: `core:default` + `dialog:allow-open`. No `fs`, `shell`, `http`, or process plugins/permissions. |
| CSP | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost`. No remote scripts, iframes, or CDNs. |
| Filesystem | The backend touches exactly two places: its own app-data directory (`%LOCALAPPDATA%\ifuyo\Tide\`) and, on explicit user action, the user-chosen download directory. No arbitrary path access exists at all in M0. |
| Errors | IPC failures are structured `{code, message}`; Rust internals, SQL errors and panics never reach the frontend; no `unwrap`/`expect` on user-input paths. |
| Logs | Structured JSON lines, UTC timestamps, ids and event names only — never URLs, magnets, paths, or user-provided names. |
| Dependencies | Minimal dependency set; `Cargo.lock` and lockfiles are committed; `--locked` is used for verification. Automated license/vuln scanning arrives with release hardening (M10). |

## Planned (future milestones)

- Local media stream gateway: loopback-only bind, random port, per-session
  capability token, no path-based access (M4).
- Media probe isolation in a separate process with resource limits (M10).
- Path normalization, containment, symlink/reparse-point checks (M1/M10).
- Fuzzing of range, path and URL parsers (M10).
