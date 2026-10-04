//! DownloadEngine (M1 §1, §3): the executor that moves an HTTP resource
//! to a local file — state machine, resume decisions, retry, progress
//! events, cancellation. UI never touches this directly (DTOs/events
//! only); M2 will attach persistence at the checkpoint seams.

use super::checksum::ChecksumWriter;
use super::file_sink::{classify_io_error, FileSink, OverwritePolicy};
use super::http_range::{parse_content_range_header, ContentRangeParse};
use super::progress::{compute_progress, SpeedTracker};
use super::request::{retry_delay_ms, DownloadRequest, MAX_RETRY_ATTEMPTS};
use super::state::{DownloadLifecycle, DownloadPhase};
use super::url::{validate_download_url, DownloadFailure};
use super::validator::{decide_resume, ResponseValidators};
use super::HttpTransport;
use super::{DownloadRepository, DownloadTaskRecord};
use crate::error::TideError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::{Notify, Semaphore};

/// Progress event throttle (M1 §27).
const EVENT_INTERVAL: Duration = Duration::from_millis(200);
/// Simple concurrency gate (M1 §42): a bound, not a scheduler.
const MAX_CONCURRENT_DOWNLOADS: usize = 3;

pub struct DownloadEngine {
    downloads: Mutex<HashMap<String, Arc<DownloadShared>>>,
    /// M2: persistence boundary — the engine writes checkpoints and
    /// state transitions through the repository, never through SQL.
    repo: Arc<DownloadRepository>,
    /// M6(上) §26: fired once per completed download (after finalize
    /// + persist) so higher layers (library import) can react without
    /// the engine knowing about them.
    on_complete: Mutex<Option<Arc<dyn Fn(&DownloadShared) + Send + Sync>>>,
}

pub struct DownloadShared {
    pub request: DownloadRequest,
    inner: Mutex<DownloadInner>,
    pause_flag: AtomicBool,
    cancel_flag: AtomicBool,
    wake: Notify,
    transport: HttpTransport,
    repo: Arc<DownloadRepository>,
    /// M4(下) §115: event-driven durable-length notification. Fired
    /// AFTER a chunk is flushed to the `.part` so streaming readers
    /// wake on real availability instead of polling (write → flush →
    /// update availability → wake readers, §112).
    durable_len_tx: tokio::sync::watch::Sender<u64>,
    /// M6(上) §26: fired once when this download completes (after
    /// finalize + persist). Copied from the engine-level hook at
    /// start() — `run_with_slot` is a free function and has no engine
    /// access.
    on_complete: Mutex<Option<Arc<dyn Fn(&DownloadShared) + Send + Sync>>>,
}

#[derive(Debug, Clone)]
struct DownloadInner {
    lifecycle: DownloadLifecycle,
    phase: DownloadPhase,
    downloaded: u64,
    total: Option<u64>,
    validators: ResponseValidators,
    resolved_filename: Option<String>,
    part_path: Option<PathBuf>,
    final_path: Option<PathBuf>,
    error: Option<(String, String)>,
    attempts: u32,
}

impl DownloadShared {
    fn new(request: DownloadRequest, repo: Arc<DownloadRepository>) -> DownloadShared {
        let (durable_len_tx, _) = tokio::sync::watch::channel(0u64);
        DownloadShared {
            request,
            inner: Mutex::new(DownloadInner {
                lifecycle: DownloadLifecycle::Created,
                phase: DownloadPhase::Preparing,
                downloaded: 0,
                total: None,
                validators: ResponseValidators::default(),
                resolved_filename: None,
                part_path: None,
                final_path: None,
                error: None,
                attempts: 0,
            }),
            pause_flag: AtomicBool::new(false),
            cancel_flag: AtomicBool::new(false),
            wake: Notify::new(),
            transport: HttpTransport::new().expect("static config"),
            repo,
            durable_len_tx,
            on_complete: Mutex::new(None),
        }
    }

    /// Reconstructs runtime state from a persisted record (M2 §42):
    /// no runtime object is deserialized — everything is rebuilt from
    /// the logical record and re-validated by the caller.
    pub(crate) fn from_record(
        request: DownloadRequest,
        repo: Arc<DownloadRepository>,
        record: &DownloadTaskRecord,
        lifecycle: DownloadLifecycle,
    ) -> DownloadShared {
        let validators = ResponseValidators::from_headers(
            record.etag.as_deref(),
            record.last_modified.as_deref(),
        );
        let (durable_len_tx, _) = tokio::sync::watch::channel(record.downloaded_bytes);
        DownloadShared {
            request,
            inner: Mutex::new(DownloadInner {
                lifecycle,
                phase: DownloadPhase::Preparing,
                downloaded: record.downloaded_bytes,
                total: record.total_bytes,
                validators,
                resolved_filename: Some(record.filename.clone()),
                part_path: Some(record.part_path.clone()),
                final_path: Some(record.final_path.clone()),
                error: record
                    .last_error_code
                    .as_deref()
                    .zip(record.last_error_message.as_deref())
                    .map(|(code, message)| (code.to_string(), message.to_string())),
                attempts: 0,
            }),
            pause_flag: AtomicBool::new(false),
            cancel_flag: AtomicBool::new(false),
            wake: Notify::new(),
            transport: HttpTransport::new().expect("static config"),
            repo,
            durable_len_tx,
            on_complete: Mutex::new(None),
        }
    }

