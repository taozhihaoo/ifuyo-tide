//! TorrentEngine (M3 §5/§34): Tide's boundary over ONE librqbit Session.
//!
//! State ownership (M3 §14): Tide owns product-level task intent in
//! SQLite; librqbit owns piece/fast-resume/peer state. This wrapper is
//! the ONLY place librqbit types appear (M3 §4/§65) — the IPC and app
//! layers see `TorrentMetadata` / `TorrentStatus` / handles only.

use super::source::{
    validate_torrent_paths, validate_torrent_relative_path, TorrentMetadata, TorrentMetadataFile,
};
use crate::error::TideError;
use librqbit::{
    AddTorrent, AddTorrentOptions, ListOnlyResponse, Session, SessionOptions, TorrentStats,
};
use librqbit::{ListenerMode, ListenerOptions, ManagedTorrent};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Runtime configuration for the engine (M3 §26: network safety defaults).
#[derive(Debug, Clone)]
pub struct TorrentEngineConfig {
    /// Stable metadata copies: `<dir>/<task-id>.torrent` (M3 §6).
    pub metadata_dir: PathBuf,
    /// Production: true (real swarms). Tests: false (loopback seeder only).
    pub dht: bool,
    /// Tests only: bind the listener to 127.0.0.1.
    pub listen_loopback_only: bool,
    /// Test/bootstrap direct peers handed to every added torrent.
    pub initial_peers: Option<Vec<SocketAddr>>,
}

/// Tide's own torrent status snapshot (M3 §30): unknown data is None,
/// never fabricated zero.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentStatus {
    pub info_hash: String,
    pub state: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
    pub download_speed_bps: Option<u64>,
    pub upload_speed_bps: Option<u64>,
    pub error: Option<String>,
    pub finished: bool,
    pub paused: bool,
}

/// Outcome of adding a torrent.
pub struct AddedTorrent {
    pub info_hash: String,
    pub handle: Arc<ManagedTorrent>,
    /// True when the session already managed this info hash (M3 §17
    /// duplicate invariant: the session reuses, never duplicates).
    pub already_managed: bool,
}

/// Clone shares the underlying librqbit Session (Arc) — streaming
/// resources reach the engine through it (M4(上)).
#[derive(Clone)]
pub struct TorrentEngine {
    session: Arc<Session>,
    initial_peers: Option<Vec<SocketAddr>>,
}

impl TorrentEngine {
    /// Bound for session mutators (pause/resume/selection): librqbit
    /// awaits internal state transitions that can stall on a torrent
    /// stuck mid-add — the IPC must fail visibly, never hang.
    const MUTATOR_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

