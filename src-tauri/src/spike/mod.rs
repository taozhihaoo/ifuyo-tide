// M0.5 SPIKE ============================================================
// Everything under `spike/` is EXPERIMENTAL code for the M0.5 technical
// spike. It must never leak into production modules: commands live in
// `commands/spike.rs`, state is managed separately (`SpikeState`), and
// the final report (docs/M0.5-report.md) decides which pieces graduate
// to M1/M3/M4 and which are deleted.
// =======================================================================

pub mod events;
pub mod fixture_server;
pub mod fixtures;
pub mod gateway;
pub mod range;
pub mod resource;
pub mod state;
pub mod torrent;
