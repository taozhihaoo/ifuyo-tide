# M9 Design System

The consolidated visual language (§112 visual consistency checklist).
Tokens live in `src/styles/global.css` `:root`; components reference
them — raw hex outside the token block and the player surface is
audit-clean.

## Tokens

| Token | Value | Use |
|---|---|---|
| `--color-bg` | dark base | app background |
| `--color-surface` | raised surface | cards, rows |
| `--color-text-primary` | near-white | primary copy |
| `--color-text-secondary` / `--text-dim` | mid gray | secondary copy, hints |
| `--color-border` / `--border` | subtle gray | separators, inputs |
| `--color-accent` / `--accent` | Tide blue | played range, primary actions, focus |
| `--color-danger` / `--accent-danger` | red | destructive actions, failed states |
| `--space-1..4` | 4/8/12/16 px | vertical rhythm (§23) |
| `--radius-sm/md/lg` | 4/8/12 px | inputs, cards, dialogs |
| `--transition-fast/normal` | 120/200 ms | hover/fade timing (§21) |

## Buttons (§82 hierarchy)

`btn-primary` (single main action per context) · `btn` (secondary) ·
`btn-small` (row-level actions) · `btn-danger` (destructive: Delete
file, Cancel, Quit). Icon-only buttons carry aria-labels.

## Status visuals (§9/§25)

`.task-state` / `.library-state` pills: text + tint, plus glyphs for
Failed (⚠) and Completed (✓) — color is never the only carrier (§25).
One term per state via `STATE_LABELS` (§56).

## Motion (§21/§22)

View fade (`--transition-normal`); controls-host opacity fade for
tray-hide; `prefers-reduced-motion` collapses all durations. Motion
never carries information alone.

## Formatting (§60)

All user-visible numbers via `lib/format.ts`: bytes (1 decimal, GB 2),
speed (`—` zero-state), ETA (`—`/`Paused`/`<1s`…), truncateMiddle for
long names. No component formats numbers inline.

## Copy (§55)

Product language: short, neutral, actionable, non-blaming. Centralized
in `lib/strings.ts` (§57 audit) — locale-swap ready.
