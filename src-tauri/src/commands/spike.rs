// M0.5 SPIKE — experiment IPC commands.
//
// These commands drive the M0.5 experiments from the spike frontend.
// They are clearly separated from production commands and will be
// reviewed at M1/M3/M4: some become real features, most get deleted.

use crate::app::paths::AppDirs;
use crate::commands::ipc_result;
use crate::error::TideError;
use crate::spike::events::SpikeEvent;
use crate::spike::fixtures::{self, MediaFixture};
use crate::spike::resource::{FileResource, HttpResource, SpikeResource, TorrentResource};
use crate::spike::state::{SpikeRuntimeInfo, SpikeState};
use crate::spike::torrent::SpikeTorrentStatus;
use serde::Serialize;
use std::time::Instant;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSessionDto {
    pub url: String,
    pub source: String,
    pub name: String,
}

#[tauri::command]
pub async fn spike_status(spike: State<'_, SpikeState>) -> Result<SpikeRuntimeInfo, TideError> {
    let fixture_port = spike.with_fixture_server(|s| s.port).await.ok();
    let gateway_port = spike.with_gateway(|g| g.port).await.ok();
    let torrent_running = spike.torrent().await.is_some();
    Ok(SpikeRuntimeInfo {
        fixture_port,
        gateway_port,
        torrent_running,
    })
}

#[tauri::command]
pub async fn spike_list_media() -> Result<Vec<MediaFixture>, TideError> {
    fixtures::list_media()
}

#[tauri::command]
pub async fn spike_start_fixture_server(spike: State<'_, SpikeState>) -> Result<u16, TideError> {
    let port = spike.ensure_fixture_server().await?;
    ipc_result("spike_start_fixture_server", Ok(port))
}

#[tauri::command]
pub async fn spike_fixture_slow_ms(
    spike: State<'_, SpikeState>,
    delay_ms: u32,
) -> Result<(), TideError> {
    spike
        .with_fixture_server(|server| server.set_slow_delay_ms(delay_ms))
        .await
}

/// experiment K (HTTP): throttle the fixture server to simulate a slow
/// network and observe WebView2 buffering behavior.
#[tauri::command]
pub async fn spike_start_torrent(
    spike: State<'_, SpikeState>,
    mode: String,
) -> Result<SpikeTorrentStatus, TideError> {
    let started = Instant::now();
    let dirs = AppDirs::resolve()?;
    let work_dir = dirs.root.join("spike");
    std::fs::create_dir_all(&work_dir)?;
    let torrent = match mode.as_str() {
        "single" => spike.ensure_torrent_single(work_dir).await?,
        "multi" => {
            let source = prepare_multi_source()?;
            spike.ensure_torrent_multi(source, work_dir).await?
        }
        other => {
            return Err(TideError::InvalidInput(format!(
                "unknown torrent spike mode: {other}"
            )))
        }
    };
    crate::spike::events::record(
        "torrent.setup",
        &[
            ("mode", mode),
            ("setup_ms", started.elapsed().as_millis().to_string()),
        ],
    );
    Ok(torrent.status())
}

