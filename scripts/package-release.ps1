# Tide RC release packaging (M11 spec §47/§63/§79/§80).
#
# Produces a self-contained release\ directory from a completed
# `pnpm tauri build`:
#   release\<setup-exe>            the NSIS installer (primary artifact)
#   release\SHA256SUMS.txt         SHA-256 for every shipped file (§47)
#   release\RELEASE-MANIFEST.json  traceable build identity (§79/§80)
#   release\DEPENDENCIES.txt       dependency inventory / SBOM-lite (§63)
#   release\RELEASE-NOTES.md       copied from repo root when present
#
# Also runs the §9 artifact-hygiene checks (no private keys / .env /
# personal artifacts tracked or shipped).

param(
    [string]$Version = "0.2.0-rc.1",
    [string]$OutDir = "release",
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$bundleDir = Join-Path $repo "src-tauri\target\release\bundle\nsis"
$setupName = "ifuyo-tide_${Version}_x64-setup.exe"
$setupSrc = Join-Path $bundleDir $setupName

if (-not (Test-Path $setupSrc)) { throw "installer not found: $setupSrc (run pnpm tauri build first)" }

$out = Join-Path $repo $OutDir
New-Item -ItemType Directory -Path $out -Force | Out-Null

# --- artifacts ----------------------------------------------------------------
Copy-Item $setupSrc (Join-Path $out $setupName) -Force
$size = (Get-Item (Join-Path $out $setupName)).Length
$sha = (Get-FileHash (Join-Path $out $setupName) -Algorithm SHA256).Hash.ToLower()
Set-Content -Path (Join-Path $out "SHA256SUMS.txt") -Value "$sha  $setupName" -Encoding ASCII
Write-Output ("ARTIFACT {0} size={1} sha256={2}" -f $setupName, $size, $sha)

# --- toolchain / identity (§5/§80: single traceable answer) -------------------
function Ver([string]$cmd, [string[]]$cmdArgs) {
    # note: $args is an automatic variable in PS - must not be a param name
    try { (& $cmd @cmdArgs 2>$null | Select-Object -First 1) } catch { "unknown" }
}
$rustcV = Ver "rustc" @("--version")
$cargoV = Ver $CargoPath @("--version")
$nodeV = Ver "node" @("--version")
$pnpmV = Ver "pnpm" @("--version")
# pnpm prints a banner line before the CLI output; filter for the tool line.
# pnpm emits its banner on stderr; merge streams via cmd (PS5 NativeCommandError).
$tauriRaw = cmd /c "pnpm tauri --version 2>&1"
$tauriV = (($tauriRaw | Select-String "tauri-cli") | Out-String).Trim() -replace ".*tauri-cli\s*", ""
$commit = git -C $repo rev-parse HEAD
$branch = git -C $repo rev-parse --abbrev-ref HEAD
$dirty = git -C $repo status --porcelain | Measure-Object | ForEach-Object { $_.Count }

# --- RELEASE-MANIFEST.json (§79) ----------------------------------------------
$manifest = [ordered]@{
    product             = "ifuyo Tide"
    version             = $Version
    channel             = "rc"
    commit              = $commit
    branch              = $branch
    working_tree_dirty  = [bool]$dirty
    platform            = "windows"
    arch                = "x64"
    build_profile       = "release"
    dependencies_locked = $true
    signing             = @{ status = "pending"; reason = "no code-signing certificate; see docs/M11-release.md (§46 Production Signing = Pending)" }
    toolchain           = @{ rustc = $rustcV; cargo = $cargoV; node = $nodeV; pnpm = $pnpmV; tauri_cli = $tauriV; target = "x86_64-pc-windows-msvc" }
    locks               = @{ cargo = "src-tauri/Cargo.lock"; pnpm = "pnpm-lock.yaml" }
    artifacts           = @(
        [ordered]@{
            filename  = $setupName
            kind      = "nsis-installer"
            size      = $size
            sha256    = $sha
            signature = "unsigned"
        }
    )
}
# UTF8 without BOM - strict JSON parsers reject a BOM.
[System.IO.File]::WriteAllText((Join-Path $out "RELEASE-MANIFEST.json"), ($manifest | ConvertTo-Json -Depth 5), (New-Object System.Text.UTF8Encoding($false)))
Write-Output "MANIFEST RELEASE-MANIFEST.json written (commit $commit)"

# --- dependency inventory (§63 SBOM-lite) -------------------------------------
$deps = @()
$deps += "# Rust transitive dependencies (cargo tree, normal edges, unique)"
$deps += (& $CargoPath tree --locked --edges normal --prefix none --manifest-path (Join-Path $repo "src-tauri\Cargo.toml") 2>$null |
    ForEach-Object { $_.Trim() } | Where-Object { $_ } | Sort-Object -Unique)
$deps += ""
$deps += "# Frontend direct dependencies (package.json; transitive resolution locked by pnpm-lock.yaml)"
$pkg = Get-Content (Join-Path $repo "package.json") -Raw | ConvertFrom-Json
foreach ($section in @("dependencies", "devDependencies")) {
    $deps += "## $section"
    $pkg.$section.PSObject.Properties | ForEach-Object { $deps += ("{0} {1}" -f $_.Name, $_.Value) }
}
Set-Content -Path (Join-Path $out "DEPENDENCIES.txt") -Value ($deps -join [Environment]::NewLine) -Encoding UTF8
Write-Output "SBOM DEPENDENCIES.txt written ($(($deps | Where-Object { $_ -and $_ -notmatch '^#' }).Count) entries)"

# --- release notes --------------------------------------------------------------
$notes = Join-Path $repo "RELEASE-NOTES.md"
if (Test-Path $notes) {
    Copy-Item $notes (Join-Path $out "RELEASE-NOTES.md") -Force
    Write-Output "NOTES RELEASE-NOTES.md copied"
}

# --- §9 artifact hygiene ---------------------------------------------------------
$hygieneHits = @()
foreach ($pattern in @("*.pfx", "*.key", "*.pem", "*.env", "*.pdb", "*.srcmap*", "*source-map*")) {
    # no-match patterns must contribute nothing (empty filter results)
    $found = Get-ChildItem $out -Recurse -File -Filter $pattern -ErrorAction SilentlyContinue
    if ($found) { $hygieneHits += $found.FullName }
}
$trackedKeys = git -C $repo ls-files | Select-String -Pattern '\.(pfx|pem)$|(^|/)\.env$|private.*key'
if ($trackedKeys) { $hygieneHits += "TRACKED-IN-GIT: $trackedKeys" }
if ($hygieneHits) {
    Write-Output "HYGIENE FAIL:"
    $hygieneHits | ForEach-Object { Write-Output "  $_" }
    exit 1
}
Write-Output "HYGIENE PASS (no private keys/.env/debug artifacts in release dir or git index)"
Write-Output "PACKAGING COMPLETE: $out"
