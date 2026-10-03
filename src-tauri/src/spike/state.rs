// M0.5 SPIKE — spike-side managed state.
//
// Deliberately separate from the production `AppState`: spike handles
// (fixture server, gateway, torrent harness) are lazily started by spike
// commands and shut down together, so experiments can't disturb the
// production pipeline and vice versa. All accessors are async and run
// on the tauri async runtime — no blocking locks.

use super::fixture_server::{self, FixtureServer};
use super::gateway::Gateway;
use super::torrent::TorrentSpike;
use crate::error::TideError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct SpikeState {
    fixture_server: Mutex<Option<FixtureServer>>,
    gateway: Mutex<Option<Gateway>>,
    torrent: Mutex<Option<Arc<TorrentSpike>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeRuntimeInfo {
    pub fixture_port: Option<u16>,
    pub gateway_port: Option<u16>,
    pub torrent_running: bool,
}

impl SpikeState {
    pub async fn ensure_fixture_server(&self) -> Result<u16, TideError> {
        let mut guard = self.fixture_server.lock().await;
        if guard.is_none() {
            *guard = Some(fixture_server::start().await?);
        }
        Ok(guard.as_ref().expect("just set").port)
    }

    pub async fn with_fixture_server<T>(
        &self,
        f: impl FnOnce(&FixtureServer) -> T,
    ) -> Result<T, TideError> {
        let guard = self.fixture_server.lock().await;
        guard
            .as_ref()
            .map(f)
            .ok_or_else(|| TideError::InvalidInput("fixture server not running".into()))
    }

    pub async fn ensure_gateway(&self) -> Result<u16, TideError> {
        let mut guard = self.gateway.lock().await;
        if guard.is_none() {
            *guard = Some(Gateway::start().await?);
        }
        Ok(guard.as_ref().expect("just set").port)
    }

    pub async fn with_gateway<T>(&self, f: impl FnOnce(&Gateway) -> T) -> Result<T, TideError> {
        let guard = self.gateway.lock().await;
        guard
            .as_ref()
            .map(f)
            .ok_or_else(|| TideError::InvalidInput("gateway not running".into()))
    }

    /// Starts the single-file torrent demo if not running; returns the
    /// running harness either way.
    pub async fn ensure_torrent_single(
        &self,
        work_dir: PathBuf,
    ) -> Result<Arc<TorrentSpike>, TideError> {
        let mut guard = self.torrent.lock().await;
        if let Some(spike) = guard.as_ref() {
            return Ok(spike.clone());
        }
        let media = super::fixtures::resolve_media_file("torrent_video_720p.mp4")?;
        let spike = Arc::new(TorrentSpike::start_single(&media, &work_dir).await?);
        *guard = Some(spike.clone());
        Ok(spike)
    }

    pub async fn ensure_torrent_multi(
        &self,
        source_dir: PathBuf,
        work_dir: PathBuf,
    ) -> Result<Arc<TorrentSpike>, TideError> {
        let mut guard = self.torrent.lock().await;
        if let Some(spike) = guard.as_ref() {
            return Ok(spike.clone());
        }
        let spike = Arc::new(TorrentSpike::start_multi(&source_dir, &work_dir).await?);
        *guard = Some(spike.clone());
        Ok(spike)
    }

    pub async fn torrent(&self) -> Option<Arc<TorrentSpike>> {
        self.torrent.lock().await.clone()
    }

    /// Lifecycle experiment: everything spike-related stops together.
    /// Returns all-none info so callers can verify no zombie listeners.
    pub async fn shutdown_all(&self) -> Result<SpikeRuntimeInfo, TideError> {
        if let Some(spike) = self.torrent.lock().await.take() {
            spike.stop().await;
        }
        if let Some(gateway) = self.gateway.lock().await.take() {
            drop(gateway);
        }
        if let Some(fixture) = self.fixture_server.lock().await.take() {
            drop(fixture);
        }
        Ok(SpikeRuntimeInfo {
            fixture_port: None,
            gateway_port: None,
            torrent_running: false,
        })
    }
}