/// experiment F: build a dedicated multi-file seed directory
/// (video + subtitle + cover + binary) from the generated fixtures.
fn prepare_multi_source() -> Result<std::path::PathBuf, TideError> {
    let dirs = AppDirs::resolve()?;
    let source = dirs.root.join("spike").join("multifile-src");
    std::fs::create_dir_all(&source)?;
    for name in [
        "torrent_video_720p.mp4",
        "subtitle.srt",
        "cover.jpg",
        "other.bin",
    ] {
        let from = fixtures::resolve_media_file(name)?;
        let to = source.join(name);
        if !to.exists() || std::fs::metadata(&to)?.len() != std::fs::metadata(&from)?.len() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(source)
}

#[tauri::command]
pub async fn spike_torrent_status(
    spike: State<'_, SpikeState>,
) -> Result<Option<SpikeTorrentStatus>, TideError> {
    Ok(spike.torrent().await.map(|t| t.status()))
}

#[tauri::command]
pub async fn spike_pause_torrent(spike: State<'_, SpikeState>) -> Result<(), TideError> {
    let torrent = spike
        .torrent()
        .await
        .ok_or_else(|| TideError::InvalidInput("torrent demo not running".into()))?;
    torrent.pause().await
}

#[tauri::command]
pub async fn spike_resume_torrent(spike: State<'_, SpikeState>) -> Result<(), TideError> {
    let torrent = spike
        .torrent()
        .await
        .ok_or_else(|| TideError::InvalidInput("torrent demo not running".into()))?;
    torrent.resume().await
}

/// Opens a gateway playback session.
///
/// kind: "file" (complete local fixture) | "http" (ranged reads from the
/// fixture server — experiment B through the gateway) | "torrent"
/// (progressive reads from the incomplete download — experiments D/E/H).
#[tauri::command]
pub async fn spike_open_playback_session(
    spike: State<'_, SpikeState>,
    kind: String,
    file_name: Option<String>,
) -> Result<PlaybackSessionDto, TideError> {
    let file_name = file_name.unwrap_or_else(|| "torrent_video_720p.mp4".to_string());
    let resource = match kind.as_str() {
        "file" => {
            let path = fixtures::resolve_media_file(&file_name)?;
            SpikeResource::File(FileResource::open(path)?)
        }
        "http" => {
            let port = spike.ensure_fixture_server().await?;
            let fixture = fixtures::resolve_media_file(&file_name)?;
            let len = std::fs::metadata(&fixture)?.len();
            SpikeResource::Http(HttpResource::new(
                std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                format!("/media/{file_name}"),
                len,
            ))
        }
        "torrent" => {
            let dirs = AppDirs::resolve()?;
            let work_dir = dirs.root.join("spike");
            let torrent = spike.ensure_torrent_single(work_dir).await?;
            let file_id = torrent
                .files
                .iter()
                .find(|f| f.name.contains(&file_name))
                .or_else(|| torrent.files.first())
                .map(|f| f.id)
                .ok_or_else(|| TideError::NotFound("torrent has no files".into()))?;
            SpikeResource::Torrent(TorrentResource::new(torrent, file_id)?)
        }
        other => {
            return Err(TideError::InvalidInput(format!(
                "unknown playback session kind: {other}"
            )))
        }
    };
    spike.ensure_gateway().await?;
    let name = file_name.clone();
    let source = resource.source_label().to_string();
    let token = spike
        .with_gateway(|gateway| gateway.create_session(resource, name.clone(), None))
        .await??;
    let url = spike
        .with_gateway(|gateway| gateway.session_url(&token))
        .await?;
    Ok(PlaybackSessionDto { url, source, name })
}

#[tauri::command]
pub async fn spike_close_playback_session(
    spike: State<'_, SpikeState>,
    token: String,
) -> Result<bool, TideError> {
    spike
        .with_gateway(|gateway| gateway.remove_session(&token))
        .await
}

/// experiment K (torrent/gateway): per-chunk artificial delay.
#[tauri::command]
pub async fn spike_gateway_slow_ms(
    spike: State<'_, SpikeState>,
    delay_ms: u32,
) -> Result<(), TideError> {
    spike
        .with_gateway(|gateway| gateway.set_slow_delay_ms(delay_ms))
        .await
}

#[tauri::command]
pub async fn spike_gateway_read_timeout_ms(
    spike: State<'_, SpikeState>,
    timeout_ms: u64,
) -> Result<(), TideError> {
    spike
        .with_gateway(|gateway| gateway.set_read_timeout_ms(timeout_ms))
        .await
}

#[tauri::command]
pub async fn spike_events(kind: Option<String>) -> Result<Vec<SpikeEvent>, TideError> {
    Ok(crate::spike::events::snapshot(kind.as_deref()))
}

#[tauri::command]
pub async fn spike_clear_events() -> Result<(), TideError> {
    crate::spike::events::clear();
    Ok(())
}

/// Stores a frontend-run experiment result (e.g. the WebView2 media
/// matrix) in the app-data spike folder and the event log.
#[tauri::command]
pub async fn spike_report_media_result(report: serde_json::Value) -> Result<(), TideError> {
    let dirs = AppDirs::resolve()?;
    let dir = dirs.root.join("spike");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("results-media.json");
    let mut entries: Vec<serde_json::Value> = if path.exists() {
        serde_json::from_slice(&std::fs::read(&path)?).unwrap_or_default()
    } else {
        Vec::new()
    };
    entries.push(report.clone());
    std::fs::write(&path, serde_json::to_vec_pretty(&entries)?)?;
    crate::spike::events::record(
        "media.result",
        &[("report", serde_json::to_string(&report).unwrap_or_default())],
    );
    Ok(())
}

#[tauri::command]
pub async fn spike_read_media_results() -> Result<Vec<serde_json::Value>, TideError> {
    let dirs = AppDirs::resolve()?;
    let path = dirs.root.join("spike").join("results-media.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_slice(&std::fs::read(&path)?).unwrap_or_default())
}

/// M0.5 SPIKE — hands-free experiment mode: when the app is launched
/// with `--spike-autorun`, the spike page runs the full experiment
/// sequence (format matrix → torrent chain → gateway playback → seek)
/// and reports measurements, so results can be collected unattended.
#[tauri::command]
pub fn spike_autorun_enabled() -> bool {
    std::env::args().any(|arg| arg == "--spike-autorun")
}

/// Lifecycle experiment (27): stops everything spike-related. The spike
/// frontend can then verify no zombie listeners remain.
#[tauri::command]
pub async fn spike_shutdown(spike: State<'_, SpikeState>) -> Result<SpikeRuntimeInfo, TideError> {
    let info = spike.shutdown_all().await?;
    crate::spike::events::record("spike.shutdown", &[]);
    Ok(info)
}
