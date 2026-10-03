//! Download URL validation and SSRF baseline (M1 §6–§8).

use crate::error::TideError;
use std::net::{Ipv4Addr, Ipv6Addr};
use url::Url;

/// A validated download URL.
///
/// Only `http` / `https` schemes are accepted, and by default hosts
/// resolving to local/private ranges are rejected (M1 §8: public
/// internet downloads allowed, local/private denied unless explicitly
/// opted in — used by tests and, later, an explicit LAN feature).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedUrl {
    pub inner: Url,
    pub host: String,
    /// True when the host is loopback/private/link-local and the caller
    /// explicitly allowed private addresses.
    pub private_allowed: bool,
}

impl ValidatedUrl {
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

/// Classifies an IP as local/private per the M1 SSRF baseline.
pub fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.octets()[0] == 100 && (v4.octets()[1] & 0b1100_0000) == 64 // CGNAT 100.64/10
                || is_benchmark_v4(&v4)
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
}

/// 192.0.2.0/24 (TEST-NET-1), 198.51.100.0/24, 203.0.113.0/24.
fn is_benchmark_v4(v4: &Ipv4Addr) -> bool {
    let o = v4.octets();
    (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
}

/// Literal-IP classification for hostnames that ARE IPs (no DNS yet).
fn host_is_private_literal(host: &str) -> Option<bool> {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(v4) = host.parse::<Ipv4Addr>() {
        return Some(is_private_ip(std::net::IpAddr::V4(v4)));
    }
    if let Ok(v6) = host.parse::<Ipv6Addr>() {
        return Some(is_private_ip(std::net::IpAddr::V6(v6)));
    }
    None
}

/// Validates a download URL: scheme allow-list, non-empty host, and the
/// M1 SSRF baseline for literal IP hosts.
///
/// DNS-resolving hostnames to detect rebinding into private ranges is
/// NOT done in M1 (documented limitation; reqwest performs resolution
/// at request time). Redirect-time re-validation happens in the
/// transport's redirect policy.
pub fn validate_download_url(raw: &str, allow_private: bool) -> Result<ValidatedUrl, TideError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(TideError::Download(DownloadFailure::InvalidUrl {
            reason: "URL must not be empty".into(),
        }));
    }
    let parsed = url::Url::parse(raw).map_err(|_| {
        TideError::Download(DownloadFailure::InvalidUrl {
            reason: "URL is not a valid absolute URL".into(),
        })
    })?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(TideError::Download(DownloadFailure::UnsupportedScheme {
            scheme,
        }));
    }
    let host = parsed.host_str().unwrap_or_default().to_string();
    if host.is_empty() {
        return Err(TideError::Download(DownloadFailure::InvalidUrl {
            reason: "URL has no host".into(),
        }));
    }
    // Credentials in URLs are a smell for a downloader and risk leaking
    // into logs via referer-like paths; reject rather than silently keep.
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(TideError::Download(DownloadFailure::InvalidUrl {
            reason: "URL must not contain credentials".into(),
        }));
    }

    let literal_private = host_is_private_literal(&host);
    if let Some(true) = literal_private {
        if !allow_private {
            return Err(TideError::Download(
                DownloadFailure::PrivateAddressBlocked { host },
            ));
        }
    }
    // Hostnames that are NOT literal IPs are allowed in M1 without DNS
    // resolution (SSRF baseline covers literal addresses); documented.

    Ok(ValidatedUrl {
        inner: parsed,
        host,
        private_allowed: matches!(literal_private, Some(true)),
    })
}

/// Re-validates a redirect target (called by the transport's redirect
/// policy). Same rules as [`validate_download_url`].
pub fn validate_redirect_target(raw: &str, allow_private: bool) -> Result<(), TideError> {
    validate_download_url(raw, allow_private).map(|_| ())
}

// ---- DownloadFailure: the M1 domain error payload ----
//
// Wrapped in TideError::Download (see error.rs) so the M0 error model
// stays the single boundary (M1 §45).

#[derive(Debug, thiserror::Error)]
pub enum DownloadFailure {
    #[error("invalid URL: {reason}")]
    InvalidUrl { reason: String },
    #[error("unsupported URL scheme: {scheme}")]
    UnsupportedScheme { scheme: String },
    #[error("redirect limit exceeded")]
    RedirectLimitExceeded,
    #[error("private address blocked: {host}")]
    PrivateAddressBlocked { host: String },
    #[error("HTTP status {status}")]
    HttpStatus { status: u16 },
    #[error("network error")]
    Network,
    #[error("network timeout")]
    Timeout,
    #[error("download cancelled")]
    Cancelled,
    #[error("disk full")]
    DiskFull,
    #[error("permission denied")]
    PermissionDenied,
    #[error("file write failed")]
    FileWrite,
    #[error("resume not supported by server")]
    ResumeNotSupported,
    #[error("remote content changed")]
    RemoteChanged,
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("invalid Content-Range")]
    InvalidContentRange,
    #[error("destination already exists: {path}")]
    DestinationExists { path: String },
    #[error("invalid torrent path: {path}")]
    InvalidTorrentPath { path: String },
    #[error("insufficient disk space (need {required_bytes} bytes)")]
    InsufficientDiskSpace { required_bytes: u64 },
}

