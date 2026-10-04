# M11 Security Regression

Scope: §20/§21/§31–§37/§61 — re-verification at RC freeze that every
M10 security boundary still holds and that release configuration is
minimal and honest. Evidence types per M11 rules. Nothing in M11
changed security-relevant code except the fail-closed stub fix
(commit c6a238c — a release-build compiler fix that narrows, not
widens, the debug surface).

## 1. Security regression corpus (TEST RESULT)

`tests/security_corpus.rs` (green, part of the 317-test run):

- SSRF matrix: 7 non-HTTP schemes rejected, 7 private IP literals
  (v4/v6, loopback/private/link-local) × allow/deny policy, numeric
  IP encodings rejected.
- Redirect to private target blocked (per-hop re-validation).
- 16 hostile strings (RTLO, NUL, 100 KB, emoji flood, device paths,
  UNC, 1 000-emoji names) across both private-address policies — no
  panic, structured rejection.
- Hostile `Range` headers (overflow, negative, inverted, garbage)
  fail closed via `resolve_range_header`.

## 2. Stream gateway (§20) — TEST RESULT

`m4_streaming::gateway_security_matrix` (green): loopback-only bind,
session token required, invalid token / wrong Host / illegal Range /
unknown resource / closed session rejected; no path-based arbitrary
file access (`http://localhost:<port>/C:/...` style URLs do not
exist — token + resource-id mapping, FACT).

## 3. Playback / media as untrusted input (§21) — TEST RESULT + inference

Hostile-string corpus covers file/URL names reaching validation;
gateway matrix covers range/token abuse. Fuzzed/corrupt media
payloads are parsed by WebView2's media stack, outside Tide's trust
boundary — Tide treats the file as opaque bytes (FACT); a corrupt-
media crash-safety suite does not exist and is not claimed (manual
item).

## 4. Windows path security (§31) — TEST RESULT (corpus) + FACT (policy)

Corpus cases: `..`, absolute/UNC/device paths, reserved-device
strings, trailing-dot/space, RTLO/Unicode, 100 KB names. Policy per
M10: app writes only into its app-data and the user-confirmed
download root; every IPC path argument is canonicalized and
containment-checked in Rust.

## 5. URL / torrent source security (§32) — TEST RESULT

Same corpus as M10 (scheme allow-list, private-IP matrix, redirect
re-validation). DB-stored sources do not bypass policy — validation
runs on every task start (FACT). DNS-rebinding TOCTOU remains
NOT GUARANTEED (KI-2) — unchanged, honestly recorded.

## 6. Capabilities audit (§33) — STATIC ANALYSIS

`src-tauri/capabilities/default.json`: `core:default`,
`dialog:allow-open`, `notification:*` (notify/is-permission-granted/
request-permission), `autostart:*` (enable/disable/is-enabled),
`deep-link:allow-get-current`. Every permission maps to a shipped
feature (dialog = download-dir picker; notifications = download/
torrent completion toasts; autostart = opt-in Windows setting;
deep-link = `tide://add` intake). No `fs`, `shell`, `http`,
`process`, `clipboard`, `updater` permissions. Nothing to remove.

## 7. Trust boundary (§34) — FACT

Frontend validation is UX only; every IPC input (task id, path, URL,
range, resource id, session token, file selection, destination,
source type) is re-validated in Rust (`security::` module, M10 §34
inventory unchanged).

## 8. Gateway / IPC audit (§35) — STATIC ANALYSIS + TEST RESULT

IPC: explicit command allow-list via `invoke_handler`; all commands
return structured `{code, message}` errors (no panic across IPC, no
internal error leakage); spike commands are fail-closed stubs in
release (verified by release-profile compile + M10 symbol scan).
Gateway: request limits, timeouts, token checks (§2 above). No
unbounded concurrency introduced in M11.

## 9. Logging audit (§36) — FACT

Diagnostics are structured JSON lines: `ts`, `level`, `event`, and
id/count/code fields only. Release min level = info. No URLs,
magnets, peer IPs, full local paths, tokens or credentials are
logged (M10 §36 inventory; unchanged — verified by sampling the
smoke-run logs, MANUAL REVIEW: only `app.started`, `db.*`,
`scheduler.*`, `ipc.error{command,code}`-class events present).

## 10. Error message audit (§37) — FACT + MANUAL REVIEW

User-facing errors are centralized, stable, short strings (M9 error
UX); raw Rust internals never cross IPC. Startup fatal failures go
to stderr and exit(1) (fail closed). **Honest gap (KI-9, P3)**: in a
release build the stderr channel is invisible to the user — a
corrupt-DB startup failure looks like "app does not open" with no
dialog. Data-safe but silent; scheduled for 1.0 UX work.

## 11. Privacy regression (§61) — FACT + STATIC ANALYSIS

No account, no cloud, no telemetry, no crash reporting, no update
check (updater not present — `bundle` config and dependencies
audited), no upload of user content or task history. The only
automatic outbound connections are the ones the user's tasks cause
(download sources, torrent peers/trackers/DHT) and the loopback
gateway (127.0.0.1). PRIVACY.md phrasing avoids absolute
"collects nothing" claims; it points at `M10-network-surface.md`.
Packet-capture verification of the no-cloud claim: manual item
(carried from M10, KI-6).

## 12. Content-policy freeze (§70)

CONTENT_POLICY.md unchanged and consistent with the product: no
resource discovery, no infringement-source recommendation, no DRM/
paywall bypass, no anonymity claim. UI copy carries no
"fast/anonymous/untraceable" marketing (MANUAL REVIEW of shipped
strings, M9 string centralization).
