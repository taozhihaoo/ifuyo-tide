//! M8 — Windows platform integration boundary.
//!
//! This module is the SINGLE entry point between the Windows host
//! (CLI args, deep links, tray, notifications, autostart, window
//! state, file associations) and the application layer. Windows is
//! the HOST ENVIRONMENT, not a business layer (M8 §1/§3): nothing
//! here touches DownloadEngine/TorrentEngine/Scheduler internals —
//! external inputs become validated `LaunchRequest`s and are routed
//! to the same application commands the UI uses.
//!
//! Components:
//! - `launch.rs` LaunchRequest parsing + validation (untrusted input)
//! - `router.rs` pending-request queue + application routing
//! - `tray.rs` system tray (real-state projection only)
//! - `settings.rs` Windows-integration settings (close-to-tray,
//!   notifications, autostart) — persisted via the existing settings
//!   KV (§80: no second store)
//!
//! Plugin notes (§66): official Tauri 2 plugins at the locked
//! versions — single-instance MUST be registered FIRST (plugin doc
//! requirement); deep-link handles tide:// on Windows via CLI args
//! and integrates with single-instance forwarding.

pub mod launch;
pub mod router;
pub mod settings;
pub mod tray;

pub use launch::{LaunchKind, LaunchRequest};
pub use router::{route_launch_request, PendingLaunchQueue};
pub use settings::WindowsSettings;