    /// M6 §26: sets this download's completion hook (copied from the
    /// engine by `start`).
    pub(crate) fn set_on_complete(
        &self,
        hook: Arc<dyn Fn(&DownloadShared) + Send + Sync>,
    ) {
        *self
            .on_complete
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(hook);
    }

    /// M6 §26: fires the completion hook (best-effort; a panicking or
    /// failing hook must never affect the completed download).
    pub(crate) fn fire_on_complete(&self) {
        let hook = self
            .on_complete
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(hook) = hook {
            hook(self);
        }
    }

    /// Subscribes to durable-length changes of the `.part` (M4(下)
    /// §115). The current value equals the flushed byte count.
    pub fn subscribe_durable_len(&self) -> tokio::sync::watch::Receiver<u64> {
        self.durable_len_tx.subscribe()
    }

    /// Fires the durable-length notification (engine calls this after
    /// every flushed chunk). A dropped receiver is not an error.
    pub fn notify_durable_len(&self, len: u64) {
        let _ = self.durable_len_tx.send(len);
    }

    /// Persisted checkpoint write (M2 §20-§23). Called AFTER bytes are in
    /// the part file so the DB ≤ durable-file invariant holds. A failed
    /// checkpoint is logged and swallowed: a stale DB row is exactly the
    /// state startup reconciliation fixes.
    fn checkpoint(&self, downloaded: u64, total: Option<u64>) {
        let validators = self.lock_inner().validators.clone();
        let checkpoint = crate::persistence::repositories::DownloadCheckpoint {
            downloaded_bytes: downloaded,
            total_bytes: total,
            etag: validators.etag.clone(),
            last_modified: validators.last_modified.clone(),
        };
        if let Err(err) = self
            .repo
            .update_checkpoint(&checkpoint, &self.request.download_id)
        {
            crate::diagnostics::warn(
                "download.checkpoint_failed",
                &[
                    ("download_id", &self.request.download_id),
                    ("code", err.code()),
                ],
            );
        }
    }

    /// Persisted state transition (M2 §66): single write-through per
    /// lifecycle change; failures logged, never propagated into the
    /// download itself.
    fn persist_state(
        &self,
        state: &str,
        downloaded: u64,
        error: Option<(&str, &str)>,
        recovery_reason: Option<&str>,
    ) {
        let completed_at = if state == "completed" {
            Some(crate::core::time::now_unix_ms())
        } else {
            None
        };
        if let Err(err) = self.repo.update_state(
            &self.request.download_id,
            state,
            downloaded,
            error,
            recovery_reason,
            completed_at,
        ) {
            crate::diagnostics::warn(
                "download.persist_state_failed",
                &[
                    ("download_id", &self.request.download_id),
                    ("code", err.code()),
                ],
            );
        }
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, DownloadInner> {
        // Poison recovery: DownloadInner is plain data; a panicked task
        // must not brick control commands.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn status(&self) -> DownloadStatus {
        let inner = self.lock_inner();
        DownloadStatus {
            lifecycle: inner.lifecycle,
            phase: inner.phase,
            downloaded: inner.downloaded,
            total: inner.total,
            validators: inner.validators.clone(),
            resolved_filename: inner.resolved_filename.clone(),
            part_path: inner.part_path.clone(),
            final_path: inner.final_path.clone(),
            attempts: inner.attempts,
            error: inner.error.clone(),
        }
    }

    fn request_pause(&self) {
        self.pause_flag.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
    }

    fn request_cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
    }

    fn take_pause(&self) -> bool {
        self.pause_flag.swap(false, Ordering::SeqCst)
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    pub fn transport(&self) -> &HttpTransport {
        &self.transport
    }

    fn flags_set(&self) -> bool {
        self.pause_flag.load(Ordering::SeqCst) || self.is_cancelled()
    }
}

#[derive(Debug, Clone)]
pub struct DownloadStatus {
    pub lifecycle: DownloadLifecycle,
    pub phase: DownloadPhase,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub validators: ResponseValidators,
    pub resolved_filename: Option<String>,
    pub part_path: Option<PathBuf>,
    pub final_path: Option<PathBuf>,
    pub attempts: u32,
    pub error: Option<(String, String)>,
}

impl DownloadEngine {
    /// `repo`: the M2 persistence boundary (own SQLite connection).
    pub fn new(repo: Arc<DownloadRepository>) -> DownloadEngine {
        DownloadEngine {
            downloads: Mutex::new(HashMap::new()),
            repo,
            on_complete: Mutex::new(None),
        }
    }

