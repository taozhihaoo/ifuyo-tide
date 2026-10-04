# M4 Report

Per M4(下) §212. Facts over adjectives; every open point is named.

## 1. Executive Summary

M4 makes Tide a progressive streaming player: a finished file, an
in-progress HTTP download and a torrent file all stream through ONE
loopback Range Gateway into ONE WebView2 player. The player reaches
the download frontier, WAITS event-driven, and continues; seeks into
never-downloaded regions are served by origin read-through; seeks in
torrents wait on real pieces; everything is bounded, revocable and
byte-exact. 235 Rust tests (incl. 17 M4 streaming integration tests
over real swarms/origins) + 3 frontend suites pass; clippy/rustfmt
clean in M4 code; release build verified. Two P0-class defects were
found BY the acceptance tests and fixed (see §21).

## 2. FACTS

- Rust: 235 passed / 0 failed (`cargo test --locked`), including
  `m4_streaming` 7/7 and `m4_lower` 10/10.
- Frontend: vitest 3/3; `tsc --noEmit` clean; `pnpm tauri build`
  produces `target/release/ifuyo-tide.exe`.
- `cargo fmt --check` clean; `cargo clippy` — 0 warnings in
  M4-owned code, 12 pre-existing warnings inventoried in §27.
- Measured performance (§19): gateway ~370–575 MiB/s, session
  open+close ~3 µs, scheduler ~0.9 µs/op, read-through seek ~3 ms,
  frontier-wait read continues at frontier pace.
- librqbit 9.0.1, reqwest 0.13.5 (default-tls, identity encoding),
  no new runtime dependencies added in M4.

## 3. M0–M3 Baseline

Unchanged and regression-proven: M1 HTTP engine (sequential append,
flush per chunk), M2 crash recovery (resume point = physical size),
M3 torrent engine (single session, DownloadOnly). M4 added no
migrations and no engine rewrites — the durable-len watch is an
additive notification after the existing flush point.

## 4. Resource Architecture

`StreamingResource::{File, Part, Torrent, Http}` — enum, not trait
(M0.5 finding). Full description: docs/M4-resource.md. The `.part`
variant (Part) is the M4(下) addition: dynamic durable length,
event-driven frontier wait, origin read-through.

## 5. Availability Architecture

Coarse `Availability` + exact `available_len()` (physical size for
`.part`, rqbit progress for torrents). Update order: write → flush →
watch fired → readers wake (§112). Interval `AvailabilitySet`
deferred with recorded constraints: docs/M4-availability.md.

## 6. Demand Architecture

Demand = (resource_id, range, reason, priority, state). In-flight
merge by (resource, range, reason) — §114; eager removal of terminal
demands — §95. docs/M4-demand.md.

## 7. Scheduler

Four classes, reason defaults, oldest-first, dedup/merge/cancel/
reprioritize. ~0.9 µs per submit+complete. Docs/M4-demand.md.
Bandwidth throttling of background downloads remains M7 (recorded
boundary §205).

## 8. HTTP Progressive Streaming

`.part` frontier wait + origin read-through; 200-to-Range rejected
as `RESOURCE_RANGE_UNSUPPORTED`; stream-limited origin reads;
network error classification (§143). docs/M4-http-progressive.md.

## 9. Torrent Progressive Streaming

Independent FileStream per read; piece waits bounded (120 s);
byte-exact at non-piece-aligned offsets; paused torrents auto-resume
for playback. docs/M4-torrent-progressive.md.

## 10. Stream Gateway

Raw HTTP/1.1 on 127.0.0.1:{random}; Host check; opaque tokens;
single-range 200/206/416/400; 405 for non-GET/HEAD; 503-before-
headers for missing resources; 256 KiB bounded chunks; per-session
cancellation watch; redacted logs. docs/M4-stream-gateway.md.

## 11. PlaybackSession

Separate lifecycle from tasks (§52/§53); owner-tracked; task deletion
invalidates its sessions (§102 — chosen policy); runtime-only tokens
(§70); capability formed at creation (§146/§147); live availability
refresh (§149). docs/M4-playback.md.

## 12. WebView2 Integration

`<video src=http://127.0.0.1:{port}/stream/{token}>` — bytes over
HTTP Range, never over IPC (ADR-8; M0.5 measured). Autoplay policy
arg from M0.5 retained. UI: one player, capability-aware seek,
Browser-buffered vs Backend-available shown as distinct metrics,
honest "unavailable" values (§148/§149/§65).

## 13. Seek Behavior

- HTTP seek beyond frontier: 2 s grace for local growth, then origin
  read-through of exactly the missing tail (~3 ms measured).
