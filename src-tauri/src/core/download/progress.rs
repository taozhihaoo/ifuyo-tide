//! Download progress: counters, speed smoothing, ETA (M1 §24–§27).

use serde::Serialize;
use std::time::{Duration, Instant};

/// Smoothed speed tracker: exponential moving average over chunk-sized
/// samples. UI must not jitter because a single chunk was large.
#[derive(Debug)]
pub struct SpeedTracker {
    ema_bps: f64,
    last_sample: Option<Instant>,
    alpha: f64,
}

impl SpeedTracker {
    pub fn new(alpha: f64) -> SpeedTracker {
        SpeedTracker {
            ema_bps: 0.0,
            last_sample: None,
            alpha: alpha.clamp(0.01, 1.0),
        }
    }

    /// Records `bytes` received now; updates the EMA.
    pub fn record(&mut self, bytes: u64, now: Instant) {
        if bytes == 0 {
            return;
        }
        match self.last_sample {
            None => {
                // First sample: assume the chunk took its fair share of a
                // nominal 1 s window to avoid a single-chunk spike.
                self.ema_bps = bytes as f64;
            }
            Some(last) => {
                let dt = now.duration_since(last).as_secs_f64().max(0.001);
                let instant = bytes as f64 / dt;
                self.ema_bps = if self.ema_bps == 0.0 {
                    instant
                } else {
                    self.alpha * instant + (1.0 - self.alpha) * self.ema_bps
                };
            }
        }
        self.last_sample = Some(now);
    }

    pub fn smoothed_bps(&self) -> f64 {
        self.ema_bps
    }
}

/// Progress snapshot crossing the DTO boundary (M1 §24).
///
/// All byte counters are u64 (>4 GB files, M1 §29). `total_bytes` is
/// legitimately `None` (unknown length / chunked transfer); percentage
/// and ETA are then `None` — never fabricated.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    /// 0.0..=100.0, only when total is known.
    pub percentage: Option<f64>,
    /// Smoothed bytes/second.
    pub speed_bps: u64,
    /// Seconds remaining, only when total and speed > 0 are known.
    pub eta_secs: Option<u64>,
    pub elapsed_ms: u64,
}

/// Computes a progress snapshot from raw counters.
pub fn compute_progress(
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    speed_bps: f64,
    elapsed: Duration,
) -> DownloadProgress {
    let percentage = total_bytes
        .filter(|t| *t > 0)
        .map(|t| ((downloaded_bytes as f64 / t as f64) * 100.0).clamp(0.0, 100.0));
    let eta_secs = total_bytes.and_then(|t| {
        let remaining = t.checked_sub(downloaded_bytes)?;
        if speed_bps > 0.0 && remaining > 0 {
            Some((remaining as f64 / speed_bps).ceil() as u64)
        } else if remaining == 0 {
            Some(0)
        } else {
            None
        }
    });
    DownloadProgress {
        downloaded_bytes,
        total_bytes,
        percentage,
        speed_bps: speed_bps as u64,
        eta_secs,
        elapsed_ms: elapsed.as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_tracker_smooths_spikes() {
        let mut tracker = SpeedTracker::new(0.3);
        let t0 = Instant::now();
        tracker.record(1_000_000, t0);
        // Second sample: 1 MB in 100 ms → instant 10 MB/s.
        tracker.record(1_000_000, t0 + Duration::from_millis(100));
        let first = tracker.smoothed_bps();
        assert!(first > 1_000_000.0, "EMA should rise toward 10MB/s: {first}");
        // Several slow samples pull the EMA down gradually (no single-chunk
        // jumps).
        for i in 1..=10 {
            tracker.record(10_000, t0 + Duration::from_millis(100) + Duration::from_millis(1000 * i));
        }
        let later = tracker.smoothed_bps();
        assert!(later < first, "EMA must decay smoothly: {later} < {first}");
    }

    #[test]
    fn compute_progress_known_total() {
        let p = compute_progress(500, Some(1000), 100.0, Duration::from_secs(5));
        assert_eq!(p.percentage, Some(50.0));
        assert_eq!(p.eta_secs, Some(5));
        assert_eq!(p.speed_bps, 100);
    }

    #[test]
    fn compute_progress_unknown_total() {
        let p = compute_progress(500, None, 100.0, Duration::from_secs(5));
        assert_eq!(p.total_bytes, None);
        assert_eq!(p.percentage, None);
        assert_eq!(p.eta_secs, None);
        assert_eq!(p.downloaded_bytes, 500);
    }

    #[test]
    fn compute_progress_complete_and_zero_speed() {
        let p = compute_progress(1000, Some(1000), 0.0, Duration::from_secs(9));
        assert_eq!(p.percentage, Some(100.0));
        assert_eq!(p.eta_secs, Some(0));
        let p = compute_progress(500, Some(1000), 0.0, Duration::from_secs(1));
        assert_eq!(p.eta_secs, None, "no ETA without speed");
    }

    #[test]
    fn progress_never_fabricates_percentage() {
        // downloaded > total (server lied): clamp, never exceed 100.
        let p = compute_progress(1500, Some(1000), 10.0, Duration::from_secs(1));
        assert_eq!(p.percentage, Some(100.0));
    }

    #[test]
    fn u64_scales_are_preserved() {
        let five_gb: u64 = 5 * 1024 * 1024 * 1024;
        let p = compute_progress(five_gb - 1, Some(five_gb), 1.0, Duration::from_secs(1));
        assert_eq!(p.downloaded_bytes, five_gb - 1);
        assert_eq!(p.total_bytes, Some(five_gb));
    }
}
