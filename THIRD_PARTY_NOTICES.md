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
| **librqbit (Rust)** | **9.0.1** | **Apache-2.0** | crates.io | BitTorrent engine (validated in M0.5) |
| tokio (Rust) | 1.x | MIT | crates.io | Async runtime |
| @tauri-apps/api (npm) | 2.x | Apache-2.0 OR MIT | npm | Frontend IPC |
| @tauri-apps/plugin-dialog (npm) | 2.x | Apache-2.0 OR MIT | npm | Picker JS API |
| @tauri-apps/cli (npm) | 2.x | Apache-2.0 OR MIT | npm | Dev/build tooling |
| react / react-dom (npm) | 19.x | MIT | npm | UI |
| vite (npm) | 7.x | MIT | npm | Build tool |
| typescript (npm) | 5.x | Apache-2.0 | npm | Type checking |
| vitest + testing-library (npm) | latest | MIT | npm | Testing (dev only) |

**librqbit** (Apache-2.0) will be added when the torrent engine lands (M3).

Version pins live in `Cargo.lock` / `pnpm-lock.yaml`; this table tracks
major lines only.
