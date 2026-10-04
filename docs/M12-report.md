# Tide M12 Final Release Report — 1.0.0

Evidence types per spec §89: FACT (code/build/observed), TEST RESULT
(scripted run at the release commit), TOOL RESULT (script/binary
output), STATIC ANALYSIS (audit), MANUAL REVIEW, INFERENCE (labeled).
No推测 is written as fact.

## 1. Executive Summary

M12 froze the M0–M11 feature set and produced **Tide 1.0.0**:
versions unified at 1.0.0 (single source per §15), release blockers
found and fixed during final validation (KI-10 spike prototype in
release nav; **KI-11 P0 — a Tauri state TypeId mismatch that aborted
every real launch seconds after startup**, latent since M8), full
re-validation (317 Rust + 38 frontend tests; 58/58 installer matrix
on the fixed artifact incl. 0.1.0→1.0.0 upgrade; real-launch
survival probe), security/privacy/compliance final audits,
documentation freeze (README/CHANGELOG/release notes/known issues),
and packaged release artifacts with SHA-256, manifest and SBOM.

**RELEASE DECISION: GO** (docs/M12-release-decision.md) — with
documented, non-blocking known limitations (unsigned installer,
no built-in updater, WebView2 codec dependence).

## 2. FACTS

- Versions: package.json = tauri.conf.json = Cargo.toml = exe
  `ProductVersion` = **1.0.0** (TEST RESULT, smoke version-resource).
- 0 occurrences of TODO/FIXME/HACK/`todo!`/`unimplemented!` in all
  Rust/TS source (STATIC ANALYSIS sweep, §77).
- 12 non-test `unwrap/expect/panic` sites, each classified
  (STATIC ANALYSIS): engine/transport constructor & planner
  invariants (`expect("static config"/"planned")`), torrent record
  fields set at creation, spike-only state (compiled out of
  release), one poisoned-Mutex `lock().unwrap()`. None on
  user-input paths (M10 panic audit still holds).
- No secrets/API keys/tokens in source, scripts, docs, or git index
  (§27 sweep + packaging HYGIENE gate PASS).
- `127.0.0.1` appears only in the loopback gateway bind and tests —
  by design (§23).
- identifier `com.ifuyo.tide`, productName `ifuyo-tide`; no dev-stage
  naming in product identity (§16).
- Dev config (`devUrl localhost:5173`) applies only to `tauri dev`;
  release build uses `frontendDist ../dist` (FACT).

## 3. CURRENT_M11_STATE

M11 ended at tag `v0.2.0-rc.1` (RC CONDITIONAL), clean tree, 317+38
tests green, NSIS artifacts packaged, P0=0 with two documented P1s.
M11's installer-matrix evidence predated the final tree — M12's
artifact self-testing (§60) exposed that the M11 "launch" evidence
was racing an undetected startup abort (see §30 RESOLVED).

## 4. M12 OBJECTIVES

Freeze → Audit → Fix release blockers → Validate → Package →
Install/Upgrade → Crash recovery → E2E → Security → Documentation →
Artifact hash → v1.0.0 (§94). No features added (§2 respected —
diff audit §34).

## 5. PRODUCT FREEZE

Feature matrix (§5) — all rows SUPPORTED with evidence:

| Feature | Status | Automated | Manual | Release |
|---|---|---:|---:|---:|
| HTTP/HTTPS Download, Redirect, Range Resume, ETag/LM, Retry, Pause, Cancel | RELEASED | M1 suites | smoke | Yes |
| Crash Recovery, SQLite Persistence | RELEASED | M2 suites + matrix S5/S7/S8 | smoke | Yes |
| Torrent, Magnet, Torrent Resume, Multi-file | RELEASED | M3 suites | manual checklist | Yes |
| Availability, Progressive Read, Range Gateway, Streaming, Seek | RELEASED | M4 suites | manual checklist | Yes |
| Playback | RELEASED | M5 suites (controller/state) | manual checklist | Yes |
| Library | RELEASED | M6 suites | manual checklist | Yes |
| Scheduler | RELEASED | M7 suites | — | Yes |
| Windows Integration, Settings | RELEASED | M8 suites + matrix | manual checklist | Yes |
| Error Reporting | RELEASED | M9 suites | manual checklist | Yes |
| Security Controls | RELEASED | security corpus | audits | Yes |

REMOVED from release: M0.5 spike prototype UI (KI-10 — dev-only).
No mock/fake/TODO implementations ship (§2 FACT).

## 6. RELEASE BLOCKERS

Found and FIXED during M12 (all P0/P1 re-verified on the final
artifact):

1. **KI-11 (P0)** — drain-command Tauri state TypeId mismatch →
   every real launch aborted seconds after startup. Fixed 812573a;
   real-launch probe alive >8s; matrix 58/58 re-run.
2. **KI-10 (P1)** — spike prototype navigation shipped in release.
   Fixed a3365d7 (dev-only gating).

