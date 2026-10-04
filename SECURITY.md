# Security Policy

## Status: M10 — Security & Release Hardening complete

This document reflects the security boundaries as hardened in M10.
The full threat model, attack-surface inventory and audit findings
live in docs/M10-threat-model.md and docs/M10-security-audit.md.

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
| Dependencies | `Cargo.lock` committed; `--locked` verification; `cargo deny check` gates advisories/licenses/bans/sources (deny.toml). RustSec-ignored entries carry documented justification. |
| Stream Gateway | Loopback-only bind (127.0.0.1, random port), per-session UUIDv4 capability token (runtime-only, revoked on stop), Host-header validation, single-range parsing, 16 KiB header cap, bounded 256 KiB chunks (M4). |
| Windows integration | Single instance (second launches forward args + exit); tide:// deep link and .torrent association route through validated LaunchRequest parsing — no path/URL reaches the backend unvalidated (M8). |

## Implemented through M10

- Local Stream Gateway: loopback-only, random port, capability token,
  no path-based access (M4).
- Path normalization, containment, reparse-point/junction refusal
  (M1/M6/M10).
- Fuzz/property coverage of range, path and URL parsing (M4/M9/M10
  corpora).
- Debug command surface removed from release builds (M10).