    /// Registers the completion hook (M6 §26). At most one hook —
    /// application wiring decides what happens, the engine stays
    /// library-agnostic. Copied into every download started after
    /// this call.
    pub fn set_on_complete(&self, hook: Arc<dyn Fn(&DownloadShared) + Send + Sync>) {
        *self
            .on_complete
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(hook);
    }

    fn engine_on_complete(
        &self,
    ) -> Option<Arc<dyn Fn(&DownloadShared) + Send + Sync>> {
        self.on_complete
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Registers and starts a download; returns its shared handle.
    /// The task is persisted first (M2): a crash between insert and
    /// first byte leaves a valid zero-progress record that startup
    /// recovery classifies as restartable.
    pub async fn start(
        &self,
        request: DownloadRequest,
        app: Option<tauri::AppHandle>,
    ) -> Result<Arc<DownloadShared>, TideError> {
        let mut downloads = self
            .downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let shared = Arc::new(DownloadShared::new(request, self.repo.clone()));
        let id = shared.request.download_id.clone();
        let now = crate::core::time::now_unix_ms();
        // Persisted lifecycle: "running" is the intent; startup recovery
        // reclassifies a stale "running" row as resumable (M2 §8).
        let record = crate::persistence::repositories::DownloadTaskRecord {
            id: id.clone(),
            source_url: shared.request.url.as_str().to_string(),
            allow_private: shared.request.url.private_allowed,
            destination_dir: shared.request.destination_dir.clone(),
            filename: shared
                .request
                .filename
                .clone()
                .unwrap_or_else(|| "download.bin".to_string()),
            // Placeholder paths until the first response resolves them
            // (update_resolved_paths overwrites these).
            part_path: shared.request.destination_dir.join(format!("{id}.part")),
            final_path: shared
                .request
                .destination_dir
                .join(format!("{id}.download")),
            overwrite_policy: match shared.request.overwrite_policy {
                OverwritePolicy::Fail => "fail".to_string(),
                OverwritePolicy::Overwrite => "overwrite".to_string(),
                OverwritePolicy::AutoRename => "auto_rename".to_string(),
            },
            state: "running".to_string(),
            total_bytes: None,
            downloaded_bytes: 0,
            content_type: None,
            etag: None,
            last_modified: None,
            checksum_algorithm: shared
                .request
                .expected_checksum
                .as_ref()
                .map(|(algo, _)| algo.as_str().to_string()),
            checksum_expected: shared
                .request
                .expected_checksum
                .as_ref()
                .map(|(_, hex)| hex.clone()),
            retry_count: 0,
            last_error_code: None,
            last_error_message: None,
            recovery_reason: None,
            created_at: now,
            updated_at: now,
            started_at: Some(now),
            completed_at: None,
        };
        if let Some(hook) = self.engine_on_complete() {
            shared.set_on_complete(hook);
        }
        self.repo.insert(&record)?;
        downloads.insert(id.clone(), shared.clone());

        let task_shared = shared.clone();
        tokio::spawn(async move {
            if let Err(err) = run_with_slot(&task_shared, app).await {
                let mut inner = task_shared.lock_inner();
                inner.lifecycle = DownloadLifecycle::Failed;
                inner.error = Some((err.code().to_string(), err.message()));
                drop(inner);
                task_shared.persist_state(
                    "failed",
                    task_shared.lock_inner().downloaded,
                    Some((err.code(), &err.message())),
                    None,
                );
            }
        });
        Ok(shared)
    }

    /// Registers a recovered persisted task as an in-memory entry
    /// WITHOUT starting it (M2 §69: paused stays paused, failed stays
    /// failed, interrupted becomes resumable-but-idle until the user
    /// resumes).
    pub fn register_restored(&self, shared: Arc<DownloadShared>) {
        let id = shared.request.download_id.clone();
        self.downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, shared);
    }

    pub fn repo(&self) -> &Arc<DownloadRepository> {
        &self.repo
    }

    pub async fn get(&self, download_id: &str) -> Option<Arc<DownloadShared>> {
        self.downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(download_id)
            .cloned()
    }

    /// Sync map lookup (the map is a plain mutex): used by the playback
    /// resolver to grab the durable-len watch without an async hop.
    pub fn get_sync(&self, download_id: &str) -> Option<Arc<DownloadShared>> {
        self.downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(download_id)
            .cloned()
    }

    pub async fn list(&self) -> Vec<Arc<DownloadShared>> {
        self.downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }

    pub async fn pause(&self, download_id: &str) -> Result<(), TideError> {
        let entry = self
            .get(download_id)
            .await
            .ok_or_else(|| TideError::NotFound(format!("download not found: {download_id}")))?;
        if matches!(entry.status().lifecycle, DownloadLifecycle::Running) {
            entry.request_pause();
        }
        Ok(())
    }

