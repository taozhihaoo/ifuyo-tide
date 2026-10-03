// M0.5 SPIKE — Tide Progressive Stream Gateway prototype (experiments G/H).
//
// Security model under test (must hold for M4):
//   * loopback-only bind on a RANDOM port — never 0.0.0.0
//   * /stream/{high-entropy-token}; tokens are UUID v4, unguessable,
//     expirable, revocable — never enumerable ids, never paths
//   * Host header must be 127.0.0.1 / localhost / [::1]
//   * GET/HEAD with single-range support: 200/206/416/400, streaming in
//     bounded chunks; reads WAIT for torrent availability
//   * every request is measured (experiment B/G data)
//
// Concurrency: the session map is a std Mutex touched only for O(1)
// lookup/mutation; each session's resource sits behind its own tokio
// Mutex so one slow viewer never blocks other sessions.
//
// Spike quality: hand-rolled HTTP/1.0-ish, one request per connection,
// Connection: close. The production M4 gateway will use a real HTTP
// stack — but must keep exactly these boundaries.

use super::events;
use super::range::{self, RangeResolution};
use super::resource::SpikeResource;
use crate::core::resource::ByteRange;
use crate::error::TideError;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const STREAM_CHUNK: u64 = 256 * 1024;

struct SessionEntry {
    resource: SpikeResource,
    name: String,
    created_at: Instant,
    ttl: Option<Duration>,
}

pub struct Gateway {
    pub port: u16,
    sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>>,
    slow_delay_ms: Arc<AtomicU32>,
    read_timeout_ms: Arc<AtomicU64>,
    done: tokio::sync::watch::Sender<bool>,
}

/// Host header must name the loopback interface (13.3).
fn valid_host(host: &str) -> bool {
    let authority = match host.rsplit_once(':') {
        // careful with [::1]:8080 — rsplit_once on the LAST colon
        Some((h, port)) if !h.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    };
    let authority = authority.trim_matches(['[', ']']);
    matches!(authority, "127.0.0.1" | "localhost" | "::1")
}

impl Gateway {
    /// Binds on a random loopback port (experiment G: loopback only).
    pub async fn start() -> Result<Gateway, TideError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| TideError::Internal("gateway bind failed".into()))?;
        let port = listener
            .local_addr()
            .map_err(|_| TideError::Internal("gateway local addr failed".into()))?
            .port();
        let (done, mut done_rx) = tokio::sync::watch::channel(false);

        let sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let slow_delay_ms = Arc::new(AtomicU32::new(0));
        let read_timeout_ms = Arc::new(AtomicU64::new(120_000));

        let gateway = Gateway {
            port,
            sessions: sessions.clone(),
            slow_delay_ms: slow_delay_ms.clone(),
            read_timeout_ms: read_timeout_ms.clone(),
            done,
        };

        crate::diagnostics::info("spike.gateway.started", &[("port", &port.to_string())]);

        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = done_rx.changed() => break,
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _peer)) => {
                                let sessions = sessions.clone();
                                let slow = slow_delay_ms.clone();
                                let timeout = read_timeout_ms.clone();
                                tauri::async_runtime::spawn(async move {
                                    let _ = serve(stream, sessions, slow, timeout).await;
                                });
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
            crate::diagnostics::info("spike.gateway.stopped", &[]);
        });

        Ok(gateway)
    }

    /// Creates a playback session bound to one resource. The token is a
    /// UUID v4 (122 random bits) — the only way to address bytes.
    pub fn create_session(
        &self,
        resource: SpikeResource,
        name: String,
        ttl: Option<Duration>,
    ) -> Result<String, TideError> {
        let token = crate::core::ids::new_id();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| TideError::Internal("gateway sessions poisoned".into()))?;
        sessions.insert(
            token.clone(),
            Arc::new(SessionEntry {
                resource,
                name,
                created_at: Instant::now(),
                ttl,
            }),
        );
        Ok(token)
    }

    pub fn remove_session(&self, token: &str) -> bool {
        self.sessions
            .lock()
            .map(|mut s| s.remove(token).is_some())
            .unwrap_or(false)
    }

    pub fn session_count(&self) -> usize {
        self.sessions.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn session_url(&self, token: &str) -> String {
        format!("http://127.0.0.1:{}/stream/{token}", self.port)
    }

    pub fn set_slow_delay_ms(&self, ms: u32) {
        self.slow_delay_ms.store(ms, Ordering::Relaxed);
        events::record("gateway.slow_mode", &[("delay_ms", ms.to_string())]);
    }

    /// Read timeout for unavailable torrent data (experiment K: lower
    /// this to observe stall handling).
    pub fn set_read_timeout_ms(&self, ms: u64) {
        self.read_timeout_ms.store(ms, Ordering::Relaxed);
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.done.send(true);
    }
}

