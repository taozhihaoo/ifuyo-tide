//! Streaming resources (M4(上) §6–§14/§20–§36, M4(下) §111/§143/§146):
//! a unified readable abstraction over a finished file, an in-progress
//! HTTP download (`.part`), a torrent file and a remote HTTP origin —
//! the SAME gateway serves all of them (M4(上) §72/§74).

use crate::core::resource::ByteRange;
use crate::error::{StreamingFailure, TideError};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Per-read hard cap (project large-file principle).
pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;

/// Default wait for unavailable data before failing a read (M4(下)
/// §96/§97: bounded resource wait, never an infinite hang).
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(120);

/// How long a read beyond the `.part` frontier waits for LOCAL growth
/// before falling back to the origin (M4(下) §145: seeks should not
/// stall behind a sequential download when the origin can serve).
pub const FRONTIER_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Complete,
    Partial { available_bytes: u64 },
    Unknown,
}

/// What a resource can do (M4(下) §146/§147). Formed once at session
/// creation — the frontend never branches on source kind, only on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCapability {
    pub supports_range: bool,
    pub supports_random_access: bool,
    pub supports_progressive: bool,
    pub known_size: bool,
}

/// A remote origin a `.part` can read through when the local frontier
/// cannot satisfy a seek (M4(下) §127 read-through path).
#[derive(Clone)]
pub struct OriginSource {
    pub url: String,
}

/// A readable byte source for streaming.
#[derive(Clone)]
pub enum StreamingResource {
    /// Local file: a completed download or any authorized local file.
    File { path: PathBuf, len: u64 },
    /// The `.part` of an IN-PROGRESS HTTP download. `total_len` is the
    /// declared remote size (None = unknown); `len_rx` receives the
    /// durable length after every flushed chunk (event-driven frontier,
    /// M4(下) §115); `origin` enables read-through for ranges beyond
    /// the frontier (§127/§145).
    Part {
        path: PathBuf,
        total_len: Option<u64>,
        len_rx: Option<tokio::sync::watch::Receiver<u64>>,
        origin: Option<OriginSource>,
    },
    /// A torrent file served through the M3 engine; reads WAIT for
    /// missing pieces. Engine clone shares the librqbit Session (Arc).
    Torrent {
        engine: crate::core::torrent::TorrentEngine,
        info_hash: String,
        file_index: usize,
        len: u64,
    },
    /// Remote HTTP(S) origin, read through ranged requests.
    Http {
        url: String,
        len: u64,
        accept_ranges: bool,
    },
}

