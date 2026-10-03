# M1 — HTTP Download Core

Status: **implemented and tested** (see M1-test-matrix.md for the
evidence). This document describes the architecture as actually built.

## Architecture

```text
Commands (IPC, DTOs)            src/commands/download.rs
        ↓
DownloadEngine                  src/core/download/engine.rs
        ↓            ↑
HttpTransport   FileSink        transport.rs / file_sink.rs
(reqwest 0.13.5)                (.part lifecycle, atomic rename)
```

- **DownloadEngine** owns the state machine, resume decisions, retry,
  progress aggregation and cancellation. It lives in `AppState`
  (dependency-injected — no global singleton, M1 §63/§64). One spawned
  task per active download; a process-wide semaphore bounds concurrency
  to 3 (M1 §42 — a bound, not a scheduler; M7 replaces it).
- **HttpTransport** owns the pooled reqwest client (M1 §63): 15 s
  connect timeout, 30 s read (idle) timeout, ≤10 redirects with every
  hop re-validated (scheme allow-list; literal-private re-check rides
  the download's opt-in), `Accept-Encoding: identity` (byte-for-byte
  downloads — no transparent decompression, M1 §17), explicit
  `User-Agent`.
- **FileSink** owns disk: `<name>.part` + `<name>` pair, truncates the
  part to the verified offset on open (crash tails can't corrupt),
  fsync + atomic rename on finalize, discard on cancel.
- **UI** consumes `DownloadDto` snapshots and the throttled
  `download://progress` event (200 ms; flush on every state transition).
  It never touches engine internals.

## State model

Reuses the M0 `TaskState` vocabulary (no parallel machine, M1 §23):

```text
Lifecycle: Created → Running ⇄ Paused → Completed | Failed | Cancelled
Phase (while Running): preparing → connecting → downloading → finalizing
                       (↺ retrying)
```

## Request flow

1. IPC validates: scheme (http/https only), SSRF baseline (private
   literals blocked unless `allow_private`), destination absolute path,
   checksum format.
2. Engine resolves filename (user > Content-Disposition `filename*` >
   `filename` > URL segment > `download.bin`) and plans the sink with
   the overwrite policy (**AutoRename** is the current default for the
   UI path; `Fail` / `Overwrite` available; never silently clobbers).
3. GET (+ `Range: bytes=N-` + `If-Range` when a validated partial
   exists) → status dispatch → body stream → 256 KiB-max reads →
   sequential writes with optional SHA-256.
4. Completion: body EOF ∧ size match (when known) ∧ checksum match
   (when configured) ∧ fsync → **atomic rename** → Completed. Any
   earlier failure leaves the `.part` (kept for diagnosis on checksum
   mismatch, discarded on cancel).

## Retry

Retryable: network errors, timeouts, 408/429/5xx. Exponential backoff
with full jitter: 500 ms × 2^attempt, capped at 8 s, max 4 attempts.
Every retry RESUMES from the partial (network reset ≠ restart). Retry
delays are cancellable (pause/cancel interrupt the sleep).

## Pause / Cancel semantics (M1 §21 — pinned)

- **Pause** = stop network, keep `.part`, lifecycle → Paused.
  Resume restarts the task; the server sees a fresh `Range` request.
- **Cancel** = stop network, **delete `.part`**, lifecycle → Cancelled.

## Checkpoint interface (for M2)

`DownloadStatus` already exposes everything M2 must persist:
`downloaded`, `total`, validators (`etag`/`last_modified`),
`resolved_filename`, `part_path`, `final_path`, lifecycle, error.
M1 deliberately persists nothing (in-memory sessions + on-disk `.part`
only); M2 adds the DB + startup reconcile.