struct ParsedRequest {
    method: String,
    path: String,
    host: Option<String>,
    range: Option<String>,
}

async fn read_request(stream: &mut TcpStream) -> Result<ParsedRequest, TideError> {
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 256];
    loop {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|_| TideError::InvalidInput("read failed".into()))?;
        if n == 0 {
            return Err(TideError::InvalidInput("connection closed early".into()));
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_HEADER_BYTES {
            return Err(TideError::Security("request headers too large".into()));
        }
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_uppercase();
    let path = parts.next().unwrap_or_default().to_string();
    let mut host = None;
    let mut range = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            match name.as_str() {
                "host" => host = Some(value),
                "range" => range = Some(value),
                _ => {}
            }
        }
    }
    Ok(ParsedRequest {
        method,
        path,
        host,
        range,
    })
}

async fn serve(
    mut stream: TcpStream,
    sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>>,
    slow_delay_ms: Arc<AtomicU32>,
    read_timeout_ms: Arc<AtomicU64>,
) {
    let started = Instant::now();
    let request = match read_request(&mut stream).await {
        Ok(request) => request,
        Err(_) => return,
    };

    // --- Host validation (13.3) ---
    let host_ok = request.host.as_deref().map(valid_host).unwrap_or(false);
    if !host_ok {
        events::record(
            "gateway.rejected",
            &[
                ("reason", "host".to_string()),
                ("path", request.path.clone()),
            ],
        );
        let _ = write_simple(&mut stream, "403 Forbidden", &[]).await;
        return;
    }

    // --- Path validation (13.1): only /stream/{token} exists ---
    let token = match request.path.strip_prefix("/stream/") {
        Some(token)
            if !token.is_empty()
                && !token.contains('/')
                && !token.contains('\\')
                && !token.contains("..")
                && !token.contains(':')
                && !token.contains('%') =>
        {
            token.to_string()
        }
        _ => {
            events::record(
                "gateway.rejected",
                &[
                    ("reason", "path".to_string()),
                    ("path", request.path.clone()),
                ],
            );
            let _ = write_simple(&mut stream, "404 Not Found", &[]).await;
            return;
        }
    };

    // --- Session lookup + expiry (map lock held only here) ---
    // A poisoned map lock is recovered from: entries are plain data and
    // the spike map has no invariant worth crashing over. The guard is
    // confined to this block so it can never be held across an await.
    let entry: Option<Arc<SessionEntry>> = {
        let mut map = match sessions.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let expired = map
            .get(&token)
            .is_some_and(|e| e.ttl.is_some_and(|ttl| e.created_at.elapsed() > ttl));
        if expired {
            map.remove(&token);
        }
        map.get(&token).cloned()
    };
    let Some(entry) = entry else {
        events::record(
            "gateway.rejected",
            &[
                ("reason", "session".to_string()),
                ("token_prefix", short_token(&token)),
            ],
        );
        let _ = write_simple(&mut stream, "404 Not Found", &[]).await;
        return;
    };

    // Clone the resource for this request: WebView2 issues CONCURRENT
    // range requests (head stream + moov fetch), so a request must never
    // serialize behind another one on the same session.
    let mut resource = entry.resource.clone();
    let total_len = resource.len();

    events::record(
        "gateway.request",
        &[
            ("token_prefix", short_token(&token)),
            ("source", resource.source_label().to_string()),
            ("name", entry.name.clone()),
            ("method", request.method.clone()),
            ("range", request.range.clone().unwrap_or_default()),
        ],
    );

    let head_only = request.method == "HEAD";
    let resolution = range::resolve_range_header(request.range.as_deref(), total_len);

    let (status, content_range_header, (start, end)) = match resolution {
        RangeResolution::Full => ("200 OK", None, (0, total_len)),
        RangeResolution::Satisfied(resolved) => (
            "206 Partial Content",
            Some(range::content_range(&resolved, total_len)),
            (resolved.start, resolved.end),
        ),
        RangeResolution::NotSatisfiable => {
            let _ = write_simple(
                &mut stream,
                "416 Range Not Satisfiable",
                &[("Content-Range", format!("bytes */{total_len}"))],
            )
            .await;
            record_served(&token, "416", 0, started);
            return;
        }
        RangeResolution::Invalid => {
            let _ = write_simple(&mut stream, "400 Bad Request", &[]).await;
            record_served(&token, "400", 0, started);
            return;
        }
    };

    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n",
        resource.content_type()
    );
    if let Some(content_range) = content_range_header {
        response.push_str(&format!("Content-Range: {content_range}\r\n"));
    }
    response.push_str(&format!("Content-Length: {}\r\n\r\n", end - start));
    if stream.write_all(response.as_bytes()).await.is_err() {
        return;
    }

    let mut sent: u64 = 0;
    if !head_only {
        let read_timeout = Duration::from_millis(read_timeout_ms.load(Ordering::Relaxed));
        let mut offset = start;
        while offset < end {
            let take = (end - offset).min(STREAM_CHUNK);
            let Ok(chunk_range) = ByteRange::new(offset, offset + take) else {
                break;
            };
            let read_started = Instant::now();
            // The gateway enforces the per-chunk availability wait itself
            // (experiment K): lower this to watch WebView2 stall handling.
            let read = tokio::time::timeout(read_timeout, resource.read_range(chunk_range));
            match read.await {
                Ok(Ok(bytes)) => {
                    events::record(
                        "gateway.read",
                        &[
                            ("token_prefix", short_token(&token)),
                            ("start", offset.to_string()),
                            ("bytes", bytes.len().to_string()),
                            ("wait_ms", read_started.elapsed().as_millis().to_string()),
                        ],
                    );
                    if stream.write_all(&bytes).await.is_err() {
                        events::record(
                            "gateway.client_disconnected",
                            &[
                                ("token_prefix", short_token(&token)),
                                ("sent", sent.to_string()),
                            ],
                        );
                        return;
                    }
                    sent += bytes.len() as u64;
                    offset += take;
                    let delay = slow_delay_ms.load(Ordering::Relaxed);
                    if delay > 0 && offset < end {
                        tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                    }
                }
                Ok(Err(err)) => {
                    // Mid-stream failure (e.g. missing pieces): status is
                    // already on the wire, so abort the connection.
                    events::record(
                        "gateway.read_failed",
                        &[
                            ("token_prefix", short_token(&token)),
                            ("start", offset.to_string()),
                            ("error_code", err.code().to_string()),
                            ("wait_ms", read_started.elapsed().as_millis().to_string()),
                        ],
                    );
                    break;
                }
                Err(_elapsed) => {
                    events::record(
                        "gateway.read_timeout",
                        &[
                            ("token_prefix", short_token(&token)),
                            ("start", offset.to_string()),
                            ("wait_ms", read_started.elapsed().as_millis().to_string()),
                        ],
                    );
                    break;
                }
            }
        }
    }
    let _ = stream.flush().await;
    record_served(&token, status, sent, started);
}