## 7. NON-BLOCKING ISSUES

KNOWN_ISSUES.md: KI-1 (ringbuf advisory, no upstream fix,
reachability-analyzed), KI-2 (DNS-rebinding TOCTOU window) — the
two P1s, each with an explicit §53 "why it does not block 1.0"
rationale; KI-3 (unsigned), KI-4 (no updater), KI-5..KI-9 (P3) —
1.1 backlog.

## 8. FINAL ARCHITECTURE

Unchanged from M0–M11 (frozen): Tauri 2 + Rust + React + SQLite;
core modules download/torrent/streaming/playback/library/scheduler/
platform/security; boundaries per docs/architecture/ and the
milestone deep-dive docs (§79 — no re-architecture; §75 respected).

## 9–17. VALIDATION (DOWNLOAD / TORRENT / RESOURCE / STREAMING /
PLAYBACK / SCHEDULER / PERSISTENCE-RECOVERY / MIGRATION / WINDOWS)

- Full automated suite at the release commit: **317 Rust / 0
  failed, 38 frontend / 0 failed** (TEST RESULT), covering all M1–M9
  chains incl. §44 corruption cases (DB>file, file>DB, final+part,
  completed-without-file, missing-part) and §72 recovery matrix
  decisions (docs/M11-recovery-validation.md §2–3).
- Crash recovery (§13/§72): dedicated suites green (crash-before-
  completion finalizes; DB-ahead never resumes past data; 1000-task
  bounded recovery) + matrix S8 fail-closed corrupt-DB.
- Windows integration (§17/§22 below).

## 16. DATABASE MIGRATION

4 transactional `user_version` migrations; idempotent runner;
`user_version=4` preserved across upgrade/uninstall/reinstall
(matrix S4–S7); fresh-DB first launch requires no manual SQL
(matrix S2/smoke — §47 PASS). No migration history rewritten (§19
FACT).

## 17. WINDOWS INTEGRATION

Single instance (M8 tests), `tide://` deep link + .torrent
association (installer config FACT), tray, autostart, close-to-tray
— code FACT + manual checklist for interactive paths.

## 18. SECURITY AUDIT

- M11 security regression set re-verified green (corpus, gateway
  matrix, SSRF matrix) — TEST RESULT.
- Capabilities minimal + feature-mapped (STATIC ANALYSIS, §25).
- Trust boundary: frontend ≠ authority; all IPC inputs re-validated
  in Rust (FACT, §26).
- Gateway: loopback + random port + token + host/range validation;
  no path-based access (TEST RESULT + FACT, §23).
- Logging policy holds: ids/codes only, no URLs/magnets/paths/
  tokens (STATIC ANALYSIS + MANUAL REVIEW of smoke logs, §28).
- Secrets: none (§2/§27).

## 19. PRIVACY AUDIT

No telemetry/analytics/update-check/crash-reporting/cloud upload
(STATIC ANALYSIS of config+deps; §67/§29). Network behavior matrix
(§68): HTTP download → user URL (user-initiated); torrent → peers/
trackers/DHT (user-initiated); playback gateway → loopback only;
telemetry/cloud → none. PRIVACY.md consistent; packet-capture proof
remains a manual item (KI-6). No ifuyo backend dependency exists
(§69 — the app has no ifuyo endpoints at all, FACT).

## 20. CONTENT POLICY

CONTENT_POLICY.md unchanged and accurate: no discovery/index/
recommendation/DRM bypass; Download-Only default (uploads disabled
— verifiable `disable_upload` feature); no anonymity/legal marketing
in any shipped copy (MANUAL REVIEW of strings + README/CHANGELOG
§30/§31).

## 21. THIRD-PARTY LICENSE AUDIT

deny.toml allow-list (MIT/Apache-2.0/BSD/ISC/Unicode/Zlib/0BSD/
Unlicense/CC0/BSL/MPL-2.0/OpenSSL/CDLA); `release/DEPENDENCIES.txt`
generated from Cargo.lock/package.json (518 entries, TOOL RESULT);
THIRD_PARTY_NOTICES.md consistent; test media is self-generated via
scripts/generate-test-media.sh (ffmpeg-synthesized patterns — no
third-party media shipped, §33). WebView2 is a runtime component of
Windows, not redistributed (§57: Required, installer bootstraps).

## 22. INSTALL / UPGRADE / UNINSTALL TEST

`scripts/install-matrix.ps1` on the FINAL fixed artifact:
**58 checks PASS / 0 FAIL** (TEST RESULT):
fresh install 0.1.0 → first-launch seed → upgrade-with-app-running
(defined: force-close) → 0.1.0→1.0.0 upgrade verified (exe+registry)
→ upgrade data preservation (seeded row + user_version=4 across the
boundary) → uninstall (app data preserved, user files untouched) →
reinstall → corrupt-DB fail-closed (structured fatal, no silent
corruption). Non-admin throughout (§16). §20 interrupted-task
upgrade: byte-level semantics engine-tested (M11-recovery §2);
UI-level live-download upgrade remains a manual item.

