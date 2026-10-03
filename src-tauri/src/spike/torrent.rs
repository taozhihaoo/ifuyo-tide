// M0.5 SPIKE — librqbit experiment harness (experiments C/D/E/F).
//
// Creates a real .torrent from the generated media, seeds it from a
// local librqbit session, and downloads it with a second session over a
// direct loopback peer connection (no tracker, no DHT, fully local and
// license-clean). Exposes progressive reads through `ManagedTorrent::
// stream(file_id)` — the critical experiment D capability.
//
// NOTHING here is production architecture; M3 will build the real
// TorrentEngine adapter on validated evidence.

use crate::error::TideError;
// librqbit 9.0.1 only re-exports these at the crate root (the modules
// themselves are private) — verified against the vendored source.
use librqbit::{
    AddTorrent, AddTorrentOptions, CreateTorrentOptions, ListenerMode, ListenerOptions,
    ManagedTorrent, Session, SessionOptions,
};
use serde::Serialize;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncSeek};

/// Piece size for spike torrents: small enough that seek experiments
/// cross piece boundaries quickly, large enough to hash fast.
const SPIKE_PIECE_LENGTH: u32 = 128 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeFileInfo {
    pub id: usize,
    pub name: String,
    pub len: u64,
    pub offset_in_torrent: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeTorrentStatus {
    pub info_hash: String,
    pub progress_bytes: u64,
    pub total_bytes: u64,
    pub finished: bool,
    pub paused: bool,
    pub error: Option<String>,
    pub file_progress: Vec<u64>,
}

pub struct TorrentSpike {
    pub info_hash: String,
    pub files: Vec<SpikeFileInfo>,
    pub download_dir: PathBuf,
    seeder: Arc<Session>,
    downloader: Arc<Session>,
    handle: Arc<ManagedTorrent>,
}

fn spike_session_options() -> SessionOptions {
    SessionOptions {
        // Fully local experiments: no DHT, no trackers, loopback TCP only.
        dht: None,
        disable_trackers: true,
        listen: Some(ListenerOptions {
            listen_addr: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 0),
            mode: ListenerMode::TcpOnly,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn new_session(output_folder: PathBuf) -> Result<Arc<Session>, TideError> {
    std::fs::create_dir_all(&output_folder)?;
    Session::new_with_opts(output_folder, spike_session_options())
        .await
        .map_err(|err| TideError::Internal(format!("failed to start librqbit session: {err}")))
}

/// A scratch folder for the seeder session (its real output folder is
/// decided by create_and_serve_torrent).
fn work_dir_of(_seed_path: &Path, fallback: Option<&Path>) -> PathBuf {
    fallback
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("tide-spike-seed"))
}

async fn wait_for_listen_addr(session: &Session) -> Result<SocketAddr, TideError> {
    for _ in 0..50 {
        if let Some(addr) = session.listen_addr() {
            return Ok(addr);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(TideError::Internal(
        "librqbit session never bound a listen port".into(),
    ))
}

impl TorrentSpike {
    /// Seeds `source_file` from one session and downloads it in another.
    ///
    /// The download directory is wiped first: spike runs must be
    /// reproducible and a stale partial/complete file changes librqbit's
    /// initialization path (existing-data verification).
    pub async fn start_single(
        source_file: &Path,
        work_dir: &Path,
    ) -> Result<TorrentSpike, TideError> {
        let download_dir = work_dir.join("torrent-download");
        let _ = std::fs::remove_dir_all(&download_dir);
        std::fs::create_dir_all(&download_dir)?;
        // Whole-harness bound: any librqbit await that hangs in the app
        // runtime must surface as an error, not a silent stall.
        match tokio::time::timeout(
            Duration::from_secs(120),
            Self::start(source_file, download_dir),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(TideError::Internal(
                "torrent harness setup timed out after 120s (see spike.torrent.step logs)"
                    .into(),
            )),
        }
    }

    /// Seeds a whole directory (multi-file torrent, experiment F).
    pub async fn start_multi(
        source_dir: &Path,
        work_dir: &Path,
    ) -> Result<TorrentSpike, TideError> {
        let download_dir = work_dir.join("torrent-download");
        let _ = std::fs::remove_dir_all(&download_dir);
        std::fs::create_dir_all(&download_dir)?;
        match tokio::time::timeout(
            Duration::from_secs(120),
            Self::start(source_dir, download_dir),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(TideError::Internal(
                "torrent harness setup timed out after 120s (see spike.torrent.step logs)"
                    .into(),
            )),
        }
    }

    async fn start(seed_path: &Path, download_dir: PathBuf) -> Result<TorrentSpike, TideError> {
        let started = std::time::Instant::now();
        let step = |name: &str| {
            crate::diagnostics::info(
                "spike.torrent.step",
                &[
                    ("step", name),
                    ("ms", &started.elapsed().as_millis().to_string()),
                ],
            );
        };

        step("session-seeder");
        let seeder = new_session(work_dir_of(seed_path, download_dir.parent())).await?;
        step("session-seeder-ok");

        // Create + seed in one step. librqbit requires `overwrite` to
        // seed files that already exist; create_and_serve_torrent does
        // that internally.
        let (created, seed_handle) = seeder
            .create_and_serve_torrent(
                seed_path,
                CreateTorrentOptions {
                    name: None,
                    trackers: Vec::new(),
                    piece_length: Some(SPIKE_PIECE_LENGTH),
                },
            )
            .await
            .map_err(|err| {
                TideError::Internal(format!("create_and_serve_torrent failed: {err}"))
            })?;
        let _ = seed_handle;
        step("created-and-seeding");

        let torrent_bytes = created
            .as_bytes()
            .map_err(|err| TideError::Internal(format!("torrent serialization failed: {err}")))?;
        let info_hash = created.info_hash().as_string();

        let seeder_addr = wait_for_listen_addr(&seeder).await?;
        step("seeder-listening");

        let downloader = new_session(download_dir.clone()).await?;
        step("session-downloader");
        let response = downloader
            .add_torrent(
                AddTorrent::TorrentFileBytes(torrent_bytes.clone()),
                Some(AddTorrentOptions {
                    initial_peers: Some(vec![seeder_addr]),
                    paused: false,
                    overwrite: false,
                    output_folder: Some(download_dir.to_string_lossy().to_string()),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|err| TideError::Internal(format!("add_torrent failed: {err}")))?;
        let handle = response
            .into_handle()
            .ok_or_else(|| TideError::Internal("torrent did not produce a handle".into()))?;
        step("added-to-downloader");

        // Wait for metadata so file listings are available immediately.
        // Bounded: an app-runtime deadlock must surface as an error, not
        // hang the spike forever.
        tokio::time::timeout(Duration::from_secs(30), handle.wait_until_initialized())
            .await
            .map_err(|_| {
                TideError::Internal("torrent initialization timed out after 30s".into())
            })?
            .map_err(|err| TideError::Internal(format!("torrent metadata failed: {err}")))?;
        step("initialized");

        let files = handle
            .with_metadata(|metadata| {
                metadata
                    .file_infos
                    .iter()
                    .enumerate()
                    .map(|(id, info)| SpikeFileInfo {
                        id,
                        name: info.relative_filename.to_string_lossy().to_string(),
                        len: info.len,
                        offset_in_torrent: info.offset_in_torrent,
                    })
                    .collect::<Vec<_>>()
            })
            .map_err(|err| TideError::Internal(format!("torrent metadata unavailable: {err}")))?;

        crate::diagnostics::info(
            "spike.torrent.started",
            &[
                ("info_hash", &info_hash),
                ("files", &files.len().to_string()),
                ("setup_ms", &started.elapsed().as_millis().to_string()),
            ],
        );

        Ok(TorrentSpike {
            info_hash,
            files,
            download_dir,
            seeder,
            downloader,
            handle,
        })
    }

    pub fn status(&self) -> SpikeTorrentStatus {
        let stats = self.handle.stats();
        SpikeTorrentStatus {
            info_hash: self.info_hash.clone(),
            progress_bytes: stats.progress_bytes,
            total_bytes: stats.total_bytes,
            finished: stats.finished,
            paused: self.handle.is_paused(),
            error: stats.error.clone(),
            file_progress: stats.file_progress.clone(),
        }
    }

    /// Opens a progressive, seek-capable read handle for one file.
    /// Reads block until the required pieces arrive — experiment D/E.
    ///
    /// librqbit 9.0.1 implements tokio's `AsyncRead`/`AsyncSeek` on its
    /// (crate-private) `FileStream` type, so the spike returns `impl
    /// Trait`; reads are `Unpin` and block on missing pieces.
    pub async fn open_stream(
        &self,
        file_id: usize,
    ) -> Result<impl AsyncRead + AsyncSeek + Unpin, TideError> {
        if file_id >= self.files.len() {
            return Err(TideError::InvalidInput(format!(
                "file id {file_id} out of range (torrent has {} files)",
                self.files.len()
            )));
        }
        self.handle
            .clone()
            .stream(file_id)
            .await
            .map_err(|err| TideError::Internal(format!("stream open failed: {err}")))
    }

    /// The path the downloader writes the file to (for verification).
    pub fn downloaded_path(&self, file_id: usize) -> Result<PathBuf, TideError> {
        let info = self
            .files
            .get(file_id)
            .ok_or_else(|| TideError::InvalidInput("bad file id".into()))?;
        Ok(self.download_dir.join(&info.name))
    }

    pub async fn pause(&self) -> Result<(), TideError> {
        self.downloader
            .pause(&self.handle)
            .await
            .map_err(|err| TideError::Internal(format!("pause failed: {err}")))
    }

    pub async fn resume(&self) -> Result<(), TideError> {
        self.downloader
            .unpause(&self.handle)
            .await
            .map_err(|err| TideError::Internal(format!("resume failed: {err}")))
    }

    /// Stops both sessions. Takes `Arc<Self>` so callers holding the
    /// harness behind an Arc can shut it down.
    pub async fn stop(self: Arc<Self>) {
        let _ = self.downloader.stop().await;
        let _ = self.seeder.stop().await;
    }
}