- HTTP seek within downloaded region: local, immediate.
- Torrent seek: FileStream read pressure; waits for pieces; bounded.
- Rapid seeks: `notify_seek` cancels stale demands (§80/§130).
- Media-level behavior (moov placement etc.): M0.5 compatibility
  matrix remains the authority; no demuxer built (§166).

## 14. Cancellation

Three axes (demand / session / owner-task), all implemented and
tested; a read blocked on never-arriving data aborts promptly on
session close (≪ the 30 s read timeout in the test). ADR-10.

## 15. Persistence / Recovery

Nothing new persisted in M4 (§160 allowance). Restart: M2 recovery
reconstructs `.part` state (physical size), M3 recovery re-adds
torrents paused; new playback sessions are created fresh — proven by
Critical #7 with a real engine restart over the same repository.

## 16. Security

docs/M4-security.md. Arbitrary-file-read P0 denied by construction
(no path ever leaves a token→resource map); full §100/§101/§125
matrix tested; capabilities unchanged (core + dialog only); no CORS
headers; log redaction audited.

## 17. Test Matrix

docs/M4-test-matrix.md — 235 Rust + 3 frontend tests mapped
requirement-by-requirement to the spec sections above.

## 18. End-to-End Results

- **HTTP-level E2E**: PASS — real DownloadEngine + real fixture
  origin + gateway + wire-level Range clients (Critical #1/#2/#7/#9,
  fuzz, security).
- **Torrent-level E2E**: PASS — real seeder swarm + production
  engine + gateway (m4_streaming suite).
- **WebView2 visual E2E**: the interactive browser run of the final
  production build was validated in M0.5 with measured data (the
  spike gateway implements the same protocol shape), and the M4
  frontend wiring is typechecked + unit-tested, but a scripted
  in-window UI test of the release build was NOT automated in M4 —
  recorded as an open item for M5 (which owns the player UX anyway).
  Manual steps: start a download → Play → watch past the frontier →
  seek beyond it.

## 19. Performance (measured, release-mode dev profile)

| Metric | Value | Source |
|---|---|---|
| Gateway throughput (64 MiB local file, full GET) | ~370–575 MiB/s | `performance_baseline_recorded` |
| Playback session create+close | ~3 µs/cycle | same |
| Scheduler submit+complete | ~0.9 µs/op | same |
| Read-through seek (origin round trip) | ~3 ms | same |
| Frontier-wait read | continues at frontier pace (60 ms/64 KiB fixture) | Critical #1 |
| Memory | bounded: 1 ≤256 KiB chunk in flight per connection; demands removed eagerly | code audit |
| CPU | no polling anywhere in the streaming path | §138 audit |

## 20. FACT / HYPOTHESIS / INFERENCE

- FACT: all §126–§134 critical chains pass as integration tests.
- FACT: WebView2 plays/seeks through a loopback Range gateway (M0.5
  measured; same protocol shape in production).
- FACT: an origin that ignores Range is detectable and rejected.
- HYPOTHESIS: WebView2's re-request behavior after a frontier-truncated
  response is reliable across media containers — supported by M0.5
  data, re-verified only at the HTTP layer in M4.
- INFERENCE: origin read-through will be replaced/layered by the
  Range-Writer persistence when random ranges become persistent
  (constraints pre-recorded) without gateway changes.

## 21. RESOLVED / CONFIRMED / OPEN / WORSENED

- RESOLVED: read-through fetched from the wrong offset when the whole
  range was beyond the frontier (wrong-bytes class) — found by
  Critical #2, fixed, regression-tested.
- RESOLVED: demand map retained terminal entries forever (unbounded
  growth, O(n²) merge scans) — found by the scheduler benchmark,
  fixed (eager removal + real dedup).
- RESOLVED: stopped sessions previously kept serving; close/reap now
  revoke gateway access and abort in-flight reads.
- CONFIRMED: `disable-upload` torrent posture, M2 recovery invariants
  — regression suites still green.
- OPEN: scripted WebView2 visual E2E of the release build (§18).
- OPEN: interval AvailabilitySet + RangeWriter persistence (deferred
  by design, constraints recorded).
- WORSENED: nothing — all pre-M4 suites pass unchanged.

## 22. P0 / P1

- **P0 count: 0** (both P0-class findings were fixed before
  completion; §200 list re-audited item by item).
- P1 (recorded, non-blocking): multi-range unsupported (§62 policy);
  torrent far-seek waits on swarm order (no piece-priority API);
  coarse torrent availability; read-through ranges not persisted;
  no If-Range on read-through; origin ETag change mid-read-through
  yields a structured error and a retry rather than a graceful
  restart; UI is deliberately minimal until M5.

## 23. Known Limitations

See §22 plus: WebView2 visual E2E harness (§18); scheduler does not
throttle background bandwidth (M7); `notify_seek` invalidates demand
bookkeeping but the media element's own byte requests are the real
seek driver.

