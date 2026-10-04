//! Range Gateway (M4(上) §37–§50/§71, M4(下) §96/§107/§115–§117):
//! the ONE Tide streaming gateway.
//!
//! Security: loopback-only bind on a random port, per-session UUID
//! tokens (never paths), Host-header loopback validation, GET/HEAD
//! only (405 otherwise), single-range 200/206/416/400, 503 before
//! headers when the resource is gone, bounded streaming chunks, reads
//! WAIT for availability through the resource with a per-read timeout.
//! Gateway reads flow through the DemandScheduler so playback beats
//! background (M4(上) §71). Every session carries a cancellation watch
//! — closing a session aborts its in-flight reads promptly (§116/§137).

use super::demand::{DemandId, DemandScheduler};
use super::resource::StreamingResource;
use crate::core::resource::ByteRange;
use crate::error::TideError;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const STREAM_CHUNK: u64 = 256 * 1024;

struct SessionEntry {
    resource: tokio::sync::Mutex<StreamingResource>,
    created_at: Instant,
    ttl: Option<Duration>,
    /// Closed on session stop/reap: in-flight reads select on this and
    /// abort instead of serving a dead session (M4(下) §116/§137).
    closed: Arc<tokio::sync::watch::Sender<bool>>,
}

impl SessionEntry {
    fn is_closed(&self) -> bool {
        *self.closed.borrow()
    }
}

