//! PlaybackSession manager (M4(上) §51–§53/§70, M4(下) §102/§146/§147).
//!
//! PlaybackSession is a SEPARATE lifecycle from DownloadTask (M4(上)
//! §52/§53): pausing playback never pauses the download; playback
//! ending never completes the task. Gateway tokens are RUNTIME-ONLY —
//! never persisted (M4(上) §70); they are regenerated per app run.
//! Sessions track their owning task so task deletion can invalidate
//! them (M4(下) §102 — chosen policy: session invalidated, never a
//! dangling path).

use super::demand::DemandScheduler;
use super::gateway::StreamingGateway;
use super::resource::{PlaybackCapability, StreamingResource};
use crate::error::TideError;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Idle-session reaping age (M4(上) §55).
const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSessionInfo {
    pub session_id: String,
    pub url: String,
    pub size: u64,
    pub content_type: String,
    pub source_kind: &'static str,
    /// Bytes durably readable behind the session right now (M4(下)
    /// §149: backend availability — refreshed on get_playback_info).
    pub available_bytes: u64,
    /// What this resource can do (M4(下) §146/§147): formed once at
    /// session creation; the player branches on this, never on kind.
    pub capability: PlaybackCapability,
    pub created_at_ms: u64,
}

struct PlaybackSession {
    info: PlaybackSessionInfo,
    /// Owning task id (download id / torrent task id) for §102
    /// invalidation on task deletion.
    owner: Option<String>,
}

/// Owns playback sessions and the gateway. One manager per app.
pub struct PlaybackManager {
    gateway: StreamingGateway,
    scheduler: Arc<DemandScheduler>,
    sessions: Mutex<HashMap<String, PlaybackSession>>,
}

impl PlaybackManager {
    pub async fn new(scheduler: Arc<DemandScheduler>) -> Result<PlaybackManager, TideError> {
        let gateway = StreamingGateway::start(scheduler.clone()).await?;
        Ok(PlaybackManager {
            gateway,
            scheduler,
            sessions: Mutex::new(HashMap::new()),
        })
    }

    /// Opens a playback session: registers the resource with the gateway
    /// and returns the loopback URL for the player element (M4(上) §69).
    /// The frontend gets ONLY the URL — never a filesystem path (§59).
    pub fn open(
        &self,
        resource: StreamingResource,
        content_type: String,
        owner: Option<String>,
    ) -> Result<PlaybackSessionInfo, TideError> {
        let size = resource.len();
        let available_bytes = resource.available_len();
        let capability = resource.capability();
        let source_kind = resource.source_label();
        let token = self.gateway.create_session(resource, None)?;
        let info = PlaybackSessionInfo {
            session_id: token.clone(),
            url: self.gateway.session_url(&token),
            size,
            content_type,
            source_kind,
            available_bytes,
            capability,
            created_at_ms: crate::core::time::now_unix_ms() as u64,
        };
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                info.session_id.clone(),
                PlaybackSession {
                    info: info.clone(),
                    owner,
                },
            );
        crate::diagnostics::info(
            "playback.opened",
            &[("session_id", &info.session_id), ("source", info.source_kind)],
        );
        Ok(info)
    }

    /// Stored session metadata with a LIVE availability refresh
    /// (M4(上) §68 `get_playback_info`, M4(下) §149).
    pub async fn status(&self, session_id: &str) -> Option<PlaybackSessionInfo> {
        let mut info = self.info(session_id)?;
        if let Some(available) = self.gateway.session_available_bytes(session_id).await {
            info.available_bytes = available;
        }
        Some(info)
    }

    /// Stored session metadata (no availability refresh).
    pub fn info(&self, session_id: &str) -> Option<PlaybackSessionInfo> {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(session_id)
            .map(|s| s.info.clone())
    }

    pub fn close(&self, session_id: &str) -> bool {
        let removed = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            sessions.remove(session_id).is_some()
        };
        // Revoke gateway access too: a stopped session URL must stop
        // serving bytes (M4(上) §53/§70, M4(下) §116), and its queued
        // demands are obsolete (§80).
        self.gateway.remove_session(session_id);
        self.scheduler.cancel_resource(session_id);
        if removed {
            crate::diagnostics::info(
                "playback.closed",
                &[("session_id", session_id)],
            );
        }
        removed
    }

    /// §102: the owning task is gone — invalidate every session it
    /// opened so no session outlives its data.
    pub fn close_for_owner(&self, owner: &str) -> usize {
        let ids: Vec<String> = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            sessions
                .values()
                .filter(|s| s.owner.as_deref() == Some(owner))
                .map(|s| s.info.session_id.clone())
                .collect()
        };
        let count = ids.len();
        for id in &ids {
            self.close(id);
        }
        if count > 0 {
            crate::diagnostics::info(
                "playback.invalidated_for_task",
                &[("sessions", &count.to_string())],
            );
        }
        count
    }

    /// Seek notification: demands queued for the old position are
    /// obsolete (M4(上) §80) — cancel them so new-range demands jump
    /// the queue. Not an error if the session is already gone.
    pub fn notify_seek(&self, session_id: &str) {
        self.scheduler.cancel_resource(session_id);
    }

    pub fn session_count(&self) -> usize {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    pub fn gateway_port(&self) -> u16 {
        self.gateway.port
    }

    /// Per-read resource wait timeout (M4(上) §96/§97); passthrough for
    /// tests and tuning.
    pub fn set_read_timeout_ms(&self, ms: u64) {
        self.gateway.set_read_timeout_ms(ms);
    }

    /// Idle-session reaping (M4(上) §55): revokes gateway access and
    /// queued demands along with the manager entry.
    pub fn reap_expired(&self) -> usize {
        let now = crate::core::time::now_unix_ms() as u64;
        let expired: Vec<String> = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            sessions
                .values()
                .filter(|s| s.info.created_at_ms + (SESSION_TTL.as_millis() as u64) < now)
                .map(|s| s.info.session_id.clone())
                .collect()
        };
        for id in &expired {
            self.close(id);
        }
        expired.len()
    }
}
