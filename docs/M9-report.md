# M9 Final Report

Per §141 — covers BOTH halves of M9.

## 1. Executive Summary

M9 converts the M0–M8 engineering core into a product-quality
application: unified error copy with recovery actions, per-view crash
isolation, one formatting system, honest loading/empty states, a
first-launch welcome flow, launch-request intake with confirmation,
Windows settings/notifications UI, terminology + i18n centralization,
theme/a11y hardening, and a reproducible soak harness. 313 Rust tests
and 38 frontend tests all pass; no core behavior changed (§2 boundary).

## 2. FACTS

- Rust 313/0 (`cargo test --locked`); frontend 38/0 (`pnpm test`);
  `tsc` clean; `cargo fmt --check` clean; clippy zero warnings in
  lib code (remaining test-target warnings auto-fixed or inventoried);
  `pnpm tauri build` produces exe + NSIS installer.
- Formatting/property fuzz: 700+ random magnitudes through the
  formatters — no NaN/Infinity/negative output (§101).
- Soak verified: 60 s / 502 churn iterations, registry bounded.

## 3. CURRENT M0–M8 STATE

All suites unchanged and green through every M9 commit (each commit
was built+tested before the next, §135). M9 touched frontend
presentation, onboarding, and docs; the only Rust changes were the
onboarding settings commands (settings-KV reuse, §163 no schema
change).

## 4. M9 OBJECTIVES

§2 quality model applied per-area: Discoverability (welcome flow,
empty states), Understandability (honest loading states, state copy),
Feedback (confirm banners, notifications), Recoverability (error copy
+ actions), Consistency (one formatter, one string source, one state
vocabulary), Efficiency (double-submit guard, budget test), Delight
(quiet motion, reduced-motion).

## 5. UX AUDIT

docs/M9-ux-audit.md — §5 walk-through findings with fix/status per
row.

## 6. DESIGN SYSTEM CHANGES

docs/M9-design-system.md — tokens centralized (:root spacing/radius/
transition), button hierarchy (primary/small/danger), status pills
text+tint+glyph, motion rules.

## 7. DOWNLOAD UX

Primary/secondary/tertiary info hierarchy (§8); failed banner explains
and offers recovery; technical details collapsed; double-submit guard;
per-task priority selector (M7) retained.

## 8. TORRENT UX

File rows with selection/size; phase text from the engine; no new
torrent capabilities (§35).

## 9. PLAYER UX

Buffer/waiting distinction via REAL element telemetry (§34); seek
feedback through seeking state + Timeline preview; unsupported media
escapes to the OS default player (§33).

## 10. LIBRARY UX

"File unavailable" + Recheck (§38); missing-file Play disabled with
title explanation; Remove vs Delete-file split (§71); scan progress
with real counts.

## 11. SETTINGS UX

Grouped sections (Windows / Bandwidth / Library folders); every
setting shows label + value + effect; destructive actions
(Remove root, Delete file) are explicit and separated (§40/§71).

## 12. ERROR / LOADING / EMPTY STATES

Every view: honest Loading (§110), explanatory Empty (§18), failure
banner with recovery (§85). No fake data anywhere (§77/§138).

## 13. ACCESSIBILITY

aria-labels on every icon-only control; ARIA slider Timeline; status
never color-only (icon+text); reduced-motion supported; keyboard map
in the player; contrast spot-checked ≥4.5:1 (§25/§128).

## 14. WINDOWS / DPI

M8 integrations unchanged; window min-size enforced; DPI/multi-
monitor rows remain the manual installed-build pass (§30/§77/§128).

## 15. PERFORMANCE

docs/M9-performance.md — all numbers reproducible; no new polling;
idle-quiet by construction (§48/§49).

## 16. LONG-RUN STABILITY

Soak harness committed (§47) + cleanup audit (§106): every listener/
timer/session has a matched teardown; registry bounded under churn.

## 17. RACE CONDITION / STATE AUDIT (§46)

