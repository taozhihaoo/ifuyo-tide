#!/usr/bin/env pwsh
# M10 §28/§37: Release artifact manifest generator.
#
# Usage:  pwsh scripts/release-manifest.ps1
# Output: target/release/MANIFEST.txt (exe + installer SHA-256, version,
#         git commit, toolchain, lockfile hashes, UTC timestamp).
#
# No usernames, absolute paths or tokens are written (M10 §28).

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..\src-tauri")

$exe = "target\release\ifuyo-tide.exe"
$installer = Get-ChildItem "target\release\bundle\nsis\*-setup.exe" |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not (Test-Path $exe)) { Write-Error "release exe not found: $exe"; exit 1 }
if (-not $installer) { Write-Error "NSIS installer not found"; exit 1 }

function Sha256([string]$path) {
    (Get-FileHash -Algorithm SHA256 $path).Hash.ToLower()
}

$commit = (git rev-parse HEAD).Trim()
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
$dirty = if (git status --porcelain) { "dirty" } else { "clean" }
$rustc = (rustc --version).Trim()
$cargo = (cargo --version).Trim()
$node = (node --version).Trim()
$pnpm = (pnpm --version).Trim()
$utc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$conf = Get-Content tauri.conf.json -Raw | ConvertFrom-Json
$version = $conf.version
$identifier = $conf.identifier

$exeHash = Sha256 $exe
$setupHash = Sha256 $installer.FullName
$cargoLock = Sha256 "Cargo.lock"
$pkgLock = $pkgLock = Sha256 "..\pnpm-lock.yaml"

$manifest = @"
Tide Release Manifest
=====================
generated_utc : $utc
version       : $version
identifier    : $identifier
git_commit    : $commit
git_branch    : $branch
git_status    : $dirty
toolchain     : $rustc / $cargo
node          : $node / pnpm $pnpm

artifacts:
  exe       : $(Split-Path $exe -Leaf)
    sha256  : $exeHash
  installer : $($installer.Name)
    sha256  : $setupHash

lockfiles:
  Cargo.lock     sha256 : $cargoLock
  pnpm-lock.yaml sha256 : $pkgLock

signing:
  status    : UNSIGNED (development build — see docs/M10-release-hardening.md
              for the documented signing path; keys are NEVER in this repo)
"@ | Set-Content -Encoding utf8 target\release\MANIFEST.txt

Get-Content target\release\MANIFEST.txt