fn record_served(token: &str, status: &str, sent: u64, started: Instant) {
    events::record(
        "gateway.served",
        &[
            ("token_prefix", short_token(token)),
            ("status", status.to_string()),
            ("bytes", sent.to_string()),
            ("elapsed_ms", started.elapsed().as_millis().to_string()),
        ],
    );
}

fn short_token(token: &str) -> String {
    token.chars().take(8).collect()
}

async fn write_simple(
    stream: &mut TcpStream,
    status: &str,
    headers: &[(&str, String)],
) -> Result<(), TideError> {
    let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("Content-Length: 0\r\n\r\n");
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|_| TideError::Internal("write failed".into()))?;
    stream
        .flush()
        .await
        .map_err(|_| TideError::Internal("flush failed".into()))
}

#[cfg(test)]
mod tests {
    use super::valid_host;

    #[test]
    fn host_validation_accepts_loopback_only() {
        assert!(valid_host("127.0.0.1:12345"));
        assert!(valid_host("localhost"));
        assert!(valid_host("localhost:8080"));
        assert!(valid_host("[::1]:9000"));
    }

    #[test]
    fn host_validation_rejects_other_names() {
        assert!(!valid_host("malicious-host"));
        assert!(!valid_host("evil.example.com:80"));
        assert!(!valid_host("192.168.1.10:8080"));
        assert!(!valid_host("0.0.0.0"));
    }
}
