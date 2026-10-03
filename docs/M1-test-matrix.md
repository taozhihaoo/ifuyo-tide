# M1 — Test Matrix and Results

All results from real runs on the reference Windows 11 x64 machine
(2026-10-04). Commands:

```text
cargo test --locked              # 167 passed, 0 failed
  ├─ lib unit tests              131 (59 new in M1)
  ├─ tests/m1_download.rs         13 + 1 #[ignore] large-file proof
  ├─ tests/integration_persistence.rs  6 (M0 regression)
  └─ tests/spike_*.rs            16 (M0.5 regression)
pnpm test                        # 3 passed (frontend)
pnpm build / pnpm tauri build    # PASS
```

## Unit tests (M1 §53)

| Area | Covered |
|---|---|
| URL validation | schemes, credentials, malformed, private literals (v4+v6 ranges), opt-in, redirect targets |
| Range parser | closed/open/suffix/multi/invalid, hostile inputs (incl. `bytes=0-18446744073709551615` → found+fixed a real u64 overflow), resolution clamps, 416 cases |
| Content-Range parser | valid, `*`, unsatisfiable, malformed (total < end, wrong unit, missing parts) |
| Filename | traversal strip, separators, illegal chars, reserved names, trailing dots/spaces, length clamp, Content-Disposition `filename`/`filename*`/UTF-8, priority order, URL segments |
| Progress/Speed/ETA | EMA smoothing, known/unknown total, percentage never fabricated, complete/zero-speed, u64 scale |
| Checksum | SHA-256 known vector, chunked==one-shot, case-insensitive compare, algorithm parsing |
| Validator | ETag preference, mismatch detection, Last-Modified fallback, empty-match semantics, resume decisions |
| FileSink | policies (Fail/AutoRename/Overwrite), write+finalize atomicity, resume truncation of stale tails, checksum follow, discard, range read-back |
| Retry | bounded delays, growth, jitter variance, retryable classification, stable codes |
| State | lifecycle ↔ M0 TaskState mapping, phase serialization |

## Integration tests (M1 §38/§39/§54) — real HTTP over loopback

| Scenario | Result |
|---|---|
| 200 full download + SHA-256 + atomic finalize (1 MiB) | PASS |
| Pause → `.part` kept → Resume with server-observed `Range` → complete | PASS |
| Cancel mid-download → `.part` deleted, never finalized | PASS |
| Connection reset mid-body (`/unstable`) → retry resumes from partial, attempts ≥ 1 | PASS |
| Redirect matrix 301/302/303/307/308 (×5 hops, per-status server log assertion) → correct final resource | PASS |
| Chunked transfer (no Content-Length) → completes, bytes exact | PASS |
| 404 → `DOWNLOAD_HTTP_STATUS`, zero retries | PASS |
| Checksum mismatch → `DOWNLOAD_CHECKSUM_MISMATCH`, `.part` kept | PASS |
| **Range-ignored server + partial → restart (never append — the §79 corruption case)** | PASS |
| Remote changed (ETag bump) + partial → restart from zero | PASS |
| Complete local + 416 (total == local) → finalize without re-download | PASS |
| SSRF private-literal URL blocked at request level | PASS |
| Small sizes, spot pattern verification at 0 / 50 % / EOF | PASS |
| **True zero-byte representation** (Content-Length: 0) → completes, final file exactly 0 bytes, empty-body SHA-256 | PASS |

## Large file (M1 §29)

| Test | Result |
|---|---|
| `huge_file_beyond_4gib_downloads_and_verifies` (`#[ignore]`, run explicitly): 4 GiB + 512 B real download through the engine, spot pattern checks beyond the 4 GiB line, full SHA-256 | **PASS** (238.9 s measured) |

## Manual validation

Engine-level demo flow (M1 §73) is executed by
`pause_resume_uses_range_and_completes` +
`unstable_connection_retries_and_resumes_from_partial` (start →
progress → pause → `.part` on disk → resume → Range observed by the
server → complete → SHA-256 verified). GUI click-through for a human
(`pnpm tauri dev` → paste URL → watch speed/ETA → pause/resume buttons)
is available in the Downloads view; automated equivalents cover the
same IPC paths. The "network interruption" scenario is covered by the
connection-reset injection (disabling the NIC mid-download requires
manual execution on real hardware).

## Known gaps

- DNS-rebinding into private ranges not detected (documented in
  M1-security.md).
- Pre-flight free-space check: `DOWNLOAD_DISK_SPACE` code reserved;
  disk-full surfaces via write-error classification today.
- Proxy support: `system-proxy` feature disabled in M1 (Node-adjacent
  DLL loading issue in the test environment); env-var proxies are not
  honored. Revisit when a real deployment needs it.
