# Tide M10 Security & Release Hardening Report

## 1. Executive Summary

M10 closes the security loop: the 18-command M0.5 debug IPC surface
is fail-closed in release builds, the supply chain is gated by
cargo-deny (advisories/licenses/bans/sources) with documented
justifications, a formal threat model and network-surface inventory
are committed, the security regression corpus is extended (URL/SSRF,
redirect, hostile-string, range-abuse suites), the release artifact
carries a reproducible manifest (SHA-256/commit/toolchain/lockfiles),
and the signing/updater paths are documented without implementation
creep. 313 Rust + 38 frontend tests pass; exe + NSIS installer build.

## 2. FACTS

- Rust 313/0, frontend 38/0, fmt/tsc clean, clippy zero warnings in
  M8/M9/M10-owned code, release exe + NSIS installer build.
- Release binary symbol scan: zero spike command symbols (debug
  surface removed).
- `cargo deny check`: advisories/licenses/bans/sources all pass with
  documented, justified ignores (3 entries).
- Redirect security: every hop re-validated (M1 transport custom
  policy), tested.
- Gateway: loopback bind, token, Host check, single-range — all
  covered by the M4/M8 suites, re-run green.

## 3. HYPOTHESES

- ringbuf RUSTSEC-2026-0293 is unreachable from Tide's usage (requires
  a Drop panic inside librqbit-utp internals) — analysis-based,
  ASSESSMENT (not fuzz-proven).

## 4. INFERENCES

- The absence of remote-loading code paths plus CSP restrictions
  implies WebView2 cannot load remote content even if navigation were
  attempted (media-src/connect-src restricted).

## 5. Current M0-M9 State

All M0–M9 suites green at M10 start and end; M10 backend changes
limited to spike gating + settings command additions.

## 6. Attack Surface Inventory

docs/M10-threat-model.md (§4 actors) + docs/M10-network-surface.md
(§34 table): HTTP downloader, torrent peers/trackers/DHT, stream
gateway (localhost), Windows integrations (launch/association), IPC
(53 commands), WebView2 surface. Updater: NOT PRESENT.

## 7. Threat Model

Four actors (malicious input provider, local process, malicious peer,
malicious media file) with per-goal defenses and statuses —
docs/M10-threat-model.md.

## 8. Tauri Capability Audit

Minimal set re-audited: core:default, dialog:allow-open,
notification (3), autostart (3), deep-link (1). No
fs/shell/process/http plugin grants. Every command re-validates
inputs in Rust (capability = first boundary, not the last, §6).

## 9. CSP Audit

`default-src 'self'; script-src 'self'; style-src 'self'
'unsafe-inline'; img-src 'self' data:; font-src 'self' data:;
media-src 'self' http://127.0.0.1:* http://localhost:*;
connect-src 'self' ipc: http://ipc.localhost` — audited: no remote
script/CDN/object/frame/form-action directives needed; media-src
loopback origins are the minimum viable scope for the dynamic
gateway port (§7 requirement met without `*` on script/connect).

## 10. IPC Security

53 production commands audited: every id re-validated (scheduler §88,
library reveal, playback sessions); no raw SQL (repositories
parameterize); spike surface fail-closed in release; error responses
structured (§80). Command/DTO/validation/side-effect table maintained
per command module docs.

## 11. HTTP/SSRF Security

Scheme allowlist; IP-literal private/loopback/link-local rejection;
per-hop redirect re-validation with bounded hops; userinfo rejected
by the url crate parse. DNS rebinding between validation and connect:
NOT GUARANTEED — recorded (M1 §62 documented limitation; mitigation
scope limited because fetches target user URLs, not internal APIs).

## 12. Filesystem Security

Canonicalize + component-wise containment + reserved names + reparse
refusal (M6 path module, fuzz-tested); .part/final inside validated
destination; TOCTOU narrowed by canonicalize-then-open (§10) and
re-validation at use (M6 §48). Windows specifics (junction/UNC/
ADS/RTLO) covered in the security corpus.

## 13. Torrent / librqbit Security

Metadata path validation (M3, fuzz-tested); DownloadOnly; no UPnP;
listener configurable (loopback in tests, production default
documented in network-surface); librqbit 9.0.1 pinned; ringbuf
advisory documented (P1).

## 14. Stream Gateway Security

Loopback 127.0.0.1:0, UUIDv4 token (runtime-only), Host check,
single-range with bounded parsing, 16 KiB headers, 256 KiB chunks,
no proxy capability, revoked on stop/close/task-deletion (§12
checklist all confirmed by M4/M8 suites).

## 15. WebView2 Security

Self-origin UI; no ExecuteScript/add-init-script usage; no remote
debugging flags in release; navigation stays on self + loopback
gateway origins (media-src CSP); external URLs are not opened in the
webview (launch requests become tasks after confirmation).

## 16. Media Security

Media bytes go straight to the OS decoder via `<video>`; Tide reads
no metadata, treats media as untrusted bytes, never interprets media
content as commands (§14).

## 17. SQLite / Persistence Security

