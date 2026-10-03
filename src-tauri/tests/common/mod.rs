//! M1 test HTTP server: a configurable local server for the download
//! engine integration tests (M1 §36/§52). Deterministic payload, fault
//! injection, request logging — no public network involved.
//!
//! One request per connection (Connection: close), hand-rolled HTTP for
//! full control over status/headers/body framing and fault injection.
#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Deterministic pattern: byte i = (i % 251).
pub fn pattern_byte(i: u64) -> u8 {
    (i % 251) as u8
}

/// SHA-256 hex of the patterned payload of `len` bytes.
pub fn pattern_sha256(len: u64) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut offset = 0u64;
    while offset < len {
        let take = (len - offset).min(buf.len() as u64) as usize;
        for (i, b) in buf.iter_mut().enumerate().take(take) {
            *b = pattern_byte(offset + i as u64);
        }
        hasher.update(&buf[..take]);
        offset += take as u64;
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[derive(Default, Clone)]
pub struct ServerState {
    /// Every Range header received (request log for resume assertions).
    pub seen_ranges: Arc<Mutex<Vec<String>>>,
    /// ETags observed on requests that carried If-Range.
    pub seen_if_ranges: Arc<Mutex<Vec<String>>>,
    /// /changing bumps its ETag on every full GET (remote-changed tests).
    pub changing_etag_version: Arc<AtomicU32>,
    /// Every response status line served (redirect-matrix assertions).
    pub seen_statuses: Arc<Mutex<Vec<String>>>,
    /// /unstable resets the first body transmission, then serves fully.
    pub unstable_reset_done: Arc<AtomicU32>,
}

pub struct TestServer {
    pub port: u16,
    pub state: ServerState,
}

impl TestServer {
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    pub fn seen_ranges(&self) -> Vec<String> {
        self.state.seen_ranges.lock().unwrap().clone()
    }

    pub fn seen_if_ranges(&self) -> Vec<String> {
        self.state.seen_if_ranges.lock().unwrap().clone()
    }

    pub fn seen_statuses(&self) -> Vec<String> {
        self.state.seen_statuses.lock().unwrap().clone()
    }
}

/// Spawns the test server on a random loopback port.
pub async fn spawn() -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().unwrap().port();
    let state = ServerState::default();
    let task_state = state.clone();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state = task_state.clone();
                    tokio::spawn(async move {
                        let _ = serve(stream, state).await;
                    });
                }
                Err(_) => break,
            }
        }
    });
    TestServer { port, state }
}

struct Request {
    path: String,
    range: Option<String>,
    if_range: Option<String>,
}

async fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 32 * 1024 {
            return None;
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let request_line = text.lines().next()?;
    let path = request_line.split_whitespace().nth(1)?.to_string();
    let mut range = None;
    let mut if_range = None;
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("range") {
                range = Some(value.trim().to_string());
            } else if name.trim().eq_ignore_ascii_case("if-range") {
                if_range = Some(value.trim().to_string());
            }
        }
        let _ = lower;
    }
    Some(Request {
        path,
        range,
        if_range,
    })
}

async fn serve(mut stream: TcpStream, state: ServerState) {
    let Some(request) = read_request(&mut stream).await else {
        return;
    };
    let Some(response) = route(&request, &state) else {
        return;
    };

    // `headers` already ends with its own CRLF; the Content-Length push
    // below terminates the header block. No extra CRLF here — it would
    // push Content-Length into the body region.
    let mut out = format!("HTTP/1.1 {}\r\n{}", response.status_line, response.headers);
    if !response.omit_length {
        // chunked bodies are delimited by their own framing; a
        // Content-Length would double-delimit and count framing bytes.
        if let Some(body) = &response.body {
            out.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        } else {
            out.push_str(&format!("Content-Length: {}\r\n\r\n", response.body_len));
        }
    } else {
        // No length header: still terminate the header block itself.
        out.push_str("\r\n");
    }
    if stream.write_all(out.as_bytes()).await.is_err() {
        return;
    }
    if let Some(body) = response.body {
        if stream.write_all(&body).await.is_err() {
            return;
        }
    } else {
        // Generated patterned body with optional mid-body reset (fault
        // injection for /unstable: the FIRST body transmission resets).
        let reset_limit = match response.generator {
            Some(inner) => inner,
            None => None,
        };
        let mut offset = response.body_start;
        let end = response.body_start + response.body_len;
        let mut chunk = vec![0u8; 64 * 1024];
        while offset < end {
            let take = (end - offset).min(chunk.len() as u64) as usize;
            for (i, b) in chunk.iter_mut().enumerate().take(take) {
                *b = pattern_byte(offset + i as u64);
            }
            if reset_limit
                .map(|limit| offset - response.body_start >= limit)
                .unwrap_or(false)
                && state
                    .unstable_reset_done
                    .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            {
                let _ = stream.shutdown().await;
                return;
            }
            if response.chunk_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(response.chunk_delay_ms)).await;
            }
            if stream.write_all(&chunk[..take]).await.is_err() {
                return;
            }
            offset += take as u64;
        }
    }
    let _ = stream.flush().await;
}