    /// Builds a runtime entry from a persisted record with security
    /// re-validation (M2 §36): the database is not a security boundary.
    pub fn shared_from_record(
        &self,
        record: &DownloadTaskRecord,
    ) -> Result<Arc<DownloadShared>, TideError> {
        let validated = validate_download_url(&record.source_url, record.allow_private)?;
        let mut request = DownloadRequest::new(
            validated,
            record.destination_dir.clone(),
            None,
            OverwritePolicy::parse(&record.overwrite_policy)?,
            None,
        )?;
        request.download_id = record.id.clone();
        let lifecycle = DownloadLifecycle::from_state_str(&record.state);
        Ok(Arc::new(DownloadShared::from_record(
            request,
            self.repo.clone(),
            record,
            lifecycle,
        )))
    }

    /// Resumes a paused download. Works identically for M1 in-session
    /// pauses and M2 restored tasks: the runtime (HTTP request, sink) is
    /// rebuilt from the persisted/derived state (M2 §42/§108).
    pub async fn resume(
        &self,
        download_id: &str,
        app: Option<tauri::AppHandle>,
    ) -> Result<(), TideError> {
        let entry = self
            .get(download_id)
            .await
            .ok_or_else(|| TideError::NotFound(format!("download not found: {download_id}")))?;
        if !matches!(
            entry.status().lifecycle,
            DownloadLifecycle::Paused | DownloadLifecycle::Ready
        ) {
            return Err(TideError::InvalidInput(format!(
                "download {download_id} is not paused or resumable-ready"
            )));
        }
        // Re-validate the persisted source (M2 §36): the database is not
        // a security boundary.
        let record_url = entry.request.url.as_str().to_string();
        let allow_private = entry.request.url.private_allowed;
        validate_download_url(&record_url, allow_private)?;
        {
            let mut inner = entry.lock_inner();
            inner.lifecycle = DownloadLifecycle::Running;
            inner.phase = DownloadPhase::Connecting;
            inner.error = None;
        }
        spawn_run(&entry, app);
        Ok(())
    }

    /// Retry a failed download (M2 §52): SAME task id, new runtime,
    /// resume from the safe point. Works after an app restart: the task
    /// is restored from the persisted record when no runtime exists.
    pub async fn retry(
        &self,
        download_id: &str,
        app: Option<tauri::AppHandle>,
    ) -> Result<(), TideError> {
        let entry = match self.get(download_id).await {
            Some(entry) => entry,
            None => {
                // Post-restart: restore the runtime entry from the record.
                let record = self.repo.get(download_id)?.ok_or_else(|| {
                    TideError::NotFound(format!("download not found: {download_id}"))
                })?;
                let shared = self.shared_from_record(&record)?;
                self.register_restored(shared.clone());
                shared
            }
        };
        let lifecycle = entry.status().lifecycle;
        if !matches!(lifecycle, DownloadLifecycle::Failed) {
            return Err(TideError::InvalidInput(format!(
                "download {download_id} is not failed (state: {})",
                lifecycle.as_str()
            )));
        }
        {
            let mut inner = entry.lock_inner();
            inner.lifecycle = DownloadLifecycle::Running;
            inner.phase = DownloadPhase::Connecting;
            inner.error = None;
        }
        spawn_run(&entry, app);
        Ok(())
    }

    /// Deletes a task (M2 §53): DB record removed,  discarded,
    /// the downloaded FINAL file is user data and is preserved.
    pub async fn delete(&self, download_id: &str) -> Result<(), TideError> {
        let entry = self.get(download_id).await;
        if let Some(entry) = &entry {
            let status = entry.status();
            if matches!(status.lifecycle, DownloadLifecycle::Running) {
                return Err(TideError::InvalidInput(
                    "cancel the running download before deleting it".to_string(),
                ));
            }
            if let Some(part) = &status.part_path {
                FileSink::discard(part);
            }
        }
        self.repo.delete(download_id)?;
        let mut downloads = self
            .downloads
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        downloads.remove(download_id);
        Ok(())
    }