Parameterized repositories only; transactional migrations; app-data
location; no media in SQLite; playback tokens never persisted (§15).

## 18. Logging / Privacy

Logs: ids, counts, reasons, codes — the §40 redaction list (full
URL/magnet/path/token/peer) is enforced by convention and audited;
privacy baseline re-verified in docs/M10-network-surface.md (§35
runtime packet-capture pass = manual item).

## 19. Dependency Audit

cargo-deny gate (deny.toml): advisories scanned via RustSec; 3
documented ignores (ringbuf vuln-no-fix, 2 unmaintained); frontend
audit recorded; classifications per §20 (vulnerability/maintenance/
transitive).

## 20. License Audit

App: MIT OR Apache-2.0. Tree: permissive-only allowlist passing.
THIRD_PARTY_NOTICES.md updated with the M8 plugin set (§21).

## 21. Build Security

`--locked` everywhere; release profile (no debug assertions); spike
commands stubbed; dev diagnostics tree-shaken (import.meta.env.DEV);
no dev server dependency in the artifact.

## 22. Installer Security

NSIS per-user; registers only shortcuts + associations; uninstall
preserves user downloads (M8 §84); no secrets/test files in bundle
(§24 audit).

## 23. Signing

Unsigned dev build; documented signing path: certificate acquisition
(M11 decision), env-var-based signtool/bundler config in release CI
only, SHA-256 timestamped signatures, never in-repo keys (§26).

## 24. Updater

NOT PRESENT. Requirements documented (Tauri updater keypair offline,
public key in config, signed manifests, downgrade rejection) —
implementation deliberately deferred (§27).

## 25. CI/CD Security

No CI workflow exists in this repository (local-build project).
Recorded: when CI lands (M11), workflow permissions must be minimal,
release workflow separated, no write-all (§36).

## 26. Security Tests

tests/security_corpus.rs (4 suites) + M4 gateway security matrix +
M6 filesystem/containment suites + M8 launch-request fuzz + prior
M1 SSRF unit tests — §31's full checklist mapped in
docs/M10-test-matrix.md.

## 27. Release Smoke Test

The NSIS installer is the §83 carrier; the full clean-machine loop
(install → … → uninstall with data-preservation checks) is the
recorded manual acceptance — automated layers cover every step.

## 28. Performance Regression

No security fix touched hot paths (spike stubbing removes surface;
deny gate is build-time). Scheduler/gateway benchmarks from M7
re-run green (807 µs/1000-task tick, gateway throughput unchanged).

## 29. RESOLVED

- M0.5 spike debug IPC surface exposed in release builds →
  fail-closed stubs + release binary symbol-verified clean (M10 §23).
- ifuyo-tide crate had no license field → MIT OR Apache-2.0.
- No supply-chain gate existed → cargo-deny wired with documented
  ignores.

## 30. CONFIRMED

- Gateway loopback-only + token model (M4 suites, re-run).
- Redirect per-hop re-validation (M1 transport code + tests).
- Launch-request validation survives fuzz (M8/M10 corpora).

## 31. OPEN

- DNS-rebinding TOCTOU (validate→connect window): PARTIALLY VERIFIED,
  recorded NOT GUARANTEED (§9) — M11 candidate for a pinned-resolution
  connector.
- Runtime packet-capture no-cloud verification (§35) — manual.
- ringbuf 0.4.x advisory — no upstream fix (documented P1).

## 32. PARTIALLY VERIFIED

- SSRF: IP literals + redirects TESTED; DNS rebinding NOT GUARANTEED.
- WebView2 navigation: CSP + config audited (STATIC ANALYSIS);
  runtime navigation-interception test not automated.

## 33. WORSENED

Nothing — all pre-M10 suites green throughout.

## 34. P0

0.

## 35. P1

- ringbuf RUSTSEC-2026-0293 (no upstream fix; documented reachability
  analysis) — revisit on librqbit-utp update.
- DNS-rebinding window (see §31 OPEN).

## 36. Remaining Limitations

Manual installed-build passes (smoke/DPI/no-cloud packet capture);
no CI pipeline exists (nothing to audit — recorded, §25); signing
pending certificate decision (M11).

## 37. Changed Files

`app/mod.rs` (conditional invoke handler), `commands/mod.rs`
(cfg-gated spike module), `Cargo.toml` (license), `Cargo.lock`
(ringbuf audit trail), `deny.toml`, `tests/security_corpus.rs`,
`scripts/release-manifest.ps1`, `SECURITY.md`, `PRIVACY.md`, docs set.

## 38. M11 Readiness

READY FOR M11 (§56): no P0, P1 items are documented upstream/
manual items that do not block RC. Signing decision and updater
implementation are the M11-adjacent items M10 prepared paths for.

## 39. DoD Checklist

§52 Security/Supply Chain/Release/Testing/Quality blocks: all items
verified per docs/M10-security-audit.md, M10-supply-chain.md,
M10-release-hardening.md and M10-test-matrix.md; manual installed-
build items disclosed per §51 evidence-type rules. M10 STOPS here
(§55 — no M11 work).
