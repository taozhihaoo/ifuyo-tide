# M9(上) UX Audit (§5) — terminology, i18n, theme, a11y

The §5 walk-through findings and their resolutions. Items marked
FIXED have code changes in this milestone; items marked DOCUMENTED
are verified-as-acceptable with the rationale recorded.

## §56 Terminology audit (FIXED)

| Term | Canonical form | Where unified |
|---|---|---|
| Task state | Created / Ready / Running / Paused / Completed / Failed / Cancelled | `lib/format.ts STATE_LABELS`; all views render `d.state`/`t.state` directly — no synonyms anywhere |
| Speed | `128.0 KB/s` via formatSpeed | Downloads debug panel, task cards |
| Size | formatBytes | Downloads/Library/Player |
| ETA | formatEta ("—"/"Paused"/"2m 5s") | Downloads |
| Remove vs Delete | "Remove" = index only; "Delete file" = destructive | Library actions (M6 §24 kept) |

No view uses synonym sets ("Running/Downloading/Active") — a
grep-level audit of `src/features/**` confirmed one term per state.

## §57 i18n audit (FIXED to a single source)

**Finding**: user-facing copy was scattered and language-mixed —
Chinese strings in the player (errorModel, StatusOverlay, buttons)
against English everywhere else.

**FIX**: every user-facing string now lives in `lib/strings.ts`
(single module = the extraction point for a future i18n framework);
product language standardized to English across all views. Audit
grep over `src/features/**` + `src/components/**` shows no remaining
hardcoded Chinese; format pluralization/units live in the formatter
(§60 one-implementation rule).

**Recorded**: full locale framework (zh-CN + en-US swap) remains
M9(下)/later work — the structure is ready.

## §24 Theme consistency (FIXED)

- Design tokens centralized in `:root` (§23): spacing 1–4, radii
  sm/md/lg, transition durations.
- State pills carry text on tinted backgrounds (never color-only).
- Player overlay text uses dark gradient + white text (readable on
  any video, §24 "player overlay 不可读" check).
- Borders/shadows/dim text flow through the `--color-*`/`--text-*`
  variables; a grep audit found no raw hex outside the token block
  and the player surface.

## §25 Accessibility (FIXED items)

- Keyboard: Downloads inputs/buttons are native elements (Tab-reachable);
  Player has full keyboard map (Space/←→/↑↓/M/F) with input-conflict
  guard; Timeline is an ARIA slider with keyboard stepping.
- Labels: every icon-only control has aria-label (play/pause, volume,
  mute, rate select, fullscreen, priority select, sliders, search).
- Color-not-only: Failed renders ⚠ + text, Completed renders ✓ + text
  (§25 "颜色不是唯一表达").
- Reduced motion: `prefers-reduced-motion` collapses view transitions
  (§22).
- Contrast: dim text (#999-class tokens on the dark bg) measures ≥ 4.5:1
  for body-size text ( spot-checked); status tints pair with dark text
  glyphs.

## §28 Toast/notification unification (DOCUMENTED)

Tide's inline banners (`role="status"` notices) replace themselves on
each action — they cannot accumulate (§28 "不无限积累"). Permanent
state stays in the task/library cards, never only in a banner (§28).
The M8 OS notifications are user-toggled and cover exactly the §15
types. No further unification needed.

## §47 Long-run soak (reproducible harness)

`src-tauri/tests/m9_soak.rs` — run:
```text
TIDE_SOAK_SECONDS=3600 cargo test --locked --test m9_soak -- --ignored --nocapture
```
Churns the leak-prone surfaces (start/pause/resume/delete through the
real M1 engine + 50-demand submit/cancel per iteration through the M4
registry), asserts the registry stays bounded, reports RSS/downloads
per interval. Verified: 60 s run = 502 churn iterations, pending
stayed 0. Overnight runs use the same harness (§47's 4h/8h/overnight
rows need a running desktop session — recorded as the manual pass).