State flows backend → IPC → projection (§44); launch requests drain
once (§132); seek flow is cancel-then-submit (idempotent, tested);
the double-submit guard prevents UI-initiated duplicates; backend
state machines remain the sole authority (§45/§133).

## 18. SECURITY REGRESSION (§125)

URL/path validation, gateway tokens, host restriction, capabilities —
all unchanged and re-verified by the M4/M6/M8 suites; no sensitive
logging introduced (§30/§54); no telemetry creep (§31/§126); no cloud
dependency (§127).

## 19. TEST RESULTS

313 Rust / 0 failed; 38 frontend / 0 failed; tsc, fmt, clippy clean;
tauri build PASS.

## 20. MANUAL ACCEPTANCE (§129 Flows A–F)

Flows A–F are covered step-by-step by automated layers (M1–M8 suites
+ frontend tests); the in-window visual click-through remains the
recorded manual pass — same disclosure as M4–M8 reports (§169
honesty: flows are IMPLEMENTED + TESTED at layer level; the visual
run is NOT YET VALIDATED on this machine).

## 21. REGRESSION MATRIX

docs/M9-regression-matrix.md — M1–M8 all PASS with suite names;
NOT TESTED rows called out explicitly (§124).

## 22. CHANGED FILES

Frontend: lib/{format,strings}.ts, components/ViewErrorBoundary.tsx,
features/{tasks,library,player,settings,onboarding}/**, app/App.tsx,
styles/global.css, test/*. Backend: commands/{settings,mod}.rs.
Docs: six M9 files.

## 23. DEPENDENCY CHANGES

`@tauri-apps/plugin-notification` added in M8 (already present before
M9). No new dependencies in M9 (§114).

## 24. FACT / HYPOTHESIS / INFERENCE

- FACT: all listed tests green; no backend behavior change.
- HYPOTHESIS: the 500 ms/1 s UI cadences stay invisible to users at
  1000-task scale (frontend budget test + M7 benchmark support this).
- INFERENCE: strings.ts unblocking means locale support is now a
  translation pass, not a refactor.

## 25. RESOLVED

- Technical errors surfaced raw → mapped copy + recovery.
- Views could white-screen → error boundaries.
- Fake-empty flash before first backend response → honest Loading.
- Double-submit duplicates → guard.
- EN/ZH copy mix → centralized single-language strings.

## 26. CONFIRMED

- M0–M8 behavior unchanged under the polish (regression matrix).
- No telemetry/cloud additions (§126/§127).

## 27. OPEN

- In-window visual demos (manual, §129).
- Locale framework swap (structure ready).
- Overnight soak execution on a desktop session (harness committed).

## 28. WORSENED

Nothing — all pre-M9 suites green throughout.

## 29. P0

0.

## 30. P1 (recorded)

- English-only UI pending the locale swap.
- High-DPI/multi-monitor + overnight soak = manual passes.
- Tray tooltip strings not yet routed through strings.ts (M8 tray is
  Rust-side; noted for the i18n pass).

## 31. KNOWN LIMITATIONS

Visual regression is checklist+manual (no snapshot framework — §98
explicitly allows this); reduced-motion collapses all transitions
including the view fade (intended); onboarding skip rule ties to the
download-dir existence check (documented §6).

## 32. M10 READINESS (§157 boundaries kept)

Security posture unchanged and re-verified (§125): validation, tokens,
capabilities, logging all intact — M10 hardening can proceed on a
stable, polished product with no new attack surface from M9.

## 33. M9 DEFINITION OF DONE

§140 A–H checklists: UX (flows walked, states complete, actions
truthful) ✓; Visual (tokens, consistent buttons/status/copy) ✓;
Accessibility (keyboard, focus, reduced-motion, long-text) ✓ —
high-DPI visual pass = manual; Reliability (race audit, teardown
audit, soak harness, no panics) ✓; Performance (benchmarks recorded,
no busy loops) ✓; Regression (M1–M8 PASS) ✓; Build (all gates PASS)
✓; Documentation (six M9 docs) ✓.

Per §143: M9 STOPS here — no M10 work.
