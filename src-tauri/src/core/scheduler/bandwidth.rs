//! Bandwidth estimation & enforcement primitives (M7(上) §41/§24/§25).

use std::time::Duration;

/// §41: exponential moving average over throughput samples.
/// `alpha` comes from the config side (centralized, §73).
#[derive(Debug, Clone)]
pub struct Ema {
    alpha: f64,
    value: f64,
    seeded: bool,
}

impl Ema {
    pub fn new(alpha: f64) -> Ema {
        Ema {
            alpha: alpha.clamp(0.01, 1.0),
            value: 0.0,
            seeded: false,
        }
    }

    pub fn push(&mut self, sample_bps: f64) {
        if !self.seeded {
            self.value = sample_bps;
            self.seeded = true;
        } else {
            self.value = self.alpha * sample_bps + (1.0 - self.alpha) * self.value;
        }
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

/// A token bucket pacer: `take(n)` waits until n bytes fit the bucket
/// refilled at `rate_bps`. Rate 0 = paused; the ENGINE owns the actual
/// wait (§3: the scheduler sets rates, the data plane enforces them).
#[derive(Debug)]
pub struct TokenBucket {
    rate_bps: std::sync::atomic::AtomicU64,
    allowed: std::sync::atomic::AtomicBool,
    inner: std::sync::Mutex<BucketState>,
}

#[derive(Debug)]
struct BucketState {
    tokens: f64,
    last_refill: std::time::Instant,
}

impl TokenBucket {
    pub fn new(rate_bps: u64) -> TokenBucket {
        TokenBucket {
            rate_bps: std::sync::atomic::AtomicU64::new(rate_bps),
            allowed: std::sync::atomic::AtomicBool::new(rate_bps > 0),
            inner: std::sync::Mutex::new(BucketState {
                tokens: 64.0 * 1024.0,
                last_refill: std::time::Instant::now(),
            }),
        }
    }

    /// Updates the enforced rate (scheduler allocation). 0 pauses.
    pub fn set_rate_bps(&self, rate_bps: u64) {
        self.rate_bps
            .store(rate_bps, std::sync::atomic::Ordering::Relaxed);
        self.allowed
            .store(rate_bps > 0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn rate_bps(&self) -> u64 {
        self.rate_bps.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// How long the caller must wait before `bytes` may pass. Returns
    /// `Duration::ZERO` when immediately allowed. The CALLER performs
    /// the sleep (tokio sleep in the engine loop) — this type stays
    /// sync and testable.
    pub fn delay_for(&self, bytes: u64) -> Duration {
        if !self.allowed.load(std::sync::atomic::Ordering::Relaxed) {
            return Duration::ZERO; // unlimited mode
        }
        let rate = self.rate_bps.load(std::sync::atomic::Ordering::Relaxed);
        if rate == 0 {
            return Duration::ZERO;
        }
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        state.tokens = (state.tokens + elapsed * rate as f64).min(rate as f64); // burst cap = 1s
        state.last_refill = now;
        if state.tokens >= bytes as f64 {
            state.tokens -= bytes as f64;
            return Duration::ZERO;
        }
        let missing = bytes as f64 - state.tokens;
        Duration::from_secs_f64(missing / rate as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ema_smooths() {
        let mut ema = Ema::new(0.3);
        ema.push(1000.0);
        assert_eq!(ema.value(), 1000.0, "first sample seeds");
        ema.push(2000.0);
        assert!((ema.value() - 1300.0).abs() < 1e-9);
    }

    #[test]
    fn unlimited_bucket_never_delays() {
        let bucket = TokenBucket::new(0);
        assert_eq!(bucket.delay_for(1 << 30), Duration::ZERO);
    }

    #[test]
    fn paced_bucket_delays_proportionally() {
        let bucket = TokenBucket::new(1_000_000); // 1 MiB/s
                                                  // First small take fits the initial burst budget.
        assert_eq!(bucket.delay_for(64 * 1024), Duration::ZERO);
        // A take beyond the 1-second burst cap must wait.
        let wait = bucket.delay_for(4 * 1024 * 1024);
        assert!(
            wait > Duration::from_secs(2),
            "expected multi-second wait, got {wait:?}"
        );
        // Raising the rate shrinks the wait.
        bucket.set_rate_bps(8_000_000);
        assert!(bucket.delay_for(4 * 1024 * 1024) < wait);
    }
}