    pub async fn cancel(&self, download_id: &str) -> Result<(), TideError> {
        let entry = self
            .get(download_id)
            .await
            .ok_or_else(|| TideError::NotFound(format!("download not found: {download_id}")))?;
        entry.request_cancel();
        // Cancel semantics (M1 §21): stop network + DISCARD partial data.
        // A paused download has no running task, so discard here.
        let status = entry.status();
        if matches!(status.lifecycle, DownloadLifecycle::Paused) {
            if let Some(part) = &status.part_path {
                FileSink::discard(part);
            }
            entry.lock_inner().lifecycle = DownloadLifecycle::Cancelled;
        }
        Ok(())
    }
}

fn spawn_run(shared: &Arc<DownloadShared>, app: Option<tauri::AppHandle>) {
    eprintln!(
        "SPAWN_DEBUG: spawning run for id={}",
        shared.request.download_id
    );
    let task_shared = shared.clone();
    tokio::spawn(async move {
        if let Err(err) = run_with_slot(&task_shared, app).await {
            let settled = matches!(
                task_shared.status().lifecycle,
                DownloadLifecycle::Completed
                    | DownloadLifecycle::Cancelled
                    | DownloadLifecycle::Paused
            );
            if !settled {
                let mut inner = task_shared.lock_inner();
                inner.lifecycle = DownloadLifecycle::Failed;
                inner.error = Some((err.code().to_string(), err.message()));
            }
        }
    });
}

async fn run_with_slot(
    shared: &Arc<DownloadShared>,
    app: Option<tauri::AppHandle>,
) -> Result<(), TideError> {
    static SLOTS: std::sync::OnceLock<Semaphore> = std::sync::OnceLock::new();
    let slots = SLOTS.get_or_init(|| Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    let _permit = slots.acquire().await;
    run_download(shared, app).await
}

async fn run_download(
    shared: &Arc<DownloadShared>,
    app: Option<tauri::AppHandle>,
) -> Result<(), TideError> {
    {
        let mut inner = shared.lock_inner();
        if matches!(inner.lifecycle, DownloadLifecycle::Completed) {
            return Ok(());
        }
        inner.lifecycle = DownloadLifecycle::Running;
        inner.error = None;
    }
    let started = Instant::now();
    let mut speed = SpeedTracker::new(0.3);
    let mut last_emit: Option<Instant> = None;
    let mut attempts: u32 = 0;

    loop {
        // ---- connect (with resume decision) ----
        set_phase(shared, DownloadPhase::Connecting);
        let status = shared.status();
        let mut downloaded = status.downloaded;
        // M2 §6: the FILE is the source of truth for how many bytes are
        // physically present. A crash after write-before-checkpoint (Crash
        // A) leaves the part LONGER than the DB row — follow the file. A
        // short part (Crash "DB ahead") truncates the resume point back.
        if let Some(part) = &status.part_path {
            if let Ok(meta) = std::fs::metadata(part) {
                let physical = meta.len();
                if physical != downloaded {
                    crate::diagnostics::info(
                        "download.reconcile_part",
                        &[
                            ("download_id", &shared.request.download_id),
                            ("db_bytes", &downloaded.to_string()),
                            ("file_bytes", &physical.to_string()),
                        ],
                    );
                    downloaded = physical;
                    shared.lock_inner().downloaded = physical;
                }
            } else if downloaded > 0 {
                // Missing artifact with a nonzero checkpoint: the record's
                // part was deleted externally — restart from zero.
                crate::diagnostics::warn(
                    "download.part_missing_restart",
                    &[("download_id", &shared.request.download_id)],
                );
                downloaded = 0;
                shared.lock_inner().downloaded = 0;
            }
        }
        let validators = status.validators.clone();
        let resume = decide_resume(downloaded);
        let (range_start, if_range) = match resume {
            super::validator::ResumeDecision::FullDownload => (None, None),
            super::validator::ResumeDecision::TryResume { local_bytes } => (
                Some(local_bytes),
                validators.if_range_value().map(str::to_string),
            ),
        };
        crate::diagnostics::info(
            "download.connect",
            &[
                ("download_id", &shared.request.download_id),
                ("host", &shared.request.url.host),
                (
                    "resume",
                    &matches!(resume, super::validator::ResumeDecision::TryResume { .. })
                        .to_string(),
                ),
                ("attempt", &attempts.to_string()),
            ],
        );

        let response = {
            let wait = flag_wait(shared);
            tokio::select! {
                r = shared.transport().get(
                    &shared.request.url,
                    range_start,
                    if_range.as_deref(),
                    shared.request.url.private_allowed,
                ) => r,
                _ = wait => return handle_interrupt(shared, app.as_ref(), &mut speed, started),
            }
        }
        .inspect_err(|err| {
            set_error(shared, err);
        })?;

        // ---- headers ----
        let response_validators = ResponseValidators::from_headers(
            response.etag.as_deref(),
            response.last_modified.as_deref(),
        );
        let mut total = response.content_length.map(|len| len + downloaded);
        let mut response = response;

        match response.status {
            200 => {
                if downloaded > 0 {
                    // Range ignored or If-Range mismatch: a FULL
                    // representation follows. NEVER append it to the part
                    // (M1 §14/§79) — restart from zero.
                    crate::diagnostics::info(
                        "download.resume_fallback_restart",
                        &[("download_id", &shared.request.download_id)],
                    );
                    downloaded = 0;
                    total = response.content_length;
                    {
                        let mut inner = shared.lock_inner();
                        inner.downloaded = 0;
                        inner.total = total;
                        inner.validators = response_validators.clone();
                    }
                }
            }
            206 => match parse_content_range_header(response.content_range.as_deref()) {
                ContentRangeParse::Valid {
                    start,
                    total: cr_total,
                    ..
                } => {
                    if start != downloaded {
                        let err = TideError::Download(DownloadFailure::InvalidContentRange);
                        set_error(shared, &err);
                        return Err(err);
                    }
                    if let Some(t) = cr_total {
                        total = Some(t);
                    }
                }
                _ => {
                    let err = TideError::Download(DownloadFailure::InvalidContentRange);
                    set_error(shared, &err);
                    return Err(err);
                }
            },
            416 => match parse_content_range_header(response.content_range.as_deref()) {
                ContentRangeParse::Unsatisfiable { total: real_total }
                    if downloaded == real_total =>
                {
                    // The local part already holds the whole resource:
                    // finalize instead of re-downloading (M1 §14).
                    let final_path = finalize_existing(shared, downloaded).await?;
                    {
                        let mut inner = shared.lock_inner();
                        inner.lifecycle = DownloadLifecycle::Completed;
                        inner.downloaded = downloaded;
                        inner.total = Some(real_total);
                        inner.final_path = Some(final_path);
                    }
                    emit_progress(
                        shared,
                        app.as_ref(),
                        downloaded,
                        Some(real_total),
                        &mut speed,
                        &mut last_emit,
                        started,
                        true,
                    );
                    return Ok(());
                }
                _ => {
                    // Wrong local size: restart cleanly with a full GET.
                    shared.lock_inner().downloaded = 0;
                    continue;
                }
            },
            status => {
                let err = TideError::Download(DownloadFailure::HttpStatus { status });
                match decide_retry(shared, &mut attempts, &err).await {
                    Some(()) => continue,
                    None => return Err(err),
                }
            }
        }

        // ---- resolve filename + plan sink (once) ----
        if shared.status().part_path.is_none() {
            let filename = super::filename::resolve_filename(
                shared.request.filename.as_deref(),
                response.content_disposition.as_deref(),
                &shared.request.url.inner,
            )?;
            let (part_path, final_path) = FileSink::plan(
                &shared.request.destination_dir,
                &filename,
                shared.request.overwrite_policy,
            )?;
            shared.repo.update_resolved_paths(
                &shared.request.download_id,
                &filename,
                &part_path,
                &final_path,
                response.content_type.as_deref(),
            )?;
            let mut inner = shared.lock_inner();
            inner.resolved_filename = Some(filename);
            inner.part_path = Some(part_path);
            inner.final_path = Some(final_path);
        }
        let (part_path, final_path) = {
            let inner = shared.lock_inner();
            (
                inner.part_path.clone().expect("planned"),
                inner.final_path.clone().expect("planned"),
            )
        };
        {
            let mut inner = shared.lock_inner();
            inner.total = total;
            inner.downloaded = downloaded;
            inner.validators = response_validators.clone();
        }

        // ---- body ----
        set_phase(shared, DownloadPhase::Downloading);
        let mut sink = FileSink::open(part_path.clone(), final_path, downloaded, None)?;
        let mut last_checkpoint = Instant::now();
        let mut bytes_since_checkpoint: u64 = 0;
        let outcome = loop {
            let wait = flag_wait(shared);
            let chunk = tokio::select! {
                c = response.next_chunk() => c,
                _ = wait => break BodyOutcome::Interrupted,
            };
            match chunk {
                Ok(Some(bytes)) => {
                    if let Err(err) = sink.write_chunk(&bytes) {
                        break BodyOutcome::Failed(err);
                    }
                    let _ = sink.bytes_written();
                    let chunk_len = bytes.len() as u64;
                    bytes_since_checkpoint += chunk_len;
                    let downloaded_now = {
                        let mut inner = shared.lock_inner();
                        inner.downloaded += chunk_len;
                        inner.downloaded
                    };
                    // M4(下) §112/§115: bytes are flushed (write_chunk
                    // flushes per chunk) — only now may waiting readers
                    // learn the new durable length.
                    shared.notify_durable_len(downloaded_now);
                    speed.record(chunk_len, Instant::now());
                    // M2 §20/§48 checkpoint: time-or-byte threshold,
                    // whichever first (1 s / 1 MiB). Written AFTER the
                    // bytes are in the part file → DB ≤ durable file.
                    if last_checkpoint.elapsed() >= Duration::from_secs(1)
                        || bytes_since_checkpoint >= 1024 * 1024
                    {
                        shared.checkpoint(downloaded_now, total);
                        last_checkpoint = Instant::now();
                        bytes_since_checkpoint = 0;
                    }
                    emit_progress(
                        shared,
                        app.as_ref(),
                        downloaded_now,
                        total,
                        &mut speed,
                        &mut last_emit,
                        started,
                        false,
                    );
                }
                Ok(None) => break BodyOutcome::Complete,
                Err(err) => break BodyOutcome::Failed(err),
            }
        };

        match outcome {
            BodyOutcome::Complete => {
                set_phase(shared, DownloadPhase::Finalizing);
                let downloaded = shared.lock_inner().downloaded;
                if let Some(expected_total) = total {
                    if downloaded != expected_total {
                        crate::diagnostics::warn(
                            "download.size_mismatch",
                            &[
                                ("download_id", &shared.request.download_id),
                                ("downloaded", &downloaded.to_string()),
                                ("total", &expected_total.to_string()),
                            ],
                        );
                        match decide_retry(
                            shared,
                            &mut attempts,
                            &TideError::Download(DownloadFailure::Network),
                        )
                        .await
                        {
                            Some(()) => continue,
                            None => {
                                let err = TideError::Download(DownloadFailure::Network);
                                set_error(shared, &err);
                                return Err(err);
                            }
                        }
                    }
                }
                // Checksum (M1 §30): whole-file digest over the final part.
                if let Some((_, expected_hex)) = &shared.request.expected_checksum {
                    let mut writer = ChecksumWriter::new();
                    let mut file = std::fs::File::open(&part_path)
                        .map_err(|err| classify_io_error(&err, "open part for checksum"))?;
                    use std::io::Read;
                    let mut buf = vec![0u8; 256 * 1024];
                    loop {
                        match file.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => writer.update(&buf[..n]),
                            Err(err) => {
                                let err = classify_io_error(&err, "read part for checksum");
                                set_error(shared, &err);
                                return Err(err);
                            }
                        }
                    }
                    let computed = writer.finalize_hex();
                    if !super::checksum::matches(expected_hex, &computed) {
                        // Keep the .part for diagnosis; Completed must NOT
                        // happen (M1 §30).
                        let err = TideError::Download(DownloadFailure::ChecksumMismatch);
                        set_error(shared, &err);
                        return Err(err);
                    }
                }
                let final_path = sink.finalize()?;
                eprintln!(
                    "COMPLETE_DEBUG: id={} renamed to {}",
                    shared.request.download_id,
                    final_path.display()
                );
                {
                    let mut inner = shared.lock_inner();
                    inner.lifecycle = DownloadLifecycle::Completed;
                    inner.downloaded = downloaded;
                    inner.final_path = Some(final_path);
                }
                shared.persist_state("completed", downloaded, None, None);
                // M6 §26: the download is durably complete — notify
                // higher layers (library import). Must run after the
                // atomic rename + persisted state.
                shared.fire_on_complete();
                emit_progress(
                    shared,
                    app.as_ref(),
                    downloaded,
                    total,
                    &mut speed,
                    &mut last_emit,
                    started,
                    true,
                );
                crate::diagnostics::info(
                    "download.completed",
                    &[
                        ("download_id", &shared.request.download_id),
                        ("bytes", &downloaded.to_string()),
                    ],
                );
                return Ok(());
            }
            BodyOutcome::Interrupted => {
                return handle_interrupt(shared, app.as_ref(), &mut speed, started)
            }
            BodyOutcome::Failed(err) => {
                if shared.is_cancelled() {
                    return handle_interrupt(shared, app.as_ref(), &mut speed, started);
                }
                if shared.take_pause() {
                    let mut inner = shared.lock_inner();
                    inner.lifecycle = DownloadLifecycle::Paused;
                    drop(inner);
                    let downloaded = shared.lock_inner().downloaded;
                    emit_progress(
                        shared,
                        app.as_ref(),
                        downloaded,
                        total,
                        &mut speed,
                        &mut last_emit,
                        started,
                        true,
                    );
                    return Ok(());
                }
                match decide_retry(shared, &mut attempts, &err).await {
                    Some(()) => continue,
                    None => {
                        set_error(shared, &err);
                        return Err(err);
                    }
                }
            }
        }
    }
}

/// Pause/cancel exit path: cancel discards the part; pause keeps it.
fn handle_interrupt(
    shared: &Arc<DownloadShared>,
    app: Option<&tauri::AppHandle>,
    speed: &mut SpeedTracker,
    started: Instant,
) -> Result<(), TideError> {
    let status = shared.status();
    if shared.is_cancelled() {
        if let Some(part) = &status.part_path {
            FileSink::discard(part);
        }
        {
            let mut inner = shared.lock_inner();
            inner.lifecycle = DownloadLifecycle::Cancelled;
            inner.part_path = None;
            inner.final_path = None;
        }
        shared.persist_state("cancelled", status.downloaded, None, None);
        crate::diagnostics::info(
            "download.cancelled",
            &[("download_id", &shared.request.download_id)],
        );
        return Ok(());
    }
    shared.lock_inner().lifecycle = DownloadLifecycle::Paused;
    shared.persist_state("paused", status.downloaded, None, None);
    // Consume the pause flag: a stale flag would instantly re-pause the
    // resumed task.
    shared.take_pause();
    emit_progress(
        shared,
        app,
        status.downloaded,
        status.total,
        speed,
        &mut None,
        started,
        true,
    );
    Ok(())
}

enum BodyOutcome {
    Complete,
    Interrupted,
    Failed(TideError),
}

/// Returns `Some(())` to retry after a cancellable backoff, `None` to fail.
async fn decide_retry(
    shared: &Arc<DownloadShared>,
    attempts: &mut u32,
    err: &TideError,
) -> Option<()> {
    let retryable = matches!(err, TideError::Download(f) if f.is_retryable());
    if !retryable {
        return None;
    }
    *attempts += 1;
    if *attempts > MAX_RETRY_ATTEMPTS {
        return None;
    }
    {
        let mut inner = shared.lock_inner();
        inner.phase = DownloadPhase::Retrying;
        inner.attempts = *attempts;
    }
    if let Err(err) = shared
        .repo
        .update_retry_count(&shared.request.download_id, *attempts)
    {
        crate::diagnostics::warn("download.persist_retry_failed", &[("code", err.code())]);
    }
    crate::diagnostics::info(
        "download.retrying",
        &[
            ("download_id", &shared.request.download_id),
            ("attempt", &attempts.to_string()),
            ("error_code", err.code()),
        ],
    );
    let delay = retry_delay_ms(*attempts);
    let wait = flag_wait(shared);
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_millis(delay)) => {}
        _ = wait => {}
    }
    if shared.is_cancelled() {
        let mut inner = shared.lock_inner();
        inner.lifecycle = DownloadLifecycle::Cancelled;
        if let Some(part) = &inner.part_path {
            FileSink::discard(part);
        }
        return None;
    }
    if shared.take_pause() {
        shared.lock_inner().lifecycle = DownloadLifecycle::Paused;
        return None;
    }
    Some(())
}

