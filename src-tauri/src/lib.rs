pub mod app;
pub mod commands;
pub mod core;
pub mod diagnostics;
pub mod dto;
pub mod error;
pub mod persistence;
pub mod platform;
// M0.5 SPIKE — experimental technical-spike code, see spike/mod.rs.
pub mod security;
pub mod spike;

pub use app::run;
