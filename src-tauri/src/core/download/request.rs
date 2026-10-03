//! DownloadRequest (M1 §5): the immutable input of one download.

use super::checksum::ChecksumAlgorithm;
use super::file_sink::OverwritePolicy;
use super::url::ValidatedUrl;
use std::path::PathBuf;

/// Retry policy constants (M1 §19): bounded, with backoff + jitter.
pub const MAX_RETRY_ATTEMPTS: u32 = 4;
pub const RETRY_BASE_DELAY_MS: u64 = 500;
pub const RETRY_MAX_DELAY_MS: u64 = 8_000;

/// Maximum redirects (M1 §7 / §62).
pub const MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    /// Unique id (UUIDv4, M1 §43 — never URL/filename/timestamp).
    pub download_id: String,
    pub url: ValidatedUrl,
    /// User-chosen download directory (canonicalized by FileSink::plan).
    pub destination_dir: PathBuf,
    pub filename: Option<String>,
    pub overwrite_policy: OverwritePolicy,
    /// Optional integrity expectation from the user.
    pub expected_checksum: Option<(ChecksumAlgorithm, String)>,
}

impl DownloadRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        url: ValidatedUrl,
        destination_dir: PathBuf,
        filename: Option<String>,
        overwrite_policy: OverwritePolicy,
        expected_checksum: Option<(ChecksumAlgorithm, String)>,
    ) -> Result<DownloadRequest, crate::error::TideError> {
        Ok(DownloadRequest {
            download_id: crate::core::ids::new_id(),
            url,
            destination_dir,
            filename,
            overwrite_policy,
            expected_checksum,
        })
    }
}

/// Exponential backoff with full jitter, capped (M1 §19).
pub fn retry_delay_ms(attempt: u32) -> u64 {
    let base = RETRY_BASE_DELAY_MS.saturating_mul(
        1u64.checked_shl(attempt.min(16))
            .unwrap_or(RETRY_MAX_DELAY_MS),
    );
    let capped = base.min(RETRY_MAX_DELAY_MS);
    // Full jitter within [base/2, capped]
    let jitter_source = crate::core::ids::new_id();
    let seed = u64::from_str_radix(&jitter_source[0..8], 16).unwrap_or(0);
    let span = (capped - capped / 2).max(1);
    capped / 2 + seed % span
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_is_bounded_and_grows() {
        let mut samples = Vec::new();
        for attempt in 0..8u32 {
            let d = retry_delay_ms(attempt);
            assert!(d <= RETRY_MAX_DELAY_MS, "attempt {attempt} delay {d}");
            samples.push(d);
        }
        assert!(
            samples.iter().any(|d| *d >= 1000),
            "later attempts reach larger delays"
        );
    }

    #[test]
    fn retry_delay_varies_with_jitter() {
        let a = retry_delay_ms(3);
        let mut differs = false;
        for _ in 0..20 {
            if retry_delay_ms(3) != a {
                differs = true;
                break;
            }
        }
        assert!(differs, "full jitter should vary between calls");
    }
}
