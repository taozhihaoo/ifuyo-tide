// M0.5 SPIKE — unified readable resource abstraction (experiments L/M).
//
// The question this file answers: can the gateway/player talk to
// "a resource" without caring whether bytes come from a finished file,
// a partially-downloaded torrent, or an HTTP server?
//
// Interface shape (conceptually `read(range)` + `availability(range)`):
//   * [`SpikeResource::len`]                      — total size
//   * [`SpikeResource::read_range`]               — bounded read, WAITS
//     for availability (the torrent implementation blocks on missing
//     pieces up to the configured timeout)
//   * [`SpikeResource::availability`]             — coarse availability
//
// This is deliberately an enum, not a trait object: Rust AFIT traits are
// not dyn-compatible and the spike should not add `async_trait`. One
// documented finding for M4 is that the real Resource layer needs a
// proper async trait design.

use super::torrent::TorrentSpike;
use crate::core::resource::ByteRange;
use crate::error::TideError;
use std::io::SeekFrom;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Per-read hard cap so no caller can ever request an unbounded read
/// (project-wide large-file principle).
pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;

/// Default wait for unavailable torrent data before failing a read.
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    /// All bytes present (complete file or finished torrent).
    Complete,
    /// Partially present; reads may wait.
    Partial {
        available_bytes: u64,
    },
    Unknown,
}

#[derive(Clone)]
pub struct FileResource {
    path: PathBuf,
    len: u64,
}

impl FileResource {
    pub fn open(path: PathBuf) -> Result<Self, TideError> {
        let len = std::fs::metadata(&path)?.len();
        Ok(FileResource { path, len })
    }
}

/// Combined trait object for stored torrent streams (Rust does not allow
/// multiple non-auto traits in one `dyn`).
trait SpikeStream: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send {}

impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send> SpikeStream for T {}

/// A torrent-backed resource. Every `read_range` opens an INDEPENDENT
/// FileStream: WebView2 issues concurrent range requests (head stream +
/// moov fetch at the file end) and a single shared, seeked stream would
/// corrupt them — measured finding of the M0.5 spike.
#[derive(Clone)]
pub struct TorrentResource {
    spike: Arc<TorrentSpike>,
    file_id: usize,
    len: u64,
    read_timeout: Duration,
}

impl TorrentResource {
    pub fn new(spike: Arc<TorrentSpike>, file_id: usize) -> Result<Self, TideError> {
        let len = spike
            .files
            .get(file_id)
            .ok_or_else(|| TideError::InvalidInput(format!("bad torrent file id {file_id}")))?
            .len;
        Ok(TorrentResource {
            spike,
            file_id,
            len,
            read_timeout: DEFAULT_READ_TIMEOUT,
        })
    }

    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.read_timeout = timeout;
    }
}

#[derive(Clone)]
pub struct HttpResource {
    addr: SocketAddr,
    path: String,
    len: u64,
}

impl HttpResource {
    pub fn new(addr: SocketAddr, path: String, len: u64) -> Self {
        HttpResource { addr, path, len }
    }
}

#[derive(Clone)]
pub enum SpikeResource {
    File(FileResource),
    Torrent(TorrentResource),
    Http(HttpResource),
}

