//! Range Gateway (M4(上) §37–§50/§71): the ONE Tide streaming gateway.
//!
//! Security: loopback-only bind on a random port, per-session UUID
//! tokens (never paths), Host header must be loopback, single-range
//! 200/206/416/400, bounded streaming chunks, reads WAIT for torrent
//! availability through the resource. Gateway reads flow through the
//! DemandScheduler so playback beats background (M4(上) §71).

use super::demand::DemandScheduler;
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
            .map_err(|_| TideError::Internal("gateway bind failed".into()))?;
        let port = listener
            .local_addr()
            .map_err(|_| TideError::Internal("gateway local addr failed".into()))?
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
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                token.clone(),
                Arc::new(SessionEntry {
                    resource: tokio::sync::Mutex::new(resource),
                    created_at: Instant::now(),
                    ttl,
                }),
            );
        Ok(token)
    }

    pub fn remove_session(&self, token: &str) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(token)
            .is_some()
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
    let request = match read_request(&mut stream).await {
        Ok(request) => request,
        Err(_) => return,
    };

    let host_ok = request.host.as_deref().map(valid_host).unwrap_or(false);
    if !host_ok {
        let _ = write_simple(&mut stream, "403 Forbidden", &[]).await;
        return;
    }
    let Some(token) = request
        .path
        .strip_prefix("/stream/")
        .filter(|t| !t.is_empty() && !t.contains(['/', '\\', ':', '%']) && !t.contains(".."))
        .map(str::to_string)
    else {
        let _ = write_simple(&mut stream, "404 Not Found", &[]).await;
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
        return;
    };

    let resource = entry.resource.lock().await;
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
            return;
        }
        RangeResolution::Invalid => {
            let _ = write_simple(&mut stream, "400 Bad Request", &[]).await;
            return;
        }
    };

    let head_only = request.method == "HEAD";
    let mut response = format!(
        "HTTP/1.1 {status}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n"
    );
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
        let mut offset = start;
        while offset < end {
            let take = (end - offset).min(STREAM_CHUNK);
            let Ok(chunk_range) = ByteRange::new(offset, offset + take) else {
                break;
            };
            let demand = scheduler.submit(
                &token,
                chunk_range,
                crate::core::streaming::demand::DemandReason::PlaybackBuffer,
                None,
            );
            match tokio::time::timeout(read_timeout, resource.read_range(chunk_range)).await {
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
    let _ = sent;
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
        assert!(valid_host("[::1]:9000"));
        assert!(!valid_host("evil.example.com:80"));
    }
}