fn valid_host(host: &str) -> bool {
    let authority = match host.rsplit_once(':') {
        Some((h, port)) if !h.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    };
    matches!(
        authority.trim_matches(['[', ']']),
        "127.0.0.1" | "localhost" | "::1"
    )
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
            .map_err(|_| gateway_failure("read failed"))?;
        if n == 0 {
            return Err(gateway_failure("connection closed early"));
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
            match name.trim().to_ascii_lowercase().as_str() {
                "host" => host = Some(value.trim().to_string()),
                "range" => range = Some(value.trim().to_string()),
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

fn gateway_failure(what: &str) -> TideError {
    TideError::Streaming(crate::error::StreamingFailure::Gateway(what.into()))
}

pub struct StreamingGateway {
    pub port: u16,
    sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>>,
    read_timeout_ms: Arc<AtomicU64>,
    done: tokio::sync::watch::Sender<bool>,
}

impl StreamingGateway {
    /// Binds on a random loopback port (M4(上) §38/§39).
    pub async fn start(scheduler: Arc<DemandScheduler>) -> Result<StreamingGateway, TideError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| gateway_failure("bind failed"))?;
        let port = listener
            .local_addr()
            .map_err(|_| gateway_failure("local addr failed"))?
            .port();
        let (done, mut done_rx) = tokio::sync::watch::channel(false);
        let sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let read_timeout_ms = Arc::new(AtomicU64::new(120_000));
        let gateway = StreamingGateway {
            port,
            sessions: sessions.clone(),
            read_timeout_ms: read_timeout_ms.clone(),
            done,
        };
        crate::diagnostics::info("gateway.started", &[("port", &port.to_string())]);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = done_rx.changed() => break,
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _)) => {
                                let sessions = sessions.clone();
                                let sched = scheduler.clone();
                                let timeout = read_timeout_ms.clone();
                                tauri::async_runtime::spawn(async move {
                                    let _ = serve(stream, sched, sessions, timeout).await;
                                });
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
            crate::diagnostics::info("gateway.stopped", &[]);
        });
        Ok(gateway)
    }

    /// Registers a playback session: UUIDv4 token (never paths).
    pub fn create_session(
        &self,
        resource: StreamingResource,
        ttl: Option<Duration>,
    ) -> Result<String, TideError> {
        let token = crate::core::ids::new_id();
        let (closed, _) = tokio::sync::watch::channel(false);
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                token.clone(),
                Arc::new(SessionEntry {
                    resource: tokio::sync::Mutex::new(resource),
                    created_at: Instant::now(),
                    ttl,
                    closed: Arc::new(closed),
                }),
            );
        Ok(token)
    }

    /// Revokes the session and signals its in-flight reads to abort.
    pub fn remove_session(&self, token: &str) -> bool {
        let entry = self
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(token);
        match entry {
            Some(entry) => {
                let _ = entry.closed.send(true);
                true
            }
            None => false,
        }
    }

    pub fn session_url(&self, token: &str) -> String {
        format!("http://127.0.0.1:{}/stream/{token}", self.port)
    }

    pub fn set_read_timeout_ms(&self, ms: u64) {
        self.read_timeout_ms.store(ms, Ordering::Relaxed);
    }

    pub fn session_count(&self) -> usize {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Bytes durably available behind a session right now (M4(下)
    /// §149: backend availability as opposed to browser buffering).
    /// None for unknown/expired tokens.
    pub async fn session_available_bytes(&self, token: &str) -> Option<u64> {
        let entry = {
            let sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
            sessions.get(token).cloned()?
        };
        let resource = entry.resource.lock().await;
        Some(resource.available_len())
    }
}

impl Drop for StreamingGateway {
    fn drop(&mut self) {
        let _ = self.done.send(true);
    }
}

async fn serve(
    mut stream: TcpStream,
    scheduler: Arc<DemandScheduler>,
    sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>>,
    read_timeout_ms: Arc<AtomicU64>,
) {
    let started = Instant::now();
    let request = match read_request(&mut stream).await {
        Ok(request) => request,
        Err(_) => return,
    };

    let host_ok = request.host.as_deref().map(valid_host).unwrap_or(false);
    if !host_ok {
        let _ = write_simple(&mut stream, "403 Forbidden", &[]).await;
        gateway_log("403", &request.path, None, started);
        return;
    }
    let Some(token) = request
        .path
        .strip_prefix("/stream/")
        .filter(|t| !t.is_empty() && !t.contains(['/', '\\', ':', '%']) && !t.contains(".."))
        .map(str::to_string)
    else {
        let _ = write_simple(&mut stream, "404 Not Found", &[]).await;
        gateway_log("404", &request.path, None, started);
        return;
    };

    let entry = {
        let mut sessions = match sessions.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        let expired = sessions
            .get(&token)
            .is_some_and(|e| e.ttl.is_some_and(|ttl| e.created_at.elapsed() > ttl));
        if expired {
            sessions.remove(&token);
        }
        sessions.get(&token).cloned()
    };
    let Some(entry) = entry else {
        let _ = write_simple(&mut stream, "404 Not Found", &[]).await;
        gateway_log("404", &request.path, None, started);
        return;
    };
    if entry.is_closed() {
        let _ = write_simple(&mut stream, "404 Not Found", &[]).await;
        gateway_log("404", &request.path, None, started);
        return;
    }

    // GET and HEAD only (M4(下) §125 "wrong HTTP method").
    if request.method != "GET" && request.method != "HEAD" {
        let _ = write_simple(
            &mut stream,
            "405 Method Not Allowed",
            &[("Allow", "GET, HEAD".to_string())],
        )
        .await;
        gateway_log("405", &request.path, Some(0), started);
        return;
    }

    // Resource gone (task deleted, torrent removed, file missing) →
    // controlled 503 BEFORE any headers are committed (§107).
    let resource = entry.resource.lock().await;
    if !resource.is_available() {
        let _ = write_simple(
            &mut stream,
            "503 Service Unavailable",
            &[("Cache-Control", "no-store".to_string())],
        )
        .await;
        gateway_log("503", &request.path, Some(0), started);
        return;
    }

    let total_len = resource.len();

    let resolution =
        crate::core::streaming::range::resolve_range_header(request.range.as_deref(), total_len);
    use crate::core::streaming::range::RangeResolution;
    let (status, content_range, (start, end)) = match resolution {
        RangeResolution::Full => ("200 OK", None, (0, total_len)),
        RangeResolution::Satisfied(resolved) => (
            "206 Partial Content",
            Some(format!(
                "bytes {}-{}/{}",
                resolved.start,
                resolved.end.saturating_sub(1),
                total_len
            )),
            (resolved.start, resolved.end),
        ),
        RangeResolution::NotSatisfiable => {
            let _ = write_simple(
                &mut stream,
                "416 Range Not Satisfiable",
                &[("Content-Range", format!("bytes */{total_len}"))],
            )
            .await;
            gateway_log("416", &request.path, Some(0), started);
            return;
        }
        RangeResolution::Invalid => {
            let _ = write_simple(&mut stream, "400 Bad Request", &[]).await;
            gateway_log("400", &request.path, Some(0), started);
            return;
        }
    };

    let head_only = request.method == "HEAD";
    let mut response =
        format!("HTTP/1.1 {status}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n");
    if let Some(content_range) = &content_range {
        response.push_str(&format!("Content-Range: {content_range}\r\n"));
    }
    response.push_str(&format!("Content-Length: {}\r\n\r\n", end - start));
    if stream.write_all(response.as_bytes()).await.is_err() {
        return;
    }

    let mut sent: u64 = 0;
    if !head_only {
        let read_timeout = Duration::from_millis(read_timeout_ms.load(Ordering::Relaxed));
        let mut closed_rx = entry.closed.subscribe();
        let mut offset = start;
        while offset < end {
            // Session closed mid-response → stop serving immediately
            // (§116/§137: the dead session must not keep reading).
            if *closed_rx.borrow() {
                break;
            }
            let take = (end - offset).min(STREAM_CHUNK);
            let Ok(chunk_range) = ByteRange::new(offset, offset + take) else {
                break;
            };
            let demand: DemandId = scheduler.submit(
                &token,
                chunk_range,
                crate::core::streaming::demand::DemandReason::PlaybackBuffer,
                None,
            );
            let read = tokio::time::timeout(read_timeout, resource.read_range(chunk_range));
            tokio::pin!(read);
            // Wait for the chunk OR the session-closed signal — a
            // stopped session never waits out the full timeout.
            let result = if *closed_rx.borrow() {
                scheduler.cancel(demand);
                break;
            } else {
                tokio::select! {
                    _ = closed_rx.changed() => {
                        scheduler.cancel(demand);
                        break;
                    }
                    result = &mut read => result,
                }
            };
            match result {
                Ok(Ok(bytes)) => {
                    scheduler.complete(demand);
                    if stream.write_all(&bytes).await.is_err() {
                        scheduler.cancel(demand);
                        return;
                    }
                    sent += bytes.len() as u64;
                    offset += take;
                }
                Ok(Err(_)) | Err(_) => {
                    scheduler.cancel(demand);
                    break;
                }
            }
        }
    }
    let _ = stream.flush().await;
    gateway_log("200", &request.path, Some(sent), started);
}

/// Low-cardinality, redacted request log (M4(下) §185): session short
/// id + status + byte counts only — never the token, path, or URL.
fn gateway_log(status: &str, path: &str, bytes: Option<u64>, started: Instant) {
    let short = path
        .split('/')
        .next_back()
        .unwrap_or("")
        .chars()
        .take(8)
        .collect::<String>();
    crate::diagnostics::info(
        "gateway.request",
        &[
            ("session", &short),
            ("status", status),
            ("bytes", &bytes.map(|b| b.to_string()).unwrap_or_default()),
            ("ms", &started.elapsed().as_millis().to_string()),
        ],
    );
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
        .map_err(|_| gateway_failure("write failed"))?;
    stream
        .flush()
        .await
        .map_err(|_| gateway_failure("flush failed"))
}

#[cfg(test)]
mod tests {
    use super::valid_host;

    #[test]
    fn host_validation_accepts_loopback_only() {
        assert!(valid_host("127.0.0.1:12345"));
        assert!(valid_host("localhost"));
        assert!(valid_host("[::1]:9000"));
        assert!(!valid_host("evil.example.com:80"));
    }
}