impl StreamingResource {
    /// Declared total size (the number Range math is built on; M4(下)
    /// §108: known-size resources must serve exact Content-Length).
    pub fn len(&self) -> u64 {
        match self {
            StreamingResource::File { len, .. } => *len,
            StreamingResource::Part { path, total_len, .. } => {
                total_len.unwrap_or_else(|| physical_len(path))
            }
            StreamingResource::Torrent { len, .. } => *len,
            StreamingResource::Http { len, .. } => *len,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self {
            StreamingResource::File { .. } => "file",
            StreamingResource::Part { .. } => "part",
            StreamingResource::Torrent { .. } => "torrent",
            StreamingResource::Http { .. } => "http",
        }
    }

    /// Cheap pre-flight check used by the gateway to answer 503 before
    /// headers instead of aborting mid-body (M4(下) §107).
    pub fn is_available(&self) -> bool {
        match self {
            StreamingResource::File { path, .. } => path.exists(),
            StreamingResource::Part { path, .. } => path.exists(),
            StreamingResource::Torrent {
                engine, info_hash, ..
            } => engine.handle_for(info_hash).is_some(),
            StreamingResource::Http { .. } => true,
        }
    }

    pub fn capability(&self) -> PlaybackCapability {
        match self {
            StreamingResource::File { .. } => PlaybackCapability {
                supports_range: true,
                supports_random_access: true,
                supports_progressive: true,
                known_size: true,
            },
            StreamingResource::Part { total_len, origin, .. } => PlaybackCapability {
                supports_range: true,
                // Beyond-frontier access needs the origin read-through
                // path; local-only parts can serve what exists.
                supports_random_access: origin.is_some(),
                supports_progressive: true,
                known_size: total_len.is_some(),
            },
            StreamingResource::Torrent { .. } => PlaybackCapability {
                supports_range: true,
                supports_random_access: true,
                supports_progressive: true,
                known_size: true,
            },
            StreamingResource::Http { accept_ranges, .. } => PlaybackCapability {
                supports_range: *accept_ranges,
                supports_random_access: *accept_ranges,
                supports_progressive: true,
                known_size: true,
            },
        }
    }

    /// Bytes durably readable right now (M4(下) §111/§149: backend
    /// availability, distinct from the browser's buffered ranges).
    pub fn available_len(&self) -> u64 {
        match self {
            StreamingResource::File { len, .. } => *len,
            StreamingResource::Part { path, .. } => physical_len(path),
            StreamingResource::Torrent { engine, info_hash, len, .. } => {
                match engine.handle_for(info_hash) {
                    // Coarse global-progress approximation (librqbit
                    // reports contiguous download progress; per-file
                    // byte mapping stays inside the FileStream).
                    Some(handle) => {
                        let stats = handle.stats();
                        if stats.finished {
                            *len
                        } else {
                            stats.progress_bytes.min(*len)
                        }
                    }
                    None => 0,
                }
            }
            StreamingResource::Http { .. } => 0,
        }
    }

    /// Coarse availability of a range (M4(上) §10-§12/§26/§34). The
    /// interval-based AvailabilitySet is deferred with the Range-Writer
    /// feature (see docs/M4-availability.md).
    pub fn availability(&self, range: crate::core::resource::ByteRange) -> Availability {
        match self {
            StreamingResource::File { .. } => Availability::Complete,
            StreamingResource::Http { .. } => Availability::Unknown,
            StreamingResource::Part { .. } => {
                let avail = self.available_len();
                if range.end() <= avail {
                    Availability::Complete
                } else {
                    Availability::Partial {
                        available_bytes: avail,
                    }
                }
            }
            StreamingResource::Torrent { engine, info_hash, .. } => {
                let Some(handle) = engine.handle_for(info_hash) else {
                    return Availability::Unknown;
                };
                let stats = handle.stats();
                if stats.finished {
                    return Availability::Complete;
                }
                Availability::Partial {
                    available_bytes: stats.progress_bytes,
                }
            }
        }
    }

    /// Reads exactly `range.len()` bytes, waiting for availability when
    /// the data is not on disk yet (bounded everywhere, M4(下) §97).
    ///
    /// `.part` reads: fully-local ranges are served from the file;
    /// beyond-frontier ranges grace-wait for LOCAL growth (event-driven
    /// via the durable-len watch, §115), then read the missing tail
    /// through the origin (§127/§145). Origin responses that ignore
    /// Range are an error — never silently served as wrong bytes.
    pub async fn read_range(&self, range: ByteRange) -> Result<Vec<u8>, TideError> {
        if range.length() as u64 > MAX_READ_BYTES {
            return Err(TideError::InvalidInput(format!(
                "read of {} bytes exceeds the {MAX_READ_BYTES} byte cap",
                range.length()
            )));
        }
        if range.end() > self.len() {
            return Err(TideError::InvalidInput(
                "read range beyond end of resource".into(),
            ));
        }
        match self {
            StreamingResource::File { path, .. } => read_file_range(path, range).await,
            StreamingResource::Part {
                path,
                len_rx,
                origin,
                ..
            } => read_part_range(path, len_rx.clone(), origin.as_ref(), range).await,
            StreamingResource::Torrent {
                engine,
                info_hash,
                file_index,
                ..
            } => {
                let handle = engine.handle_for(info_hash).ok_or_else(|| {
                    TideError::Streaming(StreamingFailure::Resource(
                        "torrent is not in the engine".into(),
                    ))
                })?;
                let mut stream = handle
                    .clone()
                    .stream(*file_index)
                    .await
                    .map_err(|err| {
                        TideError::Streaming(StreamingFailure::Resource(format!(
                            "torrent stream open failed: {err}"
                        )))
                    })?;
                stream
                    .seek(SeekFrom::Start(range.start()))
                    .await
                    .map_err(|err| {
                        TideError::Streaming(StreamingFailure::Resource(format!(
                            "torrent seek failed: {err}"
                        )))
                    })?;
                let mut buf = vec![0u8; range.length() as usize];
                let read = tokio::time::timeout(DEFAULT_READ_TIMEOUT, stream.read_exact(&mut buf))
                    .await
                    .map_err(|_| {
                        TideError::Streaming(StreamingFailure::WaitTimeout)
                    })?
                    .map_err(|err| {
                        TideError::Streaming(StreamingFailure::Resource(format!(
                            "torrent read failed: {err}"
                        )))
                    })?;
                debug_assert_eq!(read, buf.len());
                Ok(buf)
            }
            StreamingResource::Http { url, accept_ranges, .. } => {
                if !accept_ranges {
                    return Err(TideError::Streaming(StreamingFailure::RangeUnsupported));
                }
                read_origin_range(url, range.start(), range.length() as usize).await
            }
        }
    }
}

fn physical_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Waits (event-driven, no polling: M4(下) §115) until the durable
/// length reaches `beyond`, the deadline passes, or the watch closes.
async fn wait_for_durable_len(
    rx: &mut tokio::sync::watch::Receiver<u64>,
    beyond: u64,
    deadline: Instant,
) -> u64 {
    loop {
        if *rx.borrow() >= beyond {
            return *rx.borrow();
        }
        let now = Instant::now();
        if now >= deadline {
            return *rx.borrow();
        }
        match tokio::time::timeout(deadline - now, rx.changed()).await {
            Ok(Ok(())) => continue,
            // Sender dropped: the engine task ended; no further growth.
            Ok(Err(_)) | Err(_) => return *rx.borrow(),
        }
    }
}

async fn read_file_range(path: &Path, range: ByteRange) -> Result<Vec<u8>, TideError> {
    let mut file = tokio::fs::File::open(path).await.map_err(|_| {
        TideError::Streaming(StreamingFailure::Resource("file disappeared".into()))
    })?;
    file.seek(SeekFrom::Start(range.start())).await.map_err(|err| {
        TideError::Streaming(StreamingFailure::Resource(format!("seek failed: {err}")))
    })?;
    let mut buf = vec![0u8; range.length() as usize];
    file.read_exact(&mut buf).await.map_err(|_| {
        TideError::Streaming(StreamingFailure::Resource(
            "file shorter than expected".into(),
        ))
    })?;
    Ok(buf)
}

/// `.part` read model (M4(下) §111/§112/§127):
/// 1. everything at or below the durable frontier comes from the file;
/// 2. a beyond-frontier range grace-waits for local growth (critical
///    test #1: the frontier extends, playback continues);
/// 3. whatever is STILL missing after the grace window is read through
///    the origin (critical test #2: seek into an un-downloaded region).
async fn read_part_range(
    path: &Path,
    mut len_rx: Option<tokio::sync::watch::Receiver<u64>>,
    origin: Option<&OriginSource>,
    range: ByteRange,
) -> Result<Vec<u8>, TideError> {
    let deadline = Instant::now() + DEFAULT_READ_TIMEOUT;
    // Grace wait ONLY when the range asks for bytes beyond the local
    // frontier; purely-local reads never wait.
    let mut frontier = physical_len(path);
    if range.end() > frontier {
        if let Some(rx) = len_rx.as_mut() {
            let grace_deadline = (Instant::now() + FRONTIER_GRACE).min(deadline);
            frontier = wait_for_durable_len(rx, range.end(), grace_deadline).await;
        }
    }

    let local_end = range.end().min(frontier);
    let mut out = Vec::new();
    if range.start() < local_end {
        let local = ByteRange::new(range.start(), local_end)
            .map_err(|err| TideError::Streaming(StreamingFailure::Range(err.to_string())))?;
        out = read_file_range(path, local).await?;
    }
    if local_end < range.end() {
        let Some(origin) = origin else {
            // No origin: keep waiting for the local frontier until the
            // overall deadline (critical test #1 path), then fail with
            // a structured timeout (§97) — never a short read.
            if let Some(rx) = len_rx.as_mut() {
                frontier = wait_for_durable_len(rx, range.end(), deadline).await;
            }
            if range.end() > frontier {
                return Err(TideError::Streaming(StreamingFailure::WaitTimeout));
            }
            let local = ByteRange::new(range.start(), range.end())
                .map_err(|err| TideError::Streaming(StreamingFailure::Range(err.to_string())))?;
            return read_file_range(path, local).await;
        };
        // Read through ONLY what the local file could not provide —
        // starting at the first byte of the requested range that is
        // missing (a fully-beyond-frontier seek starts at range.start,
        // a straddling one at the frontier).
        let origin_start = range.start() + out.len() as u64;
        let tail =
            read_origin_range(&origin.url, origin_start, (range.end() - origin_start) as usize)
                .await?;
        out.extend_from_slice(&tail);
    }
    debug_assert_eq!(out.len() as u64, range.length() as u64);
    Ok(out)
}

/// Ranged GET via reqwest (identity encoding; M1 §17 semantics).
///
/// §143: errors are classified (timeout / connect / HTTP status), and a
/// 200 answer to a ranged request is RangeUnsupported — the body would
/// start at offset 0, so serving it as `[start, start+len)` would be
/// WRONG BYTES (P0 class). The read is stream-limited to exactly `len`
/// bytes; a short body is a structured failure, not a short read.
async fn read_origin_range(
    url: &str,
    start: u64,
    len: usize,
) -> Result<Vec<u8>, TideError> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|err| TideError::Streaming(StreamingFailure::Gateway(format!("client build failed: {err}"))))?;
    let range_end = start + len as u64 - 1; // HTTP inclusive
    let response = client
        .get(url)
        .header("Range", format!("bytes={start}-{range_end}"))
        .header("Accept-Encoding", "identity")
        .timeout(DEFAULT_READ_TIMEOUT)
        .send()
        .await
        .map_err(|err| classify_reqwest(&err))?;
    let status = response.status();
    if status == reqwest::StatusCode::OK {
        // Origin ignored the Range header (M4(下) §145).
        crate::diagnostics::warn(
            "streaming.range_unsupported",
            &[("status", "200")],
        );
        return Err(TideError::Streaming(StreamingFailure::RangeUnsupported));
    }
    if !status.is_success() {
        return Err(TideError::Streaming(StreamingFailure::HttpStatus(
            status.as_u16(),
        )));
    }
    let mut out = Vec::with_capacity(len);
    let mut response = response;
    while out.len() < len {
        match response.chunk().await.map_err(|err| classify_reqwest(&err))? {
            Some(chunk) => {
                let take = (len - out.len()).min(chunk.len());
                out.extend_from_slice(&chunk[..take]);
                if take < chunk.len() {
                    // More body than requested: stop reading; the
                    // connection is dropped with the response.
                    break;
                }
            }
            None => break,
        }
    }
    if out.len() != len {
        return Err(TideError::Streaming(StreamingFailure::Resource(format!(
            "origin sent {} of {} requested bytes",
            out.len(),
            len
        ))));
    }
    Ok(out)
}

