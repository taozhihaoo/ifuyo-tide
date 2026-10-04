# M11 Release — Build, Installer, Updater, Signing

M11 evidence-type rules (FACT / TEST RESULT / STATIC ANALYSIS / TOOL
RESULT / MANUAL REVIEW / ASSUMPTION) apply throughout.

## 1. Release Candidate identity (§5)

| Field | Value | Evidence |
|---|---|---|
| Tide version | 0.2.0-rc.1 | TEST RESULT — exe `ProductVersion` file resource checked by `scripts/release-smoke.ps1` |
| Version sources | `package.json` = `tauri.conf.json` = `Cargo.toml[package]` = 0.2.0-rc.1 | FACT — single aligned value bumped in commit 7d8afb1; `Cargo.lock` regenerated |
| Tauri | 2.x (2.12.1 line) — exact value recorded in `release/RELEASE-MANIFEST.json` at packaging time | TOOL RESULT |
| Rust / Cargo / Node / pnpm | recorded in `release/RELEASE-MANIFEST.json` `toolchain{}` | TOOL RESULT |
| Target | windows / x64 (`x86_64-pc-windows-msvc`) / profile `release` | FACT |
| Commit / branch | recorded in `release/RELEASE-MANIFEST.json` (`commit`, `branch`, `working_tree_dirty`) | TOOL RESULT |
| Dependency lock state | `src-tauri/Cargo.lock` + `pnpm-lock.yaml` committed; builds use `--locked` | FACT |

## 2. Release build baseline (§7/§8)

Commands (all from repo root; cargo on PATH):

```text
cargo fmt --check            (src-tauri)
cargo test  --locked         (src-tauri)
cargo build --release --locked
pnpm typecheck               (tsc --noEmit)
pnpm test                    (vitest run)
pnpm tauri build             (frontend build + release build + NSIS bundle)
```

Result at RC commit: 317 Rust tests passed / 0 failed; 38 frontend
tests / 0 failed; `tsc` clean; `fmt --check` clean; NSIS bundle
produced. Reproducibility level achieved: version, commit, lockfiles
and build commands are pinned and recorded (§8 requires traceability,
not bit-for-bit identity — ASSUMPTION: toolchain cache state on the
build host does not alter behavior; not disproven by any evidence).

No test was deleted, no warning suppressed, no security check
disabled, no migration touched, no environment path hardcoded to
reach this state (§7 negative list — FACT, verified by M11 diff
audit, §67).

## 3. Installer (§6/§10/§11)

- **Primary installer**: NSIS (`ifuyo-tide_0.2.0-rc.1_x64-setup.exe`),
  per-user (no admin rights). MSI is **not** shipped: a per-user
  non-admin Windows downloader does not need machine-wide install,
  and maintaining a second installer format adds surface without
  user value (decision, consistent with M10-windows-release.md).
- The installer bootstraps WebView2 if the machine lacks it (Tauri
  default `downloadBootstrapper`).
- Install-path behavior, start-menu entry, uninstall registration,
  upgrade and uninstall flows: see `M11-install-upgrade.md` for the
  executed matrix with results.

## 4. Updater Status = Not Included in RC (§44 B)

- **Why**: no signed update origin exists (unsigned build, KI-3), no
  release channel infrastructure, and Tauri's updater requires
  signed artifacts; shipping an updater that cannot verify
  signatures would weaken the security posture.
- **Current upgrade path**: run the new installer over the old
  installation (per-user in-place upgrade; data preservation verified
  — see `M11-install-upgrade.md`).
- **How it will arrive**: a signed 1.0 channel with Tauri updater
  (public key in `tauri.conf.json`, signed artifacts, private key
  outside the repository). Manual validation then covers: update
  check, signature verification, download, install, restart, data
  preservation, interrupted-update behavior (§44 A list retained as
  the acceptance checklist).

## 5. Code Signing (§45/§46)

**Production Signing = Pending** — no certificate exists; no
pseudo-production signing was fabricated. The RC ships unsigned; the
signing path (six documented steps incl. SHA-256 digest and RFC 3161
timestamping) is in `M10-release-hardening.md` §Signing. Signing is a
**Release Gate before 1.0**. Private keys: none exist in the
repository (§9 hygiene scan passes; see `package-release.ps1`
HYGIENE gate).

## 6. Artifact integrity (§47/§79/§80)

`scripts/package-release.ps1` produces the `release/` directory:
setup exe, `SHA256SUMS.txt`, `RELEASE-MANIFEST.json` (product,
version, channel, commit, branch, dirty state, platform, arch,
profile, toolchain, lock files, signing status, per-artifact
size/SHA-256), `DEPENDENCIES.txt` (§63 SBOM-lite: full Rust
transitive tree + frontend direct deps) and `RELEASE-NOTES.md`.
Given an RC artifact, the manifest answers: which commit, which
toolchain, which locks, signed or not, which SHA-256 (§80).

## 7. CI / Release pipeline (§64/§65)

**NOT APPLICABLE** — no CI pipeline exists (recorded since M10 §43).
Nothing masks as a production pipeline. Secrets audit: repository
contains no signing keys, updater keys, tokens or credentials (§9
hygiene scan; git index checked by `package-release.ps1`).

## 8. Git release discipline (§66)

RC commit = the commit recorded in `RELEASE-MANIFEST.json`; tag
candidate `v0.2.0-rc.1` (annotated) created at M11 close.
`working_tree_dirty` field in the manifest records whether the
packaged tree was clean.