fn set_phase(shared: &Arc<DownloadShared>, phase: DownloadPhase) {
    shared.lock_inner().phase = phase;
}

fn set_error(shared: &Arc<DownloadShared>, err: &TideError) {
    eprintln!(
        "SET_ERROR_DEBUG: id={} code={} lifecycle={:?}",
        shared.request.download_id,
        err.code(),
        shared.status().lifecycle
    );
    let downloaded;
    {
        let mut inner = shared.lock_inner();
        // A pause/cancel/complete that landed during the retry backoff is
        // settled user intent or success — never downgrade it to Failed
        // (this race previously overwrote a Completed row).
        if matches!(
            inner.lifecycle,
            DownloadLifecycle::Completed | DownloadLifecycle::Cancelled | DownloadLifecycle::Paused
        ) {
            return;
        }
        inner.lifecycle = DownloadLifecycle::Failed;
        inner.error = Some((err.code().to_string(), err.message()));
        downloaded = inner.downloaded;
    }
    shared.persist_state(
        "failed",
        downloaded,
        Some((err.code(), &err.message())),
        None,
    );
}

/// Waits until pause/cancel is requested (never completes on its own).
fn flag_wait(shared: &DownloadShared) -> impl std::future::Future<Output = ()> + '_ {
    use std::future::Future;
    std::future::poll_fn(move |cx| {
        if shared.flags_set() {
            return std::task::Poll::Ready(());
        }
        let mut notified = Box::pin(shared.wake.notified());
        match notified.as_mut().poll(cx) {
            std::task::Poll::Ready(()) => {
                if shared.flags_set() {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    })
}

async fn finalize_existing(
    shared: &Arc<DownloadShared>,
    downloaded: u64,
) -> Result<PathBuf, TideError> {
    let (part_path, final_path) = {
        let inner = shared.lock_inner();
        (
            inner.part_path.clone().expect("planned"),
            inner.final_path.clone().expect("planned"),
        )
    };
    let sink = FileSink::open(part_path, final_path, downloaded, None)?;
    sink.finalize()
}

/// Emits a throttled progress event (200 ms, M1 §27); `force` flushes
/// state transitions immediately.
#[allow(clippy::too_many_arguments)]
fn emit_progress(
    shared: &Arc<DownloadShared>,
    app: Option<&tauri::AppHandle>,
    downloaded: u64,
    total: Option<u64>,
    speed: &mut SpeedTracker,
    last_emit: &mut Option<Instant>,
    started: Instant,
    force: bool,
) {
    let Some(app) = app else { return };
    let due = force
        || last_emit
            .map(|t| t.elapsed() >= EVENT_INTERVAL)
            .unwrap_or(true);
    if !due {
        return;
    }
    *last_emit = Some(Instant::now());
    let status = shared.status();
    let progress = compute_progress(downloaded, total, speed.smoothed_bps(), started.elapsed());
    let payload = crate::dto::DownloadEventDto {
        download_id: shared.request.download_id.clone(),
        state: status.lifecycle.as_str().to_string(),
        phase: status.phase.as_str().to_string(),
        filename: status.resolved_filename,
        progress,
        error: status
            .error
            .map(|(code, message)| crate::dto::DownloadErrorDto { code, message }),
    };
    let _ = app.emit("download://progress", &payload);
}