impl DownloadFailure {
    /// Stable machine-readable error code (M1 §60 error matrix).
    pub fn code(&self) -> &'static str {
        match self {
            DownloadFailure::InvalidUrl { .. } => "DOWNLOAD_INVALID_URL",
            DownloadFailure::UnsupportedScheme { .. } => "DOWNLOAD_UNSUPPORTED_SCHEME",
            DownloadFailure::RedirectLimitExceeded => "DOWNLOAD_REDIRECT_LIMIT",
            DownloadFailure::PrivateAddressBlocked { .. } => "DOWNLOAD_PRIVATE_ADDRESS_BLOCKED",
            DownloadFailure::HttpStatus { .. } => "DOWNLOAD_HTTP_STATUS",
            DownloadFailure::Network => "DOWNLOAD_NETWORK",
            DownloadFailure::Timeout => "DOWNLOAD_TIMEOUT",
            DownloadFailure::Cancelled => "DOWNLOAD_CANCELLED",
            DownloadFailure::DiskFull => "DOWNLOAD_DISK_FULL",
            DownloadFailure::PermissionDenied => "DOWNLOAD_PERMISSION_DENIED",
            DownloadFailure::FileWrite => "DOWNLOAD_FILE_WRITE",
            DownloadFailure::ResumeNotSupported => "DOWNLOAD_RESUME_NOT_SUPPORTED",
            DownloadFailure::RemoteChanged => "DOWNLOAD_REMOTE_CHANGED",
            DownloadFailure::ChecksumMismatch => "DOWNLOAD_CHECKSUM_MISMATCH",
            DownloadFailure::InvalidContentRange => "DOWNLOAD_INVALID_CONTENT_RANGE",
            DownloadFailure::DestinationExists { .. } => "DOWNLOAD_DESTINATION_EXISTS",
            DownloadFailure::InvalidTorrentPath { .. } => "DOWNLOAD_INVALID_TORRENT_PATH",
            DownloadFailure::InsufficientDiskSpace { .. } => "DOWNLOAD_DISK_SPACE",
        }
    }

    /// Whether this error is transient (retry policy may re-attempt).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            DownloadFailure::Network
                | DownloadFailure::Timeout
                | DownloadFailure::HttpStatus {
                    status: 408 | 429 | 500..=599
                }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_public_http_and_https() {
        for url in [
            "http://example.com/file.bin",
            "https://example.com/file.bin",
            "https://example.com:8443/path/file.zip?q=1",
        ] {
            let v = validate_download_url(url, false).expect(url);
            assert_eq!(v.host, "example.com");
        }
    }

    #[test]
    fn rejects_other_schemes() {
        for url in [
            "ftp://example.com/f",
            "file:///C:/Windows/system32",
            "javascript:alert(1)",
            "data:text/plain,hello",
            "blob:http://localhost/x",
            "ws://example.com/",
            "wss://example.com/",
            "myapp://download",
        ] {
            let err = validate_download_url(url, false).unwrap_err();
            assert!(matches!(
                err,
                TideError::Download(DownloadFailure::UnsupportedScheme { .. })
                    | TideError::Download(DownloadFailure::InvalidUrl { .. })
            ));
        }
    }

    #[test]
    fn rejects_malformed_and_credential_urls() {
        assert!(validate_download_url("", false).is_err());
        assert!(validate_download_url("not a url", false).is_err());
        assert!(validate_download_url("http://", false).is_err());
        assert!(validate_download_url("http://user:pass@example.com/f", false).is_err());
        // scheme check is structural: "http://" inside a path does not pass
        assert!(validate_download_url("ftp://example.com/http://x", false).is_err());
    }

    #[test]
    fn blocks_private_literals_by_default() {
        for url in [
            "http://127.0.0.1/file",
            "http://10.0.0.5/file",
            "http://192.168.1.10/file",
            "http://172.16.0.1/file",
            "http://169.254.1.1/file",
            "http://[::1]/file",
            "http://[fe80::1]/file",
            "http://[fc00::1]/file",
            "http://0.0.0.0/file",
            "http://100.64.0.1/file", // CGNAT
            "http://192.0.2.1/file",  // TEST-NET
        ] {
            let err = validate_download_url(url, false).unwrap_err();
            assert!(matches!(
                err,
                TideError::Download(DownloadFailure::PrivateAddressBlocked { .. })
            ));
        }
    }

    #[test]
    fn allows_private_literals_when_opted_in() {
        let v = validate_download_url("http://127.0.0.1:9911/file", true).unwrap();
        assert!(v.private_allowed);
        assert_eq!(v.host, "127.0.0.1");
    }

    #[test]
    fn public_ips_and_hostnames_pass_without_dns() {
        assert!(validate_download_url("http://8.8.8.8/f", false).is_ok());
        assert!(validate_download_url("http://example.internal/f", false).is_ok());
    }

    #[test]
    fn redirect_targets_go_through_the_same_rules() {
        assert!(validate_redirect_target("https://example.com/b", false).is_ok());
        assert!(validate_redirect_target("file:///etc/passwd", false).is_err());
        assert!(validate_redirect_target("http://127.0.0.1/x", false).is_err());
    }

    #[test]
    fn retryable_classification() {
        assert!(DownloadFailure::Timeout.is_retryable());
        assert!(DownloadFailure::Network.is_retryable());
        assert!(DownloadFailure::HttpStatus { status: 503 }.is_retryable());
        assert!(DownloadFailure::HttpStatus { status: 429 }.is_retryable());
        assert!(!DownloadFailure::HttpStatus { status: 404 }.is_retryable());
        assert!(!DownloadFailure::ChecksumMismatch.is_retryable());
        assert!(!DownloadFailure::PrivateAddressBlocked { host: "x".into() }.is_retryable());
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(DownloadFailure::Timeout.code(), "DOWNLOAD_TIMEOUT");
        assert_eq!(
            DownloadFailure::HttpStatus { status: 404 }.code(),
            "DOWNLOAD_HTTP_STATUS"
        );
        assert_eq!(
            DownloadFailure::ChecksumMismatch.code(),
            "DOWNLOAD_CHECKSUM_MISMATCH"
        );
    }
}