impl SpikeResource {
    pub fn len(&self) -> u64 {
        match self {
            SpikeResource::File(r) => r.len,
            SpikeResource::Torrent(r) => r.len,
            SpikeResource::Http(r) => r.len,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self {
            SpikeResource::File(_) => "file",
            SpikeResource::Torrent(_) => "torrent",
            SpikeResource::Http(_) => "http",
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            SpikeResource::File(r) => super::fixtures::content_type_for(&r.path),
            SpikeResource::Torrent(r) => {
                let name = r
                    .spike
                    .files
                    .get(r.file_id)
                    .map(|f| f.name.clone())
                    .unwrap_or_default();
                super::fixtures::content_type_of_name(&name)
            }
            SpikeResource::Http(r) => {
                let name = r.path.rsplit('/').next().unwrap_or_default();
                let name = name.split('?').next().unwrap_or_default();
                if name.is_empty() {
                    "application/octet-stream"
                } else {
                    super::fixtures::content_type_of_name(name)
                }
            }
        }
    }

    /// Coarse availability (experiment L: `availability(range)`).
    pub fn availability(&self, range: ByteRange) -> Availability {
        match self {
            SpikeResource::File(_) => Availability::Complete,
            SpikeResource::Http(_) => Availability::Unknown,
            SpikeResource::Torrent(r) => {
                let stats = r.spike.status();
                let file_available = stats.file_progress.get(r.file_id).copied().unwrap_or(0);
                if stats.finished {
                    Availability::Complete
                } else {
                    // Coarse byte-level approximation: file progress is
                    // reported as contiguous-from-start bytes by librqbit.
                    let _ = range;
                    Availability::Partial {
                        available_bytes: file_available,
                    }
                }
            }
        }
    }

    /// Reads exactly `range.len()` bytes, waiting for availability when
    /// backed by an incomplete torrent. Bounded by MAX_READ_BYTES and a
    /// per-implementation timeout.
    pub async fn read_range(&mut self, range: ByteRange) -> Result<Vec<u8>, TideError> {
        if range.length() > MAX_READ_BYTES {
            return Err(TideError::InvalidInput(format!(
                "read of {} bytes exceeds the {} byte spike cap",
                range.length(),
                MAX_READ_BYTES
            )));
        }
        if range.end() > self.len() {
            return Err(TideError::InvalidInput(
                "read range beyond end of resource".into(),
            ));
        }
        match self {
            SpikeResource::File(r) => {
                use std::io::{Read, Seek};
                let mut file = std::fs::File::open(&r.path)?;
                file.seek(SeekFrom::Start(range.start()))?;
                let mut buf = vec![0u8; range.length() as usize];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            SpikeResource::Torrent(r) => {
                // Independent stream per read: concurrent range requests
                // (head + moov) must not share a seeked stream.
                let mut stream =
                    Box::new(r.spike.open_stream(r.file_id).await?) as Box<dyn SpikeStream>;
                stream
                    .seek(SeekFrom::Start(range.start()))
                    .await
                    .map_err(|err| TideError::Internal(format!("torrent seek failed: {err}")))?;
                let mut buf = vec![0u8; range.length() as usize];
                let read = tokio::time::timeout(r.read_timeout, stream.read_exact(&mut buf))
                    .await
                    .map_err(|_| {
                        TideError::Internal(format!(
                            "torrent data for range {}..{} did not arrive within {:?}",
                            range.start(),
                            range.end(),
                            r.read_timeout
                        ))
                    })?
                    .map_err(|err| TideError::Internal(format!("torrent read failed: {err}")))?;
                debug_assert_eq!(read, buf.len());
                Ok(buf)
            }
            SpikeResource::Http(r) => {
                let range_end = range.end() - 1; // HTTP inclusive
                let request = format!(
                    "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes={}-{}\r\nConnection: close\r\n\r\n",
                    r.path, range.start(), range_end
                );
                let mut stream = tokio::net::TcpStream::connect(r.addr)
                    .await
                    .map_err(|err| TideError::Internal(format!("http connect failed: {err}")))?;
                use tokio::io::AsyncWriteExt;
                stream
                    .write_all(request.as_bytes())
                    .await
                    .map_err(|err| TideError::Internal(format!("http write failed: {err}")))?;
                let mut raw = Vec::with_capacity(64 * 1024);
                stream
                    .read_to_end(&mut raw)
                    .await
                    .map_err(|err| TideError::Internal(format!("http read failed: {err}")))?;
                parse_http_body_206(&raw, range.length() as usize)
            }
        }
    }
}

/// Splits a raw HTTP response into (status, body) for the spike client.
fn parse_http_body_206(raw: &[u8], expected_len: usize) -> Result<Vec<u8>, TideError> {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| TideError::Internal("http response truncated".into()))?;
    let status_line =
        String::from_utf8_lossy(&raw[..raw.iter().position(|b| *b == b'\r').unwrap_or(0)]);
    if !status_line.contains("206") {
        return Err(TideError::Internal(format!(
            "http source returned non-206: {status_line}"
        )));
    }
    let body = &raw[header_end + 4..];
    if body.len() < expected_len {
        return Err(TideError::Internal(format!(
            "http source sent {} bytes, expected {}",
            body.len(),
            expected_len
        )));
    }
    Ok(body[..expected_len].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_206_with_headers() {
        let mut raw =
            b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-9/100\r\n\r\n".to_vec();
        raw.extend_from_slice(b"0123456789");
        let body = parse_http_body_206(&raw, 10).unwrap();
        assert_eq!(body, b"0123456789");
    }

    #[test]
    fn rejects_non_206() {
        let raw = b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec();
        assert!(parse_http_body_206(&raw, 1).is_err());
    }
}
