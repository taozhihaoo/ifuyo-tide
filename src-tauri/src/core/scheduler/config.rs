//! Scheduler configuration (M7(上) §73/§74).
//!
//! Every knob lives here with a documented INITIAL DEFAULT — none of
//! these numbers is claimed to be optimal (§74); they are starting
//! points chosen from the M4/M5 byte-window experience and are meant
//! to be tuned by experiment. No magic numbers anywhere else in the
//! scheduler.

use serde::{Deserialize, Serialize};

/// Seconds of REAL browser-buffered-ahead audio/video (M5 reports the
/// element's buffered ranges — M6上 §149 distinction: browser buffered
/// vs backend available). §74: INITIAL DEFAULTS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackWatermarks {
    /// Below this the session is STARTUP/CRITICAL territory.
    pub critical_secs: u32,
    /// Below this the session is LOW_BUFFER.
    pub low_secs: u32,
    /// At/above this the session is healthy PLAYING.
    pub target_secs: u32,
    /// Leaving CRITICAL requires REACHING this (not 1 second — §31).
    pub recovery_secs: u32,
    /// §28: the startup phase ends once this much is buffered.
    pub startup_target_secs: u32,
}

impl Default for PlaybackWatermarks {
    fn default() -> Self {
        PlaybackWatermarks {
            critical_secs: 4,
            low_secs: 12,
            target_secs: 30,
            recovery_secs: 10,
            startup_target_secs: 10,
        }
    }
}

/// All scheduler knobs (§73). Serialize-able and test-friendly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub playback: PlaybackWatermarks,
    /// §24: global download bandwidth cap in bytes/sec; `None` =
    /// unlimited. Playback reserves come out of this cap.
    pub global_bandwidth_bps: Option<u64>,
    /// §22/§21: aging adds up to `aging_max_bonus` weight after
    /// `aging_secs` of waiting — capped so Background never outranks
    /// Emergency (§21).
    pub aging_secs: u64,
    pub aging_max_bonus: f64,
    /// §22 base weights per class — decomposable, not a 30-term
    /// formula.
    pub weight_emergency: f64,
    pub weight_interactive: f64,
    pub weight_playback: f64,
    pub weight_normal: f64,
    pub weight_background: f64,
    /// §59: when every playback buffer is ≥ target, background gets at
    /// least this fraction of the leftover capacity (anti-starvation).
    pub background_floor_share: f64,
    /// §27 minimum guaranteed allocation for a CRITICAL playback
    /// session (bytes/sec) inside the global cap.
    pub emergency_min_bps: u64,
    /// §29 target allocation for a healthy playing session.
    pub playback_target_bps: u64,
    /// §68: active download caps (HTTP only — the torrent engine
    /// manages its own peer concurrency).
    pub max_active_downloads: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        SchedulerConfig {
            playback: PlaybackWatermarks::default(),
            global_bandwidth_bps: None, // unlimited until the user sets one
            aging_secs: 60,
            aging_max_bonus: 0.5, // Background can age up to +0.5 weight — never past Playback (§21 cap)
            weight_emergency: 16.0,
            weight_interactive: 6.0,
            weight_playback: 4.0,
            weight_normal: 2.0,
            weight_background: 1.0,
            background_floor_share: 0.2,
            emergency_min_bps: 2 * 1024 * 1024,   // 2 MiB/s
            playback_target_bps: 6 * 1024 * 1024, // 6 MiB/s
            max_active_downloads: 3,
        }
    }
}

impl SchedulerConfig {
    /// §22 base weight of a class.
    pub fn weight_of(&self, class: PriorityClass) -> f64 {
        match class {
            PriorityClass::Emergency => self.weight_emergency,
            PriorityClass::Interactive => self.weight_interactive,
            PriorityClass::Playback => self.weight_playback,
            PriorityClass::Normal => self.weight_normal,
            PriorityClass::Background => self.weight_background,
            PriorityClass::Maintenance => 0.5,
        }
    }
}

/// §7: semantic priority classes, mapped to weights by the config —
/// the class is NOT the final decision (§7), only its base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorityClass {
    Emergency,
    Interactive,
    Playback,
    Normal,
    Background,
    Maintenance,
}

/// §23: what the USER asked for. Not an absolute command — Emergency
/// playback may still outrank a user-High download, with the reason
/// recorded (§23/§16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserPriority {
    Highest,
    High,
    Normal,
    Low,
    Background,
}

impl UserPriority {
    /// The class a user request maps to (before playback/urgency
    /// adjustments).
    pub fn base_class(self) -> PriorityClass {
        match self {
            UserPriority::Highest => PriorityClass::Interactive,
            UserPriority::High => PriorityClass::Normal,
            UserPriority::Normal => PriorityClass::Normal,
            UserPriority::Low => PriorityClass::Background,
            UserPriority::Background => PriorityClass::Background,
        }
    }

    /// §23: a user bump lifts within the same band (weight multiplier).
    pub fn weight_multiplier(self) -> f64 {
        match self {
            UserPriority::Highest => 2.0,
            UserPriority::High => 1.5,
            UserPriority::Normal => 1.0,
            UserPriority::Low => 0.7,
            UserPriority::Background => 0.4,
        }
    }
}