## 24. Changed Files

Backend: `core/streaming/{resource,demand,gateway,playback,range,mod}.rs`,
`core/download/engine.rs` (durable-len watch + sync lookup),
`error.rs` (StreamingFailure taxonomy), `commands/{playback,download,
torrent}.rs`, `app/{mod,state}.rs`. Frontend:
`features/player/{VideoPlayer,playbackApi}.tsx/.ts`, `app/App.tsx`,
Downloads/Torrents views, styles. Tests: `tests/m4_streaming.rs`,
`tests/m4_lower.rs`. Docs: this set. (Commit-by-commit in git log.)

## 25. M5 Readiness

The player has a real, safe, seekable byte supply with capability
metadata and honest buffer state — exactly the substrate M5's
player UX (media info, controls polish, queue, volume, fullscreen)
needs. Open M5 items: the visual E2E harness and the M0.5-format
compatibility matrix re-run against the production gateway.

## 26. M7 Readiness

The scheduler's demand model is the extension point M7 needs
(reasons, priorities, telemetry-ready structure). Missing and
deliberately deferred: throughput estimation, background throttling,
fairness — the demand map and gateway hooks are where they land.

## 27. Definition of Done

| Item | Status |
|---|---|
| `cargo check --locked` | PASS |
| `cargo test --locked` | PASS — 235/0 |
| `cargo clippy --locked` | M4 code 0 warnings; 12 pre-existing (transport classifier if-same-else ×2, task module_inception, SpikeResource len, `&PathBuf` params ×3, too_many_arguments, complex type, while-let, spike dead fields) — inventoried, not M4 regressions |
| `cargo fmt --check` | PASS |
| frontend typecheck / tests / build | PASS / 3·3 / PASS |
| `pnpm tauri build` | PASS (release exe) |
| E2E HTTP playback / seek | PASS (integration-level, §18) |
| E2E Torrent playback / seek | PASS (integration-level) |
| Gateway security | PASS (full matrix) |
| Restart recovery | PASS (Critical #7) |
| Performance baseline | RECORDED (§19) |
| Documentation | COMPLETE (M4-resource/availability/demand/stream-gateway/playback/http-progressive/torrent-progressive/security/test-matrix/adr/persistence-boundaries/report) |

## §211 — The 20 questions, answered

1. HTTP 边下边播? **是** — Critical #1（frontier 等待→续播）。
2. HTTP seek 到未下载区? **是** — Critical #2（origin 读穿透，~3ms）。
3. Torrent 未完成可播? **是** — m4_streaming 尾部等待测试。
4. Torrent seek 产生新 Demand? **是** — `seek_cancels_stale_demands`。
5. Gateway 完全 loopback? **是** — bind 127.0.0.1:0 + Host 校验。
6. 无法访问任意文件? **是** — token→resource 映射，无路径代码路径；§100 全矩阵。
7. Token 不可预测? **是** — UUIDv4，运行时唯一，日志脱敏。
8. Resource 统一 HTTP/Torrent? **是** — Critical #8 同一 gateway/播放器。
9. Availability 表示 byte ranges? **前缀级** — `available_len()` 精确（.part 物理长度）；区间集按 ADR-2 延后。
10. Scheduler 影响网络/读取优先级? **部分** — 记录/取消/合并真实生效；带宽节流属 M7（已记录边界）。
11. 旧 seek 被取消/降级? **是** — Critical #5。
12. Restart 后 Resource 恢复? **是** — Critical #7。
13. 数据损坏风险? **无已知** — 字节保持测试 + §118 撕裂测试 + 唯一的错误字节缺陷已在 §21 修复。
14. 是否有 P0? **0**（两个 P0 级缺陷在完成前修复）。
15. WebView2 限制: moov-at-end 容器需要尾部请求（读穿透已覆盖）；不支持格式无法解码（UI 如实提示）— M0.5 矩阵为准。
16. HTTP server 限制: 忽略 Range 的源 → 该资源禁用随机访问（检测+记录，§145）。
17. rqbit 限制: 无 piece 优先级 API，远端 seek 等待 swarm 顺序（P1 记录）。
18. 留到 M5: 播放器 UX、视觉 E2E 自动化、格式矩阵对生产网关复测。
19. 留到 M7: 带宽节流/公平性/吞吐估计（demand 模型即扩展点）。
20. 满足进入 M5 的条件? **是** — §25。

## §199 Audit summary

FACTS: §2. INFERENCES: §20. CONFIRMED: §21. RESOLVED: §21 (3 items).
OPEN: §21 (2 items, non-blocking). WORSENED: none. Risks use only
P0/P1 (§22). Per §215: all stop conditions met → M4 STOPS here; M5
is NOT started.