struct Response {
    status_line: &'static str,
    headers: String,
    body_len: u64,
    body_start: u64,
    body: Option<Vec<u8>>,
    /// Patterned generator + optional mid-body reset point.
    generator: Option<Option<u64>>,
    /// Per-chunk delay ms (slow-route fault injection).
    chunk_delay_ms: u64,
    /// Chunked bodies must not carry Content-Length.
    omit_length: bool,
}

fn simple(status: &'static str) -> Response {
    Response {
        status_line: status,
        headers: String::new(),
        body_len: 0,
        body_start: 0,
        body: Some(Vec::new()),
        generator: None,
        chunk_delay_ms: 0,
        omit_length: false,
    }
}

fn route(request: &Request, state: &ServerState) -> Option<Response> {
    let path = &request.path;

    if let Some(rest) = path.strip_prefix("/redirect/") {
        let hop: u32 = rest.parse().unwrap_or(99);
        // Full M1 DoD matrix: 301/302/303/307/308 — one per hop, five hops
        // total before the final resource.
        if hop < 5 {
            let status_line = [
                "301 Moved Permanently",
                "302 Found",
                "303 See Other",
                "307 Temporary Redirect",
                "308 Permanent Redirect",
            ][hop as usize];
            return Some(Response {
                status_line,
                headers: format!("Location: /redirect/{}\r\n", hop + 1),
                body_len: 0,
                body_start: 0,
                body: Some(Vec::new()),
                generator: None,
                chunk_delay_ms: 0,
                omit_length: false,
            });
        }
        return Some(range_file(
            request,
            1024 * 1024,
            "\"redirect-final\"",
            None,
            state,
        ));
    }

    // True zero-byte representation (M1 §38): 200 with Content-Length: 0.
    if path == "/empty.bin" {
        return Some(Response {
            status_line: "200 OK",
            headers:
                "ETag: \"empty\"\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\n"
                    .to_string(),
            body_len: 0,
            body_start: 0,
            body: Some(Vec::new()),
            generator: None,
            chunk_delay_ms: 0,
            omit_length: false,
        });
    }

    match path.as_str() {
        "/status/404" => return Some(simple("404 Not Found")),
        "/status/403" => return Some(simple("403 Forbidden")),
        "/status/500" => return Some(simple("500 Internal Server Error")),
        "/status/429" => return Some(simple("429 Too Many Requests")),
        "/chunked/test.bin" => return Some(chunked_response(256 * 1024)),
        "/changing/file.bin" => {
            let version = state.changing_etag_version.fetch_add(1, Ordering::SeqCst);
            let etag: &'static str = Box::leak(format!("\"v{version}\"").into_boxed_str());
            return Some(range_file(request, 256 * 1024, etag, None, state));
        }
        p if p.starts_with("/slow/medium.bin") => {
            let mut r = range_file(request, 1024 * 1024, "\"slow-medium\"", None, state);
            r.chunk_delay_ms = 60;
            return Some(r);
        }
        "/unstable/file.bin" => {
            return Some(range_file(
                request,
                512 * 1024,
                "\"unstable\"",
                Some(128 * 1024),
                state,
            ));
        }
        "/huge.bin" => {
            return Some(range_file(
                request,
                4 * 1024 * 1024 * 1024 + 512,
                "\"huge\"",
                None,
                state,
            ));
        }
        _ => {}
    }

    let (len, etag): (u64, &'static str) = match path.as_str() {
        "/small.bin" => (64 * 1024, "\"small\""),
        "/medium.bin" => (1024 * 1024, "\"medium\""),
        "/range/file.bin" => (2 * 1024 * 1024, "\"range-file\""),
        "/norange/file.bin" => (512 * 1024, "\"norange\""),
        _ => (128 * 1024, "\"default\""),
    };
    if path == "/norange/file.bin" {
        return Some(full_ignoring_range(len, etag));
    }
    Some(range_file(request, len, etag, None, state))
}