## 23. CLEAN MACHINE TEST

Dev-host simulation with isolated app-data roots (§81): fresh
zero-state launch, per-user install without elevation, no dev-tool
dependency (WebView2 bootstrapped). Third-party physical machine:
NOT done (KI-5). Honest scope per §92.

## 24. LONG-RUN TEST

`m9_soak` suite green (extended-run behavior, bounded registries) —
TEST RESULT. Idle budget on the installed build: CPU ~0%/8s samples
(TOOL RESULT, smoke ×3). Overnight/manual long-run observation
remains on the manual checklist (§40 honesty).

## 25. PERFORMANCE

Cold start → `app.started`: 1088–1755 ms (installed builds, TOOL
RESULT). Idle CPU ~0%. 1000-task recovery bound PASS. Throughput/
memory instrumentation: not in scope for 1.0 (manual items
recorded, §53 of M11 matrix).

## 26. ARTIFACTS

`release/`: `ifuyo-tide_1.0.0_x64-setup.exe` (6,731,302 bytes),
`SHA256SUMS.txt`, `RELEASE-MANIFEST.json` (channel stable, commit,
toolchain, locks, signing status), `DEPENDENCIES.txt`,
`RELEASE-NOTES.md` (copied from docs/release-1.0.0.md source set —
packaged from root RELEASE-NOTES.md pointer). §58 installer
security: no debug payload/test media/secrets (HYGIENE gate TOOL
RESULT). §60 artifact self-test EXECUTED (matrix above + real-launch
probe) — not a developer-dir run.

## 27. SHA-256

Installer `ifuyo-tide_1.0.0_x64-setup.exe`:
`eafe679605677b6eb2da6e530e653b9138f41d56c4e53c580c2141c3b6a7e910`
(recomputed at final packaging on the tagged tree; see
release/SHA256SUMS.txt for the authoritative value).

## 28. KNOWN LIMITATIONS

CHANGELOG.md "Known Limitations" + KNOWN_ISSUES.md: unsigned
installer (SmartScreen), no built-in updater, WebView2 codec
dependence (H.264/VP9 supported; HEVC/AV1 conditionally supported —
environment-dependent, §34/§52), per-session torrent rate limiting,
DNS-rebinding validate→connect window, stderr-only fatal startup
reporting.

## 29. P0 / P1 / P2

- P0 = **0 open** (KI-11 found→fixed during M12).
- P1 = 2 open, non-blocking with §53 rationales (KI-1, KI-2).
- P2 = 2 (KI-3 signing, KI-4 updater) + P3 ×5 → 1.1 backlog.

## 30. RESOLVED

- [P0][RESOLVED] KI-11 drain state TypeId mismatch aborted real
  launches (812573a).
- [P1][RESOLVED] KI-10 spike prototype in release navigation
  (a3365d7).
- [P1][RESOLVED] KI-8 release-profile stub compile failure (M11,
  c6a238c).

## 31. CONFIRMED

- [P1][CONFIRMED] KI-1 ringbuf advisory — no upstream fix;
  reachability analysis documented.
- [P1][CONFIRMED] KI-2 DNS-rebinding window — NOT GUARANTEED,
  default-deny policy limits exposure.
- Security boundaries hold under the M10/M11 audit sets (green).

## 32. OPEN

KI-3 (signing), KI-4 (updater), KI-5 (third-party machine),
KI-6 (DPI walkthrough + packet capture), KI-7 (clippy backlog),
KI-9 (startup-failure dialog), manual checklist items
(docs/M11-test-matrix.md §4).

## 33. DEFERRED TO 1.1+

KI-3/KI-4/KI-5/KI-6/KI-7/KI-9, pinned-resolution SSRF connector,
per-task torrent rate limiting, startup-failure dialog, i18n locale
swap (strings centralized in M9), throughput/memory instrumentation.

## 34. CHANGED FILES

M12 range (`v0.2.0-rc.1..HEAD`): version sources (package.json,
tauri.conf.json, Cargo.toml+lock), `src/app/App.tsx`,
`src/components/Sidebar.tsx` (spike gating),
`src-tauri/src/commands/windows.rs` (P0 fix), scripts
(release-smoke/package-release/install-matrix), docs (M12 report +
decision + release-1.0.0.md), CHANGELOG.md, KNOWN_ISSUES.md,
README.md. Diff audit (§84): no secrets, no personal paths, no
temp/test data, all changes M12-related.

## 35. RELEASE DECISION

**GO** — evidence per docs/M12-release-decision.md. Conditions
already satisfied; documented non-blocking limitations accepted for
1.0 with 1.1 scheduling.

## 36. DEFINITION OF DONE

§87 checklist: all items verified — see
docs/M12-release-decision.md for the item-by-item record with
evidence pointers.
