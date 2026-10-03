// M0.5 SPIKE — minimal local HTTP file server for experiments A/B.
//
// Serves ONLY the generated media fixture directory, loopback only, with
// Range support, streaming in bounded chunks (never read_to_end). This
// is throwaway experiment infrastructure — the real production
// component is the M4 gateway. The value here is that it logs every
// request (Range header, status, bytes, latency) so experiment B can
// record exactly what WebView2 asks for.

use super::{events, fixtures, range};
use crate::error::TideError;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const CHUNK_SIZE: u64 = 64 * 1024;

pub struct FixtureServer {
    pub port: u16,
    /// Artificial per-chunk delay (ms) simulating a slow network
    /// (experiment K). 0 = full speed.
    slow_delay_ms: Arc<AtomicU32>,
    done: tokio::sync::watch::Sender<bool>,
}

impl FixtureServer {
    pub fn set_slow_delay_ms(&self, ms: u32) {
        self.slow_delay_ms.store(ms, Ordering::Relaxed);
        events::record("fixture.slow_mode", &[("delay_ms", ms.to_string())]);
    }
}

/// Binds a fixture server on a random loopback port and starts serving.
pub async fn start() -> Result<FixtureServer, TideError> {
    let media_dir = fixtures::media_dir()?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| TideError::Internal("failed to bind fixture server".into()))?;
    let port = listener
        .local_addr()
        .map_err(|_| TideError::Internal("fixture server has no local addr".into()))?
        .port();
    let (done, mut done_rx) = tokio::sync::watch::channel(false);

    let slow_delay_ms = Arc::new(AtomicU32::new(0));
    let server = FixtureServer {
        port,
        slow_delay_ms: slow_delay_ms.clone(),
        done,
    };

    crate::diagnostics::info(
        "spike.fixture_server.started",
        &[("port", &port.to_string())],
    );

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = done_rx.changed() => break,
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _peer)) => {
                            let media_dir = media_dir.clone();
                            let slow = slow_delay_ms.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = serve_connection(stream, media_dir, slow).await;
                            });
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        crate::diagnostics::info("spike.fixture_server.stopped", &[]);
    });

    Ok(server)
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        let _ = self.done.send(true);
    }
}

struct Request {
    method: String,
    path: String,
    range: Option<String>,
}

async fn read_request(stream: &mut TcpStream) -> Result<Request, TideError> {
    let mut buf = Vec::with_capacity(1024);
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
            return Err(TideError::InvalidInput("request headers too large".into()));
        }
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut range_header = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("range") {
                range_header = Some(value.trim().to_string());
            }
        }
    }
    Ok(Request {
        method,
        path,
        range: range_header,
    })
}

/// What to send after the header block.
enum BodyPlan {
    Empty,
    Full(std::fs::File),
    Range(std::fs::File, range::ResolvedRange),
}

