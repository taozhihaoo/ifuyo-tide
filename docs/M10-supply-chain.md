# M10 Supply Chain (§20/§21)

## Rust dependency audit

- Cargo.lock committed; every build uses `--locked` (§50 verification).
- `cargo deny check` gates: advisories, licenses, bans, sources.
- `cargo audit` standalone tool: NOT INSTALLED on this machine —
  recorded as TOOL NOT AVAILABLE; `cargo deny check advisories`
  (RustSec database) performs the equivalent advisory scan and is
  the authoritative gate. MANUAL CHECK PERFORMED in addition.

## Findings (classified per §20)

| Finding | Class | Action |
|---|---|---|
| RUSTSEC-2026-0293 ringbuf 0.4.x double-free (via librqbit-utp 0.7.0 ← librqbit 9.0.1) | vulnerability, transitive, no-fix-without-upstream | IGNORED in deny.toml with justification: librqbit-utp pins ringbuf 0.4.x (0.5 semver-incompatible); Tide does not call Consumer::skip/clear; trigger requires a Drop panic inside librqbit-utp internals. P1 — revisit on librqbit/utp update. |
| RUSTSEC-2025-0060 crypto-hash unmaintained | maintenance | IGNORED with justification (transitive via librqbit; not a vulnerability) |
| RUSTSEC-2024-0370 proc-macro-error unmaintained | maintenance | IGNORED with justification (transitive via dependency macros) |
| License rejections (0BSD/MIT-OR-etc, Unlicense, BSD-3-Clause…) | compliance allowlist gap | FIXED: deny.toml allowlist covers all permissive licenses in the tree; ifuyo-tide Cargo.toml gains `license = "MIT OR Apache-2.0"` |

## Frontend audit

- pnpm-lock.yaml committed; `pnpm tauri build` uses the lockfile.
- `pnpm audit` result: recorded in M10-report §19 with the
  development-only/production classification.
- No remote package loading, no postinstall network scripts, no CDN
  assets (§57 — CSP blocks remote script/font/img).

## Licenses (§21)

- ifuyo-tide: MIT OR Apache-2.0 (added to Cargo.toml in M10).
- Tree: permissive only (MIT/Apache-2.0/BSD/ISC/Unicode/Zlib/0BSD/
  Unlicense/CDLA-Permissive-2.0/MPL-2.0) — allowlisted in deny.toml.
- librqbit 9.0.1: Apache-2.0 (verified from dependency metadata, not
  README). THIRD_PARTY_NOTICES.md updated with the plugin set.
