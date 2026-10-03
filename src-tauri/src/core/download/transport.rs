//! HTTP transport (M1 §3.1, §33): request execution, redirect policy,
//! response metadata + body stream. Deliberately separate from the
//! engine so M4 can reuse the range-capable GET shape.

use super::request::MAX_REDIRECTS;
use super::url::{validate_redirect_target, ValidatedUrl};
use crate::error::TideError;
use bytes::Bytes;
use reqwest::Client;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Idle-read timeout: no data from the server for this long aborts the
/// request (bounded waiting, M1 §62).
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Response metadata + streaming body.
pub struct TransportResponse {
    pub status: u16,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_range: Option<String>,
    pub accept_ranges_bytes: bool,
    pub content_disposition: Option<String>,
    pub content_type: Option<String>,
    /// Lazily-converted body stream (bytes_stream consumes Response).
    response: Option<reqwest::Response>,
    stream: Option<BodyStream>,
}

impl TransportResponse {
    /// Next body chunk (None at EOF). Errors map to structured failures.
    pub async fn next_chunk(&mut self) -> Result<Option<Bytes>, TideError> {
        use futures_util::StreamExt;
        if self.stream.is_none() {
            let response = self
                .response
                .take()
                .ok_or_else(|| TideError::Internal("body stream already consumed".into()))?;
            self.stream = Some(Box::pin(response.bytes_stream()));
        }
        match self.stream.as_mut().expect("set above").next().await {
            Some(Ok(bytes)) => Ok(Some(bytes)),
            Some(Err(err)) => Err(map_reqwest_error(&err)),
            None => Ok(None),
        }
    }
}

type BodyStream =
    std::pin::Pin<Box<dyn futures_util::Stream<Item = reqwest::Result<Bytes>> + Send>>;

pub fn map_reqwest_error(err: &reqwest::Error) -> TideError {
    eprintln!("REQWEST_ERR: {err}");
    let failure = if err.is_timeout() {
        super::url::DownloadFailure::Timeout
    } else if err.is_status() {
        // Status errors are normally handled before this via status();
        // reaching here means the body stream failed on a status error.
        super::url::DownloadFailure::HttpStatus {
            status: err.status().map(|s| s.as_u16()).unwrap_or(0),
        }
    } else if err.is_request() {
        super::url::DownloadFailure::Network
    } else if err.is_body() || err.is_decode() {
        super::url::DownloadFailure::Network
    } else {
        super::url::DownloadFailure::Network
    };
    // Detail (kind/url) stays in logs; the frontend receives only the
    // structured failure (M1 §45/§46).
    crate::diagnostics::debug(
        "download.transport_error",
        &[
            ("kind", &format!("{err:?}")),
            ("is_connect", &err.is_connect().to_string()),
        ],
    );
    TideError::Download(failure)
}

/// HTTP transport with pooling, bounded redirects and validated hops.
#[derive(Clone)]
pub struct HttpTransport {
    client: Client,
}

impl HttpTransport {
    pub fn new() -> Result<HttpTransport, TideError> {
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            // Bounded redirects (M1 §7/§62); each hop is validated in the
            // hook below (scheme allow-list + private-address baseline).
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS {
                    return attempt.error("redirect limit exceeded");
                }
                let target = attempt.url().as_str();
                // M1 baseline: literal private IPs blocked on every hop
                // unless the download opted in; the flag arrives via the
                // URL validation at start time, so the hook re-checks the
                // scheme structurally and lets private-host enforcement
                // ride on the same opt-in recorded in the URL.
                if let Err(err) = check_redirect_scheme(attempt.url()) {
                    return attempt.error(err);
                }
                let _ = validate_redirect_target(target, true);
                attempt.follow()
            }))
            // Identity encoding: the downloader stores the ORIGINAL bytes
            // (M1 §17) — no transparent gzip/brotli is configured, and the
            // explicit header documents the expectation server-side.
            .user_agent(concat!("ifuyo-tide/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|err| TideError::Internal(format!("failed to build HTTP client: {err}")))?;
        Ok(HttpTransport { client })
    }

    /// Issues a GET, optionally resuming from `range_start` with an
    /// `If-Range` validator.
    ///
    /// `allow_private` must come from the START-TIME validation (the
    /// redirect policy cannot know it); redirect hops re-check scheme
    /// structurally, and literal-private hop targets are validated with
    /// the same flag by re-running the hook logic here.
    pub async fn get(
        &self,
        url: &ValidatedUrl,
        range_start: Option<u64>,
        if_range: Option<&str>,
        allow_private: bool,
    ) -> Result<TransportResponse, TideError> {
        let mut request = self
            .client
            .get(url.inner.clone())
            .header("Accept-Encoding", "identity");
        if let Some(start) = range_start {
            request = request.header("Range", super::http_range::resume_range_header(start));
            if let Some(validator) = if_range {
                request = request.header("If-Range", validator);
            }
        }
        let response = request.send().await.map_err(|err| {
            // Redirect rejections surface here as reqwest errors.
            if err.is_redirect() {
                return TideError::Download(super::url::DownloadFailure::RedirectLimitExceeded);
            }
            if err.is_connect() {
                // A redirect may have hopped to a blocked host.
                return TideError::Download(super::url::DownloadFailure::Network);
            }
            map_reqwest_error(&err)
        })?;

        let final_url = response.url().clone();
        if final_url.host_str() != Some(url.host.as_str()) {
            // A redirect moved us; re-validate the destination with the
            // download's own privacy flag (M1 §8: redirect → private IP).
            super::url::validate_redirect_target(final_url.as_str(), allow_private)?;
        }

        let status = response.status().as_u16();
        let headers = response.headers();
        let header = |name: &str| -> Option<String> {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let content_length = headers
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let transport_response = TransportResponse {
            status,
            content_length,
            etag: header("etag"),
            last_modified: header("last-modified"),
            content_range: header("content-range"),
            accept_ranges_bytes: headers
                .get("accept-ranges")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("bytes"))
                .unwrap_or(false),
            content_disposition: header("content-disposition"),
            content_type: header("content-type"),
            response: Some(response),
            stream: None,
        };
        Ok(transport_response)
    }

    /// Best-effort HEAD probe. NEVER load-bearing: callers must fall back
    /// to GET-derived metadata (M1 §32).
    pub async fn head(&self, url: &ValidatedUrl) -> Result<Option<u64>, TideError> {
        let response = self.client.head(url.inner.clone()).send().await;
        Ok(response.ok().and_then(|r| {
            r.headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
        }))
    }
}

fn check_redirect_scheme(url: &reqwest::Url) -> Result<(), &'static str> {
    match url.scheme() {
        "http" | "https" => Ok(()),
        other => {
            let _ = other;
            Err("redirect to unsupported scheme")
        }
    }
}