fn full_ignoring_range(len: u64, etag: &'static str) -> Response {
    Response {
        status_line: "200 OK",
        headers: format!(
            "Accept-Ranges: none\r\nETag: {etag}\r\nContent-Type: application/octet-stream\r\n"
        ),
        body_len: len,
        body_start: 0,
        body: None,
        generator: Some(None),
        chunk_delay_ms: 0,
        omit_length: false,
    }
}

fn chunked_response(len: u64) -> Response {
    let mut body = Vec::with_capacity((len + 32) as usize);
    let mut offset = 0u64;
    while offset < len {
        let take = (len - offset).min(32 * 1024) as usize;
        body.extend_from_slice(format!("{take:x}\r\n").as_bytes());
        let mut chunk = vec![0u8; take];
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = pattern_byte(offset + i as u64);
        }
        body.extend_from_slice(&chunk);
        body.extend_from_slice(b"\r\n");
        offset += take as u64;
    }
    body.extend_from_slice(b"0\r\n\r\n");
    Response {
        status_line: "200 OK",
        omit_length: true,
        headers: "Transfer-Encoding: chunked\r\nETag: \"chunked\"\r\nContent-Type: application/octet-stream\r\n".to_string(),
        body_len: body.len() as u64,
        body_start: 0,
        body: Some(body),
        generator: None,
        chunk_delay_ms: 0,
    }
}

fn range_file(
    request: &Request,
    len: u64,
    etag: &'static str,
    reset_after_bytes: Option<u64>,
    state: &ServerState,
) -> Response {
    if let Some(range) = &request.range {
        state.seen_ranges.lock().unwrap().push(range.clone());
    }
    if let Some(if_range) = &request.if_range {
        state.seen_if_ranges.lock().unwrap().push(if_range.clone());
    }
    // If-Range mismatch → full 200 with the (new) representation.
    let if_range_ok = match &request.if_range {
        None => true,
        Some(v) => v == etag,
    };
    let base_headers = format!(
        "ETag: {etag}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\n"
    );
    if request.if_range.is_some() && !if_range_ok {
        return Response {
            status_line: "200 OK",
            headers: base_headers,
            body_len: len,
            body_start: 0,
            body: None,
            generator: Some(None),
            chunk_delay_ms: 0,
            omit_length: false,
        };
    }
    let Some(range) = &request.range else {
        return Response {
            status_line: "200 OK",
            headers: format!(
                "{base_headers}Content-Disposition: attachment; filename=\"served.bin\"\r\n"
            ),
            body_len: len,
            body_start: 0,
            body: None,
            generator: Some(reset_after_bytes),
            chunk_delay_ms: 0,
            omit_length: false,
        };
    };
    let Some(spec) = range.strip_prefix("bytes=") else {
        return simple("400 Bad Request");
    };
    let Some((start_str, end_str)) = spec.split_once('-') else {
        return simple("400 Bad Request");
    };
    let Ok(start) = start_str.parse::<u64>() else {
        return simple("400 Bad Request");
    };
    if start >= len {
        return Response {
            status_line: "416 Range Not Satisfiable",
            headers: format!("ETag: {etag}\r\nContent-Range: bytes */{len}\r\n"),
            body_len: 0,
            body_start: 0,
            body: Some(Vec::new()),
            generator: None,
            chunk_delay_ms: 0,
            omit_length: false,
        };
    }
    let end = if end_str.is_empty() {
        len - 1
    } else {
        end_str.parse::<u64>().unwrap_or(len - 1).min(len - 1)
    };
    Response {
        status_line: "206 Partial Content",
        headers: format!("{base_headers}Content-Range: bytes {start}-{end}/{len}\r\n"),
        body_len: end - start + 1,
        body_start: start,
        body: None,
        generator: Some(reset_after_bytes),
        chunk_delay_ms: 0,
        omit_length: false,
    }
}
