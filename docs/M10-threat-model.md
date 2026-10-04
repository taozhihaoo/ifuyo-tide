# M10 Threat Model (§5)

## Attacker A — Malicious input provider

Surfaces: download URL, redirect hops, magnet URI, .torrent metadata,
file names from torrents, HTTP headers, media content, range headers.

| Goal | Defense | Status |
|---|---|---|
| Arbitrary file write | Filename sanitization (M1), torrent path validation (M3), canonicalize+containment (M6), .part under validated destination | TESTED (m1/m3/m6 suites) |
| Arbitrary file read | Gateway token+session only; no path-based IPC; library reveal re-resolves from item id | TESTED |
| SSRF | Scheme allowlist (http/https only), private/loopback/link-local IP literal rejection, per-hop redirect re-validation with bounded hops | TESTED (unit); DNS-rebinding = PARTIALLY VERIFIED (see below) |
| Path escape | Component-wise containment, reserved device names, reparse-point canonicalize check | TESTED (m6 fuzz + junction test) |
| Resource exhaustion | 16 KiB header cap, 256 KiB chunks, bounded redirects (M1), demand registry bounded, single-range policy, per-task rate limit (M7) | TESTED |
| Crash | 313-test suite incl. hostile corpora; panic audit classified | TESTED |

**DNS rebinding**: the M1 validator resolves IP literals but does NOT
pin DNS resolution (TOCTOU between validate and connect). Recorded:
PARTIALLY VERIFIED — a rebinding attacker who controls a hostname
could reach localhost between validation and connection. Mitigation
scope: fetches target user-provided URLs only (not internal APIs);
gateway/IPC are not reachable via DNS names.

## Attacker B — Local process

| Goal | Defense | Status |
|---|---|---|
| Read media via gateway | UUIDv4 capability token, loopback bind, Host check, token revoked on stop/close/deletion | TESTED (M4/M8 suites) |
| Guess session/token | 122-bit random UUIDv4; runtime-only; never logged (M4 §70) | TESTED (grep audit) |
| Invoke debug IPC | Spike commands compile to fail-closed stubs in release (M10 §23) | TESTED (release binary symbol scan) |
| Corrupt another instance's DB | Single-instance plugin; second launch forwards+exits | PLUGIN CONTRACT |
| Read logs/config | %LOCALAPPDATA% app-data ACL (user-scoped); logs contain ids/counts only | DOCUMENTED |

## Attacker C — Malicious network peer (Torrent)

BitTorrent protocol safety is librqbit's boundary (§11). Tide's
controls: metadata path validation (M3, tested), per-task rate limits
(M7), bounded connections via librqbit defaults, DHT/trackers
configurable (off in tests), no UPnP (M3 §28). librqbit version
audited (M10 supply chain): 9.0.1, ringbuf advisory documented.

## Attacker D — Malicious media file

WebView2 media stack is the trust boundary — Tide does not parse
media bytes itself (M5 §91: no FFmpeg/native player). Untrusted media
reaches only the OS-provided decoder. "Open in default app" hands the
file to the OS handler — the user's explicit action, same risk as
double-clicking in Explorer.
