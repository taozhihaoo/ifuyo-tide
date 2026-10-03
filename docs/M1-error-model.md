# M1 — Error Model

All download failures flow through `TideError::Download(DownloadFailure)`
(M1 §45 — one error boundary, no second error type). IPC serializes to
`{ code, message }`; frontend DTOs additionally expose the stable code.

## Error matrix

| Code | Source | Recoverable | Retry (auto) | User action |
|---|---|---|---|---|
| `DOWNLOAD_INVALID_URL` | URL validation | no | no | fix the URL |
| `DOWNLOAD_UNSUPPORTED_SCHEME` | URL validation | no | no | use http/https |
| `DOWNLOAD_PRIVATE_ADDRESS_BLOCKED` | SSRF baseline | yes (opt-in flag / future LAN setting) | no | explicitly allow private targets |
| `DOWNLOAD_REDIRECT_LIMIT` | >10 redirects or non-http hop | no | no | server misconfiguration |
| `DOWNLOAD_HTTP_STATUS` (404/403/410 …) | status dispatch | no | **no** | check the URL / authorization |
| `DOWNLOAD_HTTP_STATUS` (5xx/429/408) | status dispatch | yes | **yes** (bounded, backoff) | none / wait |
| `DOWNLOAD_NETWORK` | connect/reset/body decode | yes | **yes** (bounded) | check connectivity |
| `DOWNLOAD_TIMEOUT` | 15 s connect / 30 s idle read | yes | **yes** (bounded) | retry later |
| `DOWNLOAD_CANCELLED` | user cancel | — | no | — |
| `DOWNLOAD_DISK_FULL` | write path | yes | after user frees space | free disk space |
| `DOWNLOAD_PERMISSION_DENIED` | write path | no | no | fix directory permissions |
| `DOWNLOAD_FILE_WRITE` | write path (other) | maybe | no | inspect the directory |
| `DOWNLOAD_DESTINATION_EXISTS` | overwrite policy = Fail | yes | no | choose another name / enable AutoRename |
| `DOWNLOAD_RESUME_NOT_SUPPORTED` | reserved (server without Range falls back to restart today) | — | — | — |
| `DOWNLOAD_REMOTE_CHANGED` | reserved (If-Range mismatch currently restarts automatically) | — | — | — |
| `DOWNLOAD_CHECKSUM_MISMATCH` | SHA-256 comparison | yes | policy-dependent (NO auto retry in M1) | verify the expected checksum; `.part` is kept for diagnosis |
| `DOWNLOAD_INVALID_CONTENT_RANGE` | 206 Content-Range inconsistent with local size | no | no | server bug / stale state |
| `DOWNLOAD_DISK_SPACE` | reserved code (pre-flight free-space check lands with M2 checkpoints) | yes | after user action | free disk space |

## Rules

- Retryable classification lives in one place:
  `DownloadFailure::is_retryable()` (network/timeout/5xx/429/408 only).
  Max 4 attempts, exponential backoff with full jitter, 8 s cap.
- Raw OS strings, reqwest error chains, full URLs and credentials never
  reach the frontend or logs (M1 §46): logs carry `download_id`, host,
  state, error_code, byte counts only.
- Non-retryable failures keep the `.part` where diagnosis matters
  (checksum mismatch); cancel discards it.
