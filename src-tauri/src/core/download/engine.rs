//! DownloadEngine (M1 §1, §3): the executor that moves an HTTP resource
//! to a local file — state machine, resume decisions, retry, progress
//! events, cancellation. UI never touches this directly (DTOs/events
//! only); M2 will attach persistence at the checkpoint seams.

use super::checksum::ChecksumWriter;
use super::file_sink::{classify_io_error, FileSink};
use super::http_range::{parse_content_range_header, ContentRangeParse};
use super::progress::{compute_progress, SpeedTracker};
use super::request::{retry_delay_ms, DownloadRequest, MAX_RETRY_ATTEMPTS};
use super::state::{DownloadLifecycle, DownloadPhase};
use super::url::DownloadFailure;
use super::validator::{decide_resume, ResponseValidators};
use super::HttpTransport;
use crate::error::TideError;
use tauri::Emitter;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, Semaphore};

/// Progress event throttle (M1 §27).
const EVENT_INTERVAL: Duration = Duration::from_millis(200);
/// Simple concurrency gate (M1 §42): a bound, not a scheduler.
const MAX_CONCURRENT_DOWNLOADS: usize = 3;

pub struct DownloadEngine {
    downloads: Mutex<HashMap<String, Arc<DownloadShared>>>,
}

pub struct DownloadShared {
    pub request: DownloadRequest,
    inner: Mutex<DownloadInner>,
    pause_flag: AtomicBool,
    cancel_flag: AtomicBool,
    wake: Notify,
    transport: HttpTransport,
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
    fn new(request: DownloadRequest) -> DownloadShared {
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
    /// Construction is static-config only (TLS setup); called once at
    /// startup, never on a user path.
    pub fn new() -> DownloadEngine {
        DownloadEngine {
            downloads: Mutex::new(HashMap::new()),
        }
    }

    /// Registers and starts a download; returns its shared handle.
    pub async fn start(
        &self,
        request: DownloadRequest,
        app: Option<tauri::AppHandle>,
    ) -> Result<Arc<DownloadShared>, TideError> {
        let mut downloads = self.downloads.lock().unwrap_or_else(PoisonError::into_inner);
        let shared = Arc::new(DownloadShared::new(request));
        let id = shared.request.download_id.clone();
        downloads.insert(id.clone(), shared.clone());

        let task_shared = shared.clone();
        tokio::spawn(async move {
            if let Err(err) = run_with_slot(&task_shared, app).await {
                let mut inner = task_shared.lock_inner();
                inner.lifecycle = DownloadLifecycle::Failed;
                inner.error = Some((err.code().to_string(), err.message()));
            }
        });
        Ok(shared)
    }

    pub async fn get(&self, download_id: &str) -> Option<Arc<DownloadShared>> {
        self.downloads.lock().unwrap_or_else(PoisonError::into_inner).get(download_id).cloned()
    }

    pub async fn list(&self) -> Vec<Arc<DownloadShared>> {
        self.downloads.lock().unwrap_or_else(PoisonError::into_inner).values().cloned().collect()
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

    /// Resumes a paused download: in-memory session + existing `.part`
    /// (M1 §51). Cross-restart recovery is M2 scope.
    pub async fn resume(
        &self,
        download_id: &str,
        app: Option<tauri::AppHandle>,
    ) -> Result<(), TideError> {
        let entry = self
            .get(download_id)
            .await
            .ok_or_else(|| TideError::NotFound(format!("download not found: {download_id}")))?;
        if !matches!(entry.status().lifecycle, DownloadLifecycle::Paused) {
            return Err(TideError::InvalidInput(format!(
                "download {download_id} is not paused"
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
    let task_shared = shared.clone();
    tokio::spawn(async move {
        if let Err(err) = run_with_slot(&task_shared, app).await {
            let mut inner = task_shared.lock_inner();
            inner.lifecycle = DownloadLifecycle::Failed;
            inner.error = Some((err.code().to_string(), err.message()));
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
        let validators = status.validators.clone();
        let resume = decide_resume(downloaded);
        let (range_start, if_range) = match resume {
            super::validator::ResumeDecision::FullDownload => (None, None),
            super::validator::ResumeDecision::TryResume { local_bytes } => {
                (Some(local_bytes), validators.if_range_value().map(str::to_string))
            }
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
        .map_err(|err| {
            set_error(shared, &err);
            err
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
                ContentRangeParse::Valid { start, total: cr_total, .. } => {
                    if start != downloaded {
                        let err =
                            TideError::Download(DownloadFailure::InvalidContentRange);
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
                    emit_progress(shared, app.as_ref(), downloaded, Some(real_total), &mut speed, &mut last_emit, started, true);
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
                    {
                        let mut inner = shared.lock_inner();
                        inner.downloaded += chunk_len;
                    }
                    speed.record(chunk_len, Instant::now());
                    let downloaded_now = shared.lock_inner().downloaded;
                    emit_progress(shared, app.as_ref(), downloaded_now, total, &mut speed, &mut last_emit, started, false);
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
                        match decide_retry(shared, &mut attempts, &TideError::Download(DownloadFailure::Network)).await {
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
                    let mut file = std::fs::File::open(&part_path).map_err(|err| {
                        classify_io_error(&err, "open part for checksum")
                    })?;
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
                {
                    let mut inner = shared.lock_inner();
                    inner.lifecycle = DownloadLifecycle::Completed;
                    inner.downloaded = downloaded;
                    inner.final_path = Some(final_path);
                }
                emit_progress(shared, app.as_ref(), downloaded, total, &mut speed, &mut last_emit, started, true);
                crate::diagnostics::info(
                    "download.completed",
                    &[
                        ("download_id", &shared.request.download_id),
                        ("bytes", &downloaded.to_string()),
                    ],
                );
                return Ok(());
            }
            BodyOutcome::Interrupted => return handle_interrupt(shared, app.as_ref(), &mut speed, started),
            BodyOutcome::Failed(err) => {
                if shared.is_cancelled() {
                    return handle_interrupt(shared, app.as_ref(), &mut speed, started);
                }
                if shared.take_pause() {
                    let mut inner = shared.lock_inner();
                    inner.lifecycle = DownloadLifecycle::Paused;
                    drop(inner);
                    let downloaded = shared.lock_inner().downloaded;
                    emit_progress(shared, app.as_ref(), downloaded, total, &mut speed, &mut last_emit, started, true);
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
        shared.lock_inner().lifecycle = DownloadLifecycle::Cancelled;
        crate::diagnostics::info(
            "download.cancelled",
            &[("download_id", &shared.request.download_id)],
        );
        return Ok(());
    }
    shared.lock_inner().lifecycle = DownloadLifecycle::Paused;
    emit_progress(shared, app, status.downloaded, status.total, speed, &mut None, started, true);
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
    let mut inner = shared.lock_inner();
    inner.lifecycle = DownloadLifecycle::Failed;
    inner.error = Some((err.code().to_string(), err.message()));
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
        error: status.error.map(|(code, message)| crate::dto::DownloadErrorDto {
            code,
            message,
        }),
    };
    let _ = app.emit("download://progress", &payload);
}

