//! Launch-request routing (M8 §73/§74/§75/§46).
//!
//! Requests that arrive BEFORE the application layer is ready are
//! kept in an in-memory pending queue (§74) and dispatched by the
//! frontend once ready — no launch request is silently lost. Routing
//! emits `app://launch-request` with the validated request; the UI's
//! Add-Source flow performs the actual task creation after user
//! confirmation (§46: LaunchRequest ≠ Task).

use super::launch::LaunchRequest;
use std::sync::Mutex;
use tauri::Emitter;

pub struct PendingLaunchQueue {
    queue: Mutex<Vec<LaunchRequest>>,
}

impl Default for PendingLaunchQueue {
    fn default() -> Self {
        PendingLaunchQueue {
            queue: Mutex::new(Vec::new()),
        }
    }
}

impl PendingLaunchQueue {
    pub fn new() -> PendingLaunchQueue {
        Self::default()
    }

    /// §74: store a request that arrived before the frontend was ready.
    pub fn push(&self, request: LaunchRequest) {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(request);
    }

    /// Drains all pending requests (§75: all of them — none dropped).
    pub fn drain(&self) -> Vec<LaunchRequest> {
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *queue)
    }

    pub fn len(&self) -> usize {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Routes a validated request to the application layer: emits
/// `app://launch-request` for the Add-Source confirmation flow (§45/§46)
/// and Focus requests activate the main window instead (§76).
pub fn route_launch_request(
    app: &tauri::AppHandle,
    queue: &PendingLaunchQueue,
    request: LaunchRequest,
) -> Result<(), crate::error::TideError> {
    if matches!(request.kind, super::launch::LaunchKind::Focus) {
        crate::platform::tray::activate_main_window(app);
        return Ok(());
    }
    // The window may not exist yet (early launch): queue for the
    // frontend drain, but still try a live emit for the running case.
    if app
        .emit(
            "app://launch-request",
            serde_json::to_value(&request).unwrap_or_default(),
        )
        .is_err()
    {
        queue.push(request);
    }
    Ok(())
}
