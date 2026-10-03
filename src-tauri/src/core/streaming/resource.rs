//! Streaming resources (M4(上) §6–§14/§20–§36): a unified readable
//! abstraction over a finished file, an in-progress HTTP download and a
//! torrent file — the SAME gateway serves all three (M4(上) §72/§74).

use crate::core::resource::ByteRange;
use crate::error::TideError;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Per-read hard cap (project large-file principle).
pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;

/// Default wait for unavailable torrent data before failing a read.
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Complete,
    Partial { available_bytes: u64 },
    Unknown,
}

/// A readable byte source for streaming.
#[derive(Clone)]
pub enum StreamingResource {
    /// Local file: a completed download or the `.part` of an in-progress
    /// HTTP download (progressive playback of what exists).
    File { path: PathBuf, len: u64 },
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
    pub fn len(&self) -> u64 {
        match self {
            StreamingResource::File { len, .. } => *len,
            StreamingResource::Torrent { len, .. } => *len,
            StreamingResource::Http { len, .. } => *len,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self {
            StreamingResource::File { .. } => "file",
            StreamingResource::Torrent { .. } => "torrent",
            StreamingResource::Http { .. } => "http",
        }
    }

    /// Coarse availability of a range (M4(上) §10-§12/§26/§34).
    pub fn availability(&self, range: crate::core::resource::ByteRange) -> Availability {
        match self {
            StreamingResource::File { .. } => Availability::Complete,
            StreamingResource::Http { .. } => Availability::Unknown,
            StreamingResource::Torrent { engine, info_hash, .. } => {
                let Some(handle) = engine.handle_for(info_hash) else {
                    return Availability::Unknown;
                };
                let stats = handle.stats();
                if stats.finished {
                    return Availability::Complete;
                }
                // file_progress[i] is the contiguous downloaded prefix of
                // file i (librqbit semantics).
                let _ = range;
                let idx = 0usize;
                let _ = idx;
                Availability::Partial {
                    available_bytes: stats.progress_bytes,
                }
            }
        }
    }

    /// Reads exactly `range.len()` bytes. Torrent-backed reads WAIT for
    /// missing pieces (bounded by the read timeout). One independent
    /// stream per read: WebView2 issues concurrent range requests and a
    /// shared seeked stream would corrupt them (M0.5 measured finding).
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
            StreamingResource::File { path, .. } => {
                use std::io::{Read, Seek};
                let mut file = std::fs::File::open(path)?;
                file.seek(SeekFrom::Start(range.start()))?;
                let mut buf = vec![0u8; range.length() as usize];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            StreamingResource::Torrent { engine, info_hash, file_index, .. } => {
                let handle = engine.handle_for(info_hash).ok_or_else(|| {
                    TideError::Internal("torrent handle missing for streaming".into())
                })?;
                let mut stream = handle
                    .clone()
                    .stream(*file_index)
                    .await
                    .map_err(|err| {
                        TideError::Internal(format!("torrent stream open failed: {err}"))
                    })?;
                stream
                    .seek(SeekFrom::Start(range.start()))
                    .await
                    .map_err(|err| TideError::Internal(format!("torrent seek failed: {err}")))?;
                let mut buf = vec![0u8; range.length() as usize];
                tokio::time::timeout(DEFAULT_READ_TIMEOUT, stream.read_exact(&mut buf))
                    .await
                    .map_err(|_| {
                        TideError::Internal(format!(
                            "torrent data for range {}..{} did not arrive within {DEFAULT_READ_TIMEOUT:?}",
                            range.start(),
                            range.end()
                        ))
                    })?
                    .map_err(|err| TideError::Internal(format!("torrent read failed: {err}")))?;
                Ok(buf)
            }
            StreamingResource::Http { url, .. } => {
                read_http_range(url, range.start(), range.length()).await
            }
        }
    }
}

/// Ranged GET via reqwest (identity encoding; M1 §17 semantics).
async fn read_http_range(url: &str, start: u64, len: u64) -> Result<Vec<u8>, TideError> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|err| TideError::Internal(format!("http client build failed: {err}")))?;
    let range_end = start + len - 1; // HTTP inclusive
    let response = client
        .get(url)
        .header("Range", format!("bytes={start}-{range_end}"))
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|err| TideError::Internal(format!("http request failed: {err}")))?;
    if !response.status().is_success() {
        return Err(TideError::Internal(format!(
            "http source returned {}",
            response.status()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|err| TideError::Internal(format!("http read failed: {err}")))?;
    if bytes.len() as u64 != len {
        return Err(TideError::Internal(format!(
            "http source sent {} bytes, expected {len}",
            bytes.len()
        )));
    }
    Ok(bytes.to_vec())
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
}
