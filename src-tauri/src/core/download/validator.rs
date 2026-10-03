//! Resume validators: ETag / Last-Modified (M1 §13).
//!
//! Resume safety rule (recorded in docs/M1-resume-and-range.md):
//!   * validators known  → send `Range: bytes=N-` + `If-Range: <validator>`
//!     (server returns 206 only when unchanged; a changed resource yields
//!     200 full → we restart from zero)
//!   * no validators     → send plain `Range: bytes=N-` and VERIFY the
//!     206's `Content-Range` consistency with the local size; the remote
//!     could still have changed — accepted risk only when the server gave
//!     us no validator at all (weak but explicit, M1 scope).

use serde::Serialize;

/// Validators captured from the first response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseValidators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl ResponseValidators {
    pub fn from_headers(etag: Option<&str>, last_modified: Option<&str>) -> ResponseValidators {
        ResponseValidators {
            etag: etag.map(str::to_string),
            last_modified: last_modified.map(str::to_string),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.etag.is_none() && self.last_modified.is_none()
    }

    /// The validator to use with `If-Range`. RFC 9110 §13.1.5: an ETag
    /// validator is preferred; Last-Modified is only usable when no ETag
    /// exists (HTTP-date comparators are weaker).
    pub fn if_range_value(&self) -> Option<&str> {
        self.etag.as_deref().or(self.last_modified.as_deref())
    }

    /// Whether the remote validators (from a resume attempt's response
    /// headers) match the ones recorded when the partial was written.
    pub fn remote_matches(&self, remote: &ResponseValidators) -> bool {
        if self.is_empty() || remote.is_empty() {
            // Nothing to compare: treat as "unknown", handled by the
            // resume decision (If-Range absent / Content-Range check).
            return true;
        }
        if let (Some(local_etag), Some(remote_etag)) = (&self.etag, &remote.etag) {
            return local_etag == remote_etag;
        }
        self.last_modified == remote.last_modified
    }
}

/// The resume decision for one download (re)start, given the local
/// partial file size and the validators recorded for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeDecision {
    /// No local bytes: plain full download.
    FullDownload,
    /// Local bytes exist; try `Range: bytes=N-` with `If-Range` when a
    /// validator is available.
    TryResume { local_bytes: u64 },
}

pub fn decide_resume(local_bytes: u64) -> ResumeDecision {
    if local_bytes == 0 {
        ResumeDecision::FullDownload
    } else {
        ResumeDecision::TryResume { local_bytes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_preferred_over_last_modified() {
        let v = ResponseValidators::from_headers(Some("\"abc\""), Some("yesterday"));
        assert_eq!(v.if_range_value(), Some("\"abc\""));
        let v = ResponseValidators::from_headers(None, Some("yesterday"));
        assert_eq!(v.if_range_value(), Some("yesterday"));
        assert_eq!(v.if_range_value(), v.last_modified.as_deref());
    }

    #[test]
    fn empty_validators_match_anything() {
        let empty = ResponseValidators::default();
        let remote = ResponseValidators::from_headers(Some("\"x\""), None);
        assert!(empty.remote_matches(&remote));
        assert!(remote.remote_matches(&empty));
    }

    #[test]
    fn etag_mismatch_detected() {
        let local = ResponseValidators::from_headers(Some("\"v1\""), None);
        let same = ResponseValidators::from_headers(Some("\"v1\""), None);
        let changed = ResponseValidators::from_headers(Some("\"v2\""), None);
        assert!(local.remote_matches(&same));
        assert!(!local.remote_matches(&changed));
    }

    #[test]
    fn last_modified_fallback_comparison() {
        let local = ResponseValidators::from_headers(None, Some("Mon, 01 Jan 2026 00:00:00 GMT"));
        let same = ResponseValidators::from_headers(None, Some("Mon, 01 Jan 2026 00:00:00 GMT"));
        let changed = ResponseValidators::from_headers(None, Some("Tue, 02 Jan 2026 00:00:00 GMT"));
        assert!(local.remote_matches(&same));
        assert!(!local.remote_matches(&changed));
    }

    #[test]
    fn resume_decision_zero_vs_partial() {
        assert_eq!(decide_resume(0), ResumeDecision::FullDownload);
        assert_eq!(
            decide_resume(1024),
            ResumeDecision::TryResume { local_bytes: 1024 }
        );
    }
}