    /// Creates the single librqbit Session (M3 §34).
    ///
    /// Safety defaults (M3 §26): UPnP port forwarding OFF, no HTTP API,
    /// no Prometheus, TCP listener only (uTP experimental feature left
    /// off), DownloadOnly upload policy via the `disable-upload` cargo
    /// feature plus `disable_upload: true` — a compile-and-runtime
    /// verifiable guarantee, not a UI claim (M3 §28).
    pub async fn new(config: &TorrentEngineConfig) -> Result<TorrentEngine, TideError> {
        std::fs::create_dir_all(&config.metadata_dir)?;
        let listen_addr: SocketAddr = if config.listen_loopback_only {
            ([127, 0, 0, 1], 0).into()
        } else {
            ([0, 0, 0, 0], 0).into()
        };
        let options = SessionOptions {
            dht: if config.dht {
                Some(Default::default())
            } else {
                None
            },
            disable_trackers: !config.dht,
            listen: Some(ListenerOptions {
                listen_addr,
                mode: ListenerMode::TcpOnly,
                enable_upnp_port_forwarding: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let session = Session::new_with_opts(config.metadata_dir.clone(), options)
            .await
            .map_err(|err| {
                TideError::Internal(format!("failed to initialize torrent session: {err}"))
            })?;
        Ok(TorrentEngine {
            session,
            initial_peers: config.initial_peers.clone(),
        })
    }

    /// Metadata-only probe of `.torrent` bytes (M3 §59): nothing is added
    /// to the session, nothing is written to disk. Every file path in the
    /// metadata is validated — one hostile path rejects the whole torrent
    /// (M3 §12).
    pub async fn inspect_torrent_bytes(
        &self,
        bytes: Vec<u8>,
    ) -> Result<TorrentMetadata, TideError> {
        let list = self.list_only(bytes).await?;
        metadata_from_list_only(&list)
    }

    pub async fn list_only(&self, bytes: Vec<u8>) -> Result<ListOnlyResponse, TideError> {
        let response = self
            .session
            .add_torrent(
                AddTorrent::TorrentFileBytes(bytes.into()),
                Some(AddTorrentOptions {
                    list_only: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|err| TideError::InvalidInput(format!("torrent metadata rejected: {err}")))?;
        match response {
            librqbit::AddTorrentResponse::ListOnly(list) => Ok(list),
            _ => Err(TideError::Internal(
                "list-only add returned unexpected response".into(),
            )),
        }
    }

    /// Adds a `.torrent` (raw bytes) to the session. Paths inside the
    /// metadata are validated first — one hostile path rejects the whole
    /// torrent (M3 §12).
    ///
    /// `storage_root` is the output folder. `initial_peers` (tests /
    /// bootstrap) is forwarded to librqbit. `paused` adds without
    /// starting (metadata/file-selection flow, M3 §23).
    pub async fn add_torrent_bytes(
        &self,
        bytes: Vec<u8>,
        storage_root: &Path,
        only_files: Option<Vec<usize>>,
        initial_peers: Option<Vec<SocketAddr>>,
        paused: bool,
    ) -> Result<AddedTorrent, TideError> {
        // Validate every metadata path BEFORE anything is written.
        let list = self.list_only(bytes.clone()).await?;
        let paths: Vec<String> = file_paths_from_list(&list);
        validate_torrent_paths(&paths)?;
        self.add(
            AddTorrent::TorrentFileBytes(bytes.into()),
            storage_root,
            only_files,
            initial_peers.or_else(|| self.initial_peers.clone()),
            paused,
        )
        .await
    }

    /// Adds a magnet task. Metadata resolves asynchronously (M3 §7);
    /// `wait_metadata` blocks until it is available.
    pub async fn add_magnet(
        &self,
        uri: String,
        storage_root: &Path,
        // Applies once the metadata resolves. Some(empty) = select
        // nothing: the magnet-intake flow uses this so nothing
        // downloads before the user picks files.
        only_files: Option<Vec<usize>>,
        initial_peers: Option<Vec<SocketAddr>>,
        paused: bool,
    ) -> Result<AddedTorrent, TideError> {
        let (uri, info_hash, _) = super::source::validate_magnet_uri(&uri)?;
        let handle = self
            .add(
                AddTorrent::Url(uri.clone().into()),
                storage_root,
                only_files,
                initial_peers.or_else(|| self.initial_peers.clone()),
                paused,
            )
            .await?;
        Ok(AddedTorrent {
            info_hash,
            handle: handle.handle,
            already_managed: handle.already_managed,
        })
    }

    async fn add(
        &self,
        add: AddTorrent<'static>,
        storage_root: &Path,
        only_files: Option<Vec<usize>>,
        initial_peers: Option<Vec<SocketAddr>>,
        paused: bool,
    ) -> Result<AddedTorrent, TideError> {
        let response = self
            .session
            .add_torrent(
                add,
                Some(AddTorrentOptions {
                    output_folder: Some(storage_root.to_string_lossy().to_string()),
                    only_files,
                    initial_peers,
                    paused,
                    // Re-adding over existing files resumes; librqbit
                    // re-verifies pieces (M3 §22 corruption recovery).
                    overwrite: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|err| TideError::Internal(format!("failed to add torrent: {err}")))?;
        let (info_hash, handle, already_managed) = match response {
            librqbit::AddTorrentResponse::AlreadyManaged(_, handle) => {
                (handle.info_hash().as_string(), handle, true)
            }
            librqbit::AddTorrentResponse::Added(_, handle) => {
                (handle.info_hash().as_string(), handle, false)
            }
            librqbit::AddTorrentResponse::ListOnly(_) => {
                return Err(TideError::Internal(
                    "unexpected list-only response for a real add".into(),
                ));
            }
        };
        handle
            .wait_until_initialized()
            .await
            .map_err(|err| TideError::Internal(format!("torrent metadata failed: {err}")))?;
        Ok(AddedTorrent {
            info_hash,
            handle,
            already_managed,
        })
    }

    /// Blocks until the torrent's metadata is available (magnet flow,
    /// M3 §23). Bounded by `timeout`.
    pub async fn wait_metadata(
        &self,
        handle: &Arc<ManagedTorrent>,
        timeout: Duration,
    ) -> Result<TorrentMetadata, TideError> {
        tokio::time::timeout(timeout, handle.wait_until_initialized())
            .await
            .map_err(|_| TideError::Internal("torrent metadata resolution timed out".into()))?
            .map_err(|err| TideError::Internal(format!("torrent metadata failed: {err}")))?;
        self.metadata_of(handle)
    }

    /// Snapshot of the metadata (files with offsets/sizes, path-validated).
    pub fn metadata_of(&self, handle: &Arc<ManagedTorrent>) -> Result<TorrentMetadata, TideError> {
        handle
            .with_metadata(|metadata| {
                let mut files = Vec::new();
                for details in metadata.info.iter_file_details_ext() {
                    let raw = details.details.filename.to_pathbuf();
                    let path = match validate_torrent_relative_path(&raw.to_string_lossy()) {
                        Ok(p) => p,
                        Err(err) => return Err(err),
                    };
                    files.push(TorrentMetadataFile {
                        index: files.len(),
                        relative_path: path.to_string_lossy().to_string(),
                        size: details.details.len,
                        offset: details.offset,
                    });
                }
                let total_size: u64 = files.iter().map(|f| f.size).sum();
                Ok(TorrentMetadata {
                    info_hash: handle.info_hash().as_string(),
                    name: metadata
                        .info
                        .name()
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    total_size,
                    piece_length: metadata.info.info().piece_length,
                    files,
                })
            })
            .map_err(|err| TideError::Internal(format!("metadata access failed: {err}")))?
    }

    pub fn handle_for(&self, info_hash: &str) -> Option<Arc<ManagedTorrent>> {
        // dyn Iterator is unsized: adapters like `find` require Sized, so
        // a for-loop (via the &mut I IntoIterator impl) is used here.
        self.session.with_torrents(|torrents| {
            for (_, t) in &mut *torrents {
                if t.info_hash().as_string() == info_hash {
                    return Some((*t).clone());
                }
            }
            None
        })
    }

    pub async fn pause(&self, handle: &Arc<ManagedTorrent>) -> Result<(), TideError> {
        match tokio::time::timeout(Self::MUTATOR_TIMEOUT, self.session.pause(handle)).await {
            Ok(result) => {
                result.map_err(|err| TideError::Internal(format!("torrent pause failed: {err}")))
            }
            Err(_) => Err(TideError::Internal(
                "torrent pause timed out (engine busy)".into(),
            )),
        }
    }

    pub async fn resume(&self, handle: &Arc<ManagedTorrent>) -> Result<(), TideError> {
        match tokio::time::timeout(Self::MUTATOR_TIMEOUT, self.session.unpause(handle)).await {
            Ok(result) => {
                result.map_err(|err| TideError::Internal(format!("torrent resume failed: {err}")))
            }
            Err(_) => Err(TideError::Internal(
                "torrent resume timed out (engine busy)".into(),
            )),
        }
    }

    pub async fn set_only_files(
        &self,
        handle: &Arc<ManagedTorrent>,
        files: &[usize],
    ) -> Result<(), TideError> {
        let set: std::collections::HashSet<usize> = files.iter().copied().collect();
        match tokio::time::timeout(
            Self::MUTATOR_TIMEOUT,
            self.session.update_only_files(handle, &set),
        )
        .await
        {
            Ok(result) => {
                result.map_err(|err| TideError::Internal(format!("file selection failed: {err}")))
            }
            Err(_) => Err(TideError::Internal(
                "file selection timed out (engine busy)".into(),
            )),
        }
    }

    /// Removes the torrent from the session. `delete_files` also removes
    /// the downloaded data (M3 §20: three distinct semantics live at the
    /// command layer — this is the engine-level primitive).
    pub async fn remove(
        &self,
        handle: &Arc<ManagedTorrent>,
        delete_files: bool,
    ) -> Result<(), TideError> {
        self.session
            .delete(handle.info_hash().into(), delete_files)
            .await
            .map_err(|err| TideError::Internal(format!("torrent remove failed: {err}")))
    }

    /// Whether torrent metadata has been resolved for this handle.
    pub fn metadata_resolved(&self, handle: &Arc<ManagedTorrent>) -> bool {
        handle.with_metadata(|_| ()).is_ok()
    }

    pub fn stats(&self, handle: &Arc<ManagedTorrent>) -> TorrentStats {
        handle.stats()
    }

    /// M7(上) §25/§12: session-level download rate via librqbit's
    /// PUBLIC ratelimits API — no private modules. `None` = unlimited.
    pub fn set_download_rate_bps(&self, bps: Option<u64>) {
        let capped = bps
            .and_then(|b| u32::try_from(b.min(u32::MAX as u64)).ok())
            .and_then(core::num::NonZeroU32::new);
        self.session.ratelimits.set_download_bps(capped);
    }

    /// M7(上) §41: the session's live download throughput (bytes/sec).
    /// Live per-torrent download speed (bytes/sec); None while the
    /// torrent has no live stats yet (paused / initializing).
    pub fn live_download_speed_bps(&self, handle: &Arc<ManagedTorrent>) -> Option<u64> {
        handle.stats().live.map(|l| l.download_speed.as_bytes())
    }

    pub fn live_download_bps(&self) -> u64 {
        let total = std::cell::Cell::new(0u64);
        self.session.with_torrents(|torrents| {
            for (_, torrent) in torrents {
                if let Some(live) = &torrent.stats().live {
                    total.set(total.get() + live.download_speed.as_bytes());
                }
            }
        });
        total.get()
    }

    pub async fn shutdown(&self) {
        self.session.stop().await;
    }
}

/// Extracts the (already validated at inspect time) relative file paths.
fn file_paths_from_list(list: &ListOnlyResponse) -> Vec<String> {
    let mut out = Vec::new();
    for details in list.info.iter_file_details_ext() {
        out.push(
            details
                .details
                .filename
                .to_pathbuf()
                .to_string_lossy()
                .to_string(),
        );
    }
    out
}

/// Converts a list-only response into Tide's metadata model, validating
/// every path (one hostile path rejects the whole torrent).
fn metadata_from_list_only(list: &ListOnlyResponse) -> Result<TorrentMetadata, TideError> {
    let mut files = Vec::new();
    for details in list.info.iter_file_details_ext() {
        let raw = details.details.filename.to_pathbuf();
        let path = validate_torrent_relative_path(&raw.to_string_lossy())?;
        files.push(TorrentMetadataFile {
            index: files.len(),
            relative_path: path.to_string_lossy().to_string(),
            size: details.details.len,
            offset: details.offset,
        });
    }
    let paths: Vec<String> = files.iter().map(|f| f.relative_path.clone()).collect();
    validate_torrent_paths(&paths)?;
    Ok(TorrentMetadata {
        info_hash: list.info_hash.as_string(),
        name: list.info.name().map(|n| n.to_string()).unwrap_or_default(),
        total_size: files.iter().map(|f| f.size).sum(),
        piece_length: list.info.info().piece_length,
        files,
    })
}