/// M4(下) §143: one classifier for network failures — no blanket
/// NetworkError. (Reset/refused/DNS all surface as NETWORK_CONNECT;
/// the distinguishing detail stays in logs.)
fn classify_reqwest(err: &reqwest::Error) -> TideError {
    if err.is_timeout() {
        return TideError::Streaming(StreamingFailure::NetworkTimeout);
    }
    if err.is_connect() || err.is_body() || err.is_decode() || err.is_request() {
        return TideError::Streaming(StreamingFailure::NetworkConnect);
    }
    if let Some(status) = err.status() {
        return TideError::Streaming(StreamingFailure::HttpStatus(status.as_u16()));
    }
    TideError::Streaming(StreamingFailure::NetworkConnect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_and_labels() {
        let f = StreamingResource::File {
            path: PathBuf::from("x"),
            len: 10,
        };
        assert_eq!(f.len(), 10);
        assert_eq!(f.source_label(), "file");
    }

    #[test]
    fn part_capability_reflects_origin() {
        let local_only = StreamingResource::Part {
            path: PathBuf::from("x.part"),
            total_len: Some(100),
            len_rx: None,
            origin: None,
        };
        assert!(!local_only.capability().supports_random_access);
        assert!(local_only.capability().supports_progressive);
        assert!(local_only.capability().known_size);

        let with_origin = StreamingResource::Part {
            path: PathBuf::from("x.part"),
            total_len: None,
            len_rx: None,
            origin: Some(OriginSource {
                url: "http://127.0.0.1:1/f.bin".into(),
            }),
        };
        assert!(with_origin.capability().supports_random_access);
        assert!(!with_origin.capability().known_size);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_resource_reads_range() {
        // Mini origin: answers a single-byte-range GET with 206 and the
        // exact requested slice of the patterned payload.
        let payload: Vec<u8> = (0..4096u64).map(|i| (i % 251) as u8).collect();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_payload = payload.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let body = server_payload.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = vec![0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let text = String::from_utf8_lossy(&buf);
                    let range = text
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                        .and_then(|l| l.split_once(':'))
                        .map(|(_, v)| v.trim().trim_start_matches("bytes=").to_string())
                        .unwrap_or_default();
                    let (start, end) = range.split_once('-').unwrap_or(("0", "0"));
                    let start: u64 = start.parse().unwrap_or(0);
                    let end: u64 = end.parse().unwrap_or(0);
                    let slice = &body[start as usize..=(end.min(body.len() as u64 - 1)) as usize];
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                        slice.len(),
                        body.len()
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(slice).await;
                });
            }
        });
        let res = StreamingResource::Http {
            url: format!("http://{addr}/f.bin"),
            len: payload.len() as u64,
            accept_ranges: true,
        };
        let bytes = res
            .read_range(ByteRange::new(100, 1124).unwrap())
            .await
            .expect("read range");
        assert_eq!(bytes.len(), 1024);
        assert_eq!(bytes[..], payload[100..1124]);
    }

    /// M4(下) §145: an origin that ignores Range (answers 200) must
    /// yield a structured error — never silently-served wrong bytes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn origin_that_ignores_range_is_rejected() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    // Read the request first: closing with unread data
                    // would RST the connection and mask the response.
                    let mut buf = vec![0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let body = vec![0u8; 4096];
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(&body).await;
                });
            }
        });
        let res = StreamingResource::Http {
            url: format!("http://{addr}/f.bin"),
            len: 4096,
            accept_ranges: true,
        };
        let err = res
            .read_range(ByteRange::new(100, 1124).unwrap())
            .await
            .expect_err("200-to-range must fail");
        assert_eq!(err.code(), "RESOURCE_RANGE_UNSUPPORTED");
    }
}
