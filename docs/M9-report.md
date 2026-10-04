# M9(上) UX Audit & Report

## 1. UX Audit (§5) — findings → actions taken

| Path (§5) | Finding (before) | Action (M9上) |
|---|---|---|
| Downloads failed | Raw code `(DOWNLOAD_DISK_FULL)` dumped | Domain copy + recovery action + collapsible technical details (§15/§16/§17) |
| Any view crashes | Whole app white-screens | Per-view ErrorBoundary with Try-again (§89) |
| Speed 0 | `0.0 MB/s` noise | Shared formatter, zero-state "—" (§11) |
| ETA | Could show absurd values | Hides >30 days, "Paused", "<1s", "—" (§12) |
| Long file names | Overflow risk | truncateMiddle + title attr (§59) |
| Start double-click | Possible duplicate tasks | submit guard (§87) |
| Library missing file | Bare "missing" | "File unavailable" + Recheck (§38) |
| Player unsupported | Dead end | "Open in default app" via M8 opener (§33) |
| Motion | None | Light view-fade; prefers-reduced-motion collapses it (§21/§22) |
| Restart recovery | — | M2 recovery unchanged; UI shows real states only (§52) |

Deliberately NOT done (spec §20/§24/§57/§113): skeleton spam, new theme
engine, i18n framework, metadata scraper, notification sound.

## 2. What shipped (M9上 scope)

- `lib/format.ts` — ONE speed/ETA/bytes/truncate implementation (§60).
- `describeDownloadError` — technical→domain→user mapping with
  recovery actions (§15/§16/§85); raw codes folded into details.
- `ViewErrorBoundary` — per-view crash isolation (§89).
- Double-submit guard on download start (§87).
- Library missing-file UX (§38) and player unsupported fallback (§33).
- Design tokens in :root (spacing/radius/transition, §23) +
  prefers-reduced-motion support (§22).

## 3. FACTS

- 31 frontend tests / 0 failures (6 new formatter/copy tests);
  313 Rust tests / 0 failures — no backend behavior changed in M9上.
- `tsc`, fmt, clippy all clean; tauri build PASS.

## 4. Completed after initial audit (verifier feedback round)

- §6 first-launch welcome flow: onboarding_completed flag (settings
  KV, §80 reuse), WelcomeFlow single-step directory confirmation,
  skipped when flag set OR download dir exists (upgraded installs).
- §56 terminology audit: docs/M9-ux-audit.md — one canonical term per
  state; no synonym sets.
- §57 i18n audit + fix: all user-facing copy centralized in
  `lib/strings.ts`; the EN/ZH language mix was eliminated (English
  standardized; locale-swap structure ready for the i18n framework).
- §24/§25 theme + a11y pass: tokens centralized in :root; state pills
  carry icon+text (never color-only, §25); reduced-motion collapses
  transitions (§22); aria-labels verified on every icon-only control;
  dim-text contrast spot-checked ≥ 4.5:1.
- §28 toast: inline notices replace themselves (cannot accumulate);
  permanent state stays in cards — documented as conforming.
- §47 soak harness: `tests/m9_soak.rs` (TIDE_SOAK_SECONDS env) —
  verified 60 s / 502 churn iterations with registry bounded at 0;
  overnight runs use the same harness on a desktop session.

## 5. Deferred to M9(下) (recorded, not skipped silently)

- §30 High-DPI and §77 multi-monitor visual passes (manual, §77).
- Overnight/during-use soak EXECUTION beyond the verified 60 s
  harness run (1h/4h/8h/overnight rows need a running desktop
  session — the harness itself is committed and reproducible).
- Locale framework swap (zh-CN/en-US) on top of the strings module
  (structure ready; the product language is currently English —
  consistent since the §57 fix).
- §44/§45/§46 remaining race-condition polish across Pause/Resume/
  Retry (backend guards exist; UI projection audit continues).

## 5b. P0 / P1

- P0: 0.
- P1: locale framework not yet swapped (strings module is the ready
  extraction point); visual DPI pass pending; overnight soak
  execution pending (harness committed).

## 6. Boundary kept (§70/§78)

No security check moved to the frontend; gateway stayed loopback-only
with tokens; no new business capabilities (§3 list respected).