async fn serve_connection(
    mut stream: TcpStream,
    media_dir: PathBuf,
    slow_delay_ms: Arc<AtomicU32>,
) {
    let started = std::time::Instant::now();
    let request = match read_request(&mut stream).await {
        Ok(request) => request,
        Err(_) => return,
    };

    let (status, mut headers, body) = match route(&request, &media_dir) {
        Ok(response) => response,
        Err(err) => error_response(&err),
    };

    events::record(
        "fixture.request",
        &[
            ("method", request.method.clone()),
            ("path", request.path.clone()),
            ("range", request.range.clone().unwrap_or_default()),
            ("status", status.to_string()),
        ],
    );

    let head_only = request.method == "HEAD";
    let mut response =
        format!("HTTP/1.1 {status}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n");
    let content_length = match &body {
        BodyPlan::Empty => 0,
        BodyPlan::Full(file) => file.metadata().map(|m| m.len()).unwrap_or(0),
        BodyPlan::Range(_, resolved) => resolved.len(),
    };
    headers.push(("Content-Length".to_string(), content_length.to_string()));
    for (name, value) in &headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    if stream.write_all(response.as_bytes()).await.is_err() {
        return;
    }

    let mut sent: u64 = 0;
    if !head_only {
        match body {
            BodyPlan::Empty => {}
            BodyPlan::Full(mut file) => loop {
                let mut chunk = vec![0u8; CHUNK_SIZE as usize];
                match std::io::Read::read(&mut file, &mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if stream.write_all(&chunk[..n]).await.is_err() {
                            break;
                        }
                        sent += n as u64;
                        apply_slow(&slow_delay_ms).await;
                    }
                    Err(_) => break,
                }
            },
            BodyPlan::Range(mut file, resolved) => {
                if std::io::Seek::seek(&mut file, SeekFrom::Start(resolved.start)).is_ok() {
                    let mut remaining = resolved.len();
                    while remaining > 0 {
                        let take = remaining.min(CHUNK_SIZE) as usize;
                        let mut chunk = vec![0u8; take];
                        match std::io::Read::read(&mut file, &mut chunk) {
                            Ok(0) => break,
                            Ok(n) => {
                                if stream.write_all(&chunk[..n]).await.is_err() {
                                    break;
                                }
                                remaining -= n as u64;
                                sent += n as u64;
                                apply_slow(&slow_delay_ms).await;
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
    }
    let _ = stream.flush().await;
    events::record(
        "fixture.served",
        &[
            ("path", request.path.clone()),
            ("bytes", sent.to_string()),
            ("elapsed_ms", started.elapsed().as_millis().to_string()),
        ],
    );
}

async fn apply_slow(slow_delay_ms: &Arc<AtomicU32>) {
    let delay = slow_delay_ms.load(Ordering::Relaxed);
    if delay > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
    }
}

fn route(
    request: &Request,
    media_dir: &PathBuf,
) -> Result<(&'static str, Vec<(String, String)>, BodyPlan), TideError> {
    if request.method != "GET" && request.method != "HEAD" {
        return Ok(("405 Method Not Allowed", vec![], BodyPlan::Empty));
    }
    let name = request
        .path
        .strip_prefix("/media/")
        .ok_or_else(|| TideError::NotFound("unknown fixture path".into()))?
        .split('?')
        .next()
        .unwrap_or_default();
    let path = fixtures::resolve_media_file(name)?;
    // Defense in depth: canonical containment. The fixture server never
    // serves anything outside the media dir, whatever the URL says.
    let canonical = path
        .canonicalize()
        .map_err(|_| TideError::NotFound("fixture vanished".into()))?;
    let dir_canonical = media_dir
        .canonicalize()
        .map_err(|_| TideError::Internal("media dir vanished".into()))?;
    if !canonical.starts_with(&dir_canonical) {
        return Err(TideError::Security("path escapes media directory".into()));
    }

    let file = std::fs::File::open(&canonical)
        .map_err(|_| TideError::NotFound("fixture unreadable".into()))?;
    let total_len = file
        .metadata()
        .map_err(|_| TideError::NotFound("stat failed".into()))?
        .len();
    let content_type = fixtures::content_type_for(&canonical).to_string();

    match range::resolve_range_header(request.range.as_deref(), total_len) {
        range::RangeResolution::Full => Ok((
            "200 OK",
            vec![("Content-Type".to_string(), content_type)],
            BodyPlan::Full(file),
        )),
        range::RangeResolution::Satisfied(resolved) => Ok((
            "206 Partial Content",
            vec![
                ("Content-Type".to_string(), content_type),
                (
                    "Content-Range".to_string(),
                    range::content_range(&resolved, total_len),
                ),
            ],
            BodyPlan::Range(file, resolved),
        )),
        range::RangeResolution::NotSatisfiable => Ok((
            "416 Range Not Satisfiable",
            vec![("Content-Range".to_string(), format!("bytes */{total_len}"))],
            BodyPlan::Empty,
        )),
        range::RangeResolution::Invalid => Ok(("400 Bad Request", vec![], BodyPlan::Empty)),
    }
}

fn error_response(err: &TideError) -> (&'static str, Vec<(String, String)>, BodyPlan) {
    match err.code() {
        "NOT_FOUND" => ("404 Not Found", vec![], BodyPlan::Empty),
        "SECURITY" => ("403 Forbidden", vec![], BodyPlan::Empty),
        "INVALID_INPUT" => ("400 Bad Request", vec![], BodyPlan::Empty),
        _ => ("500 Internal Server Error", vec![], BodyPlan::Empty),
    }
}
