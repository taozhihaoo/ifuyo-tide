//! M2 — Task Recovery.
//!
//! Startup scan + filesystem reconciliation (M2 §9/§57). The decision
//! logic is pure; the service applies decisions to the repository and
//! registers idle runtime entries with the engine.

pub mod decision;
pub mod service;

pub use decision::{FilesystemFacts, RecoveryAction, RecoveryDecision, RecoveryReason};
pub use service::{reconcile_one, recover_all, RecoveryOutcome};
