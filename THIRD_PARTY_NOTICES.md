# Third-Party Notices

This file tracks the **direct** dependencies of ifuyo Tide and their
licenses. A complete, automated inventory of all transitive dependencies
(`cargo tree` + npm license scan + SBOM) will be generated during release
hardening (M10/M11) and shipped with installers.

| Dependency | Version | License | Source | Notes |
|---|---|---|---|---|
| tauri (Rust) | 2.x | Apache-2.0 OR MIT | crates.io | Application framework |
| tauri-build (Rust) | 2.x | Apache-2.0 OR MIT | crates.io | Build integration |
| tauri-plugin-dialog (Rust) | 2.x | Apache-2.0 OR MIT | crates.io | Native directory picker |
| rusqlite (Rust) | 0.37.x | MIT | crates.io | SQLite bindings |
| SQLite (bundled) | (see rusqlite) | Public Domain | sqlite.org | Compiled into the binary |
| thiserror (Rust) | 2.x | Apache-2.0 OR MIT | crates.io | Error derive |
| uuid (Rust) | 1.x | Apache-2.0 OR MIT | crates.io | ID generation |
| serde / serde_json (Rust) | 1.x | Apache-2.0 OR MIT | crates.io | Serialization |
| **librqbit (Rust)** | **9.0.1** | **Apache-2.0** | crates.io | BitTorrent engine (production since M3; features: `disable-upload`, `stream`) |
| tokio (Rust) | 1.x | MIT | crates.io | Async runtime |
| reqwest (Rust) | 0.13.5 | Apache-2.0 OR MIT | crates.io | HTTP client (M1; rustls TLS, no auto-decompression) |
| sha2 (Rust) | 0.11 | Apache-2.0 OR MIT | crates.io | SHA-256 checksums |
| url (Rust) | 2.x | Apache-2.0 OR MIT | crates.io | URL parsing |
| bytes / futures-util (Rust) | 1.x / 0.3 | MIT OR Apache-2.0 | crates.io | Byte buffers / async traits |
| @tauri-apps/api (npm) | 2.x | Apache-2.0 OR MIT | npm | Frontend IPC |
| @tauri-apps/plugin-dialog (npm) | 2.x | Apache-2.0 OR MIT | npm | Picker JS API |
| @tauri-apps/cli (npm) | 2.x | Apache-2.0 OR MIT | npm | Dev/build tooling |
| react / react-dom (npm) | 19.x | MIT | npm | UI |
| vite (npm) | 7.x | MIT | npm | Build tool |
| typescript (npm) | 5.x | Apache-2.0 | npm | Type checking |
| vitest + testing-library (npm) | latest | MIT | npm | Testing (dev only) |

Version pins live in `Cargo.lock` / `pnpm-lock.yaml`; this table tracks
major lines only.

## Tauri plugins (M8 Windows integration)

| Package | Version | License | Direct | Used In |
|---|---|---|---|---|
| tauri-plugin-single-instance | 2.5.2 | MIT OR Apache-2.0 | direct (Rust) | single-instance enforcement (M8) |
| tauri-plugin-deep-link | 2.6.1 | MIT OR Apache-2.0 | direct (Rust) | tide:// protocol (M8) |
| tauri-plugin-autostart | 2.7.0 | MIT OR Apache-2.0 | direct (Rust) | optional launch-at-login (M8) |
| tauri-plugin-notification | 2.5.1 | MIT OR Apache-2.0 | direct (Rust+JS) | download notifications (M8) |
| tauri-plugin-window-state | 2.5.0 | MIT OR Apache-2.0 | direct (Rust) | window restore (M8) |
| tauri-plugin-opener | 2.7.0 | MIT OR Apache-2.0 | direct (Rust+JS) | open file / reveal in Explorer (M6/M8) |
| @tauri-apps/plugin-notification | 2.5.1 | MIT OR Apache-2.0 | direct (JS) | frontend notification calls |
| @tauri-apps/plugin-dialog | 2.4.0 | MIT OR Apache-2.0 | direct (JS) | directory/file pickers (M0/M6) |
