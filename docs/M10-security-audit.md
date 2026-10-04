# M10 Security Audit (§6–§18 decision table + findings)

Per §43 decision table. Evidence types per §54: FACT / TEST RESULT /
STATIC ANALYSIS / TOOL RESULT / MANUAL REVIEW / ASSUMPTION.

| Area | Current | Risk | Evidence | Status | Action |
|---|---|---|---|---|---|
| Tauri Capability | core:default + dialog:allow-open + notification (3) + autostart (3) + deep-link (1) | LOW | capabilities/default.json audit (M8 §19); no fs/shell/process/http | CONFIRMED | none — re-audited M10 |
| CSP | self + media 127.0.0.1:*/localhost:* + ipc connect | LOW | tauri.conf.json (M0.5/M8 verified); no remote script/CDN | CONFIRMED | none — gateway needs the loopback media origin, not `*` |
| IPC | 53 production commands + 18 dev-only spike stubs | LOW | commands/* audit: ids re-validated, no raw SQL, no path-from-frontend; spike stubs fail-closed in release | CONFIRMED | — |
| URL/SSRF | scheme allowlist + private-IP block + per-hop redirect re-validation (bounded) | MEDIUM-LOW | M1 transport code + security_corpus tests | CONFIRMED (IP literals) / PARTIALLY VERIFIED (DNS rebinding TOCTOU — recorded NOT GUARANTEED, §9) | M7/M11: consider pinned-resolution connector |
| Filesystem | canonicalize + component-wise containment + reserved-name + reparse checks (M1/M6) | LOW | m1/m6/security_corpus suites (traversal matrix, junction, RTLO, reserved names) | CONFIRMED | — |
| Torrent | librqbit 9.0.1 boundary; path validation; DownloadOnly; no UPnP; metadata path checks | LOW | M3 suites + supply-chain audit | CONFIRMED | ringbuf advisory documented (supply-chain) |
| Stream Gateway | loopback-only, UUIDv4 token, Host check, single-range, bounded chunks, revoked on stop | LOW | M4/M8 gateway suites incl. security matrix | CONFIRMED | — |
| WebView2 | self-origin UI; navigation not intercepted beyond OS default; no ExecuteScript injection; no remote debugging in release | LOW | tauri.conf + code audit | CONFIRMED | — |
| SQLite | parameterized queries only (repositories); migrations transactional; app-data location | LOW | M2 suite + code audit | CONFIRMED | — |
| Logging | ids/counts/reasons/codes only; redaction rules enforced since M1 | LOW | log audit (M4 §30, M6 §64, M10 §40) | CONFIRMED | — |
| Privacy | no telemetry/analytics/cloud/uploader — network-surface inventory confirms only user-URL/peer/gateway traffic | LOW | docs/M10-network-surface.md (code-level inventory) | CONFIRMED | runtime packet-capture pass = manual (§35) |
| Dependencies | cargo-deny gate: 1 ignored vulnerability (documented), 2 ignored unmaintained, licenses allowlisted | LOW | cargo deny check (TOOL RESULT) | CONFIRMED | ringbuf P1 on librqbit-utp update |
| License | MIT OR Apache-2.0 (app); permissive tree | NONE | cargo deny check licenses | RESOLVED | — |
| Installer | NSIS per-user; .torrent association (Editor); tide:// scheme; no user-data deletion | LOW | M8 §83 smoke + installer config audit | CONFIRMED | full uninstall loop = manual |
| Signing | UNSIGNED; documented signing path (release-hardening doc) | NONE (dev build) | docs/M10-release-hardening.md | CONFIRMED | certificate acquisition = M11 decision |
| Updater | NOT PRESENT; architecture requirements documented | NONE | docs/M10-release-hardening.md | CONFIRMED | — |
| CI/CD | NO CI workflow exists in this repo | NONE (no pipeline to audit) | repo audit | NOT APPLICABLE | M11: release workflow with minimal permissions when CI lands |

## Panic audit summary (§29)

197 unwrap/expect sites classified:
- startup invariants (paths/config at boot) — fail with fatal_startup,
  by design (§51)
- format-utility invariants pre-vetted by property tests (M9 §101)
- download/torrent state internals — guarded by state machines and
  313-test suite; the M9(下)/M10 fuzz corpora cover external-input
  paths
- ZERO unwraps on the external-input paths added by M8/M9/M10
  (platform launch parsing, library validation, scheduler commands —
  all return structured errors)

## Resource exhaustion (§30)

Bounded by construction: 16 KiB headers, 256 KiB chunks, single-range
policy, bounded redirects, per-task rate limit, demand registry with
eager removal, single-range gateway streaming (no whole-file buffers),
64 KiB launch-arg cap, library scan batched transactions, 12h session
TTL, ring-buffered decisions (256). No unbounded queues discovered.
