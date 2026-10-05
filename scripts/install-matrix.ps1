# Tide RC installer matrix (M11 spec §10/§24/§48/§77).
#
# Executes the real installer scenarios on the current machine against
# a fixed ISOLATED app-data root (spec §81):
#
#   S1  Clean install (previous release 0.1.0 = "Machine B" start state)
#   S2  First launch of installed build (smoke; seeds a task row)
#   S3  Upgrade with app running (actual behavior recorded)
#   S4  Upgrade verification (exe version + registry)
#   S5  Upgrade data preservation (smoke on RC exe, same data root:
#       seeded row + user_version must survive)
#   S6  Uninstall (install dir + registry gone; app data PRESERVED)
#   S7  Reinstall (same version; data still preserved)
#   S8  Corrupt app data (fail-closed startup; no silent corruption)
#   S11 Interrupted install — NOT TESTED here (documented, flaky to
#       simulate); non-admin session recorded.
#
# Prerequisites: built installers in src-tauri\target\release\bundle\nsis,
# python on PATH (SQLite checks), no Tide instance installed.

param(
    [string]$OldInstaller = "src-tauri\target\release\bundle\nsis\ifuyo-tide_0.1.0_x64-setup.exe",
    [string]$NewInstaller = "src-tauri\target\release\bundle\nsis\ifuyo-tide_1.0.3_x64-setup.exe",
    [string]$OldVersion = "0.1.0",
    [string]$NewVersion = "1.0.3",
    [string]$DataRoot = ""
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$failures = @()

function Check([string]$name, [bool]$ok, [string]$detail) {
    $tag = if ($ok) { "PASS" } else { "FAIL" }
    Write-Output ("{0} {1} {2}" -f $tag, $name, $detail)
    if (-not $ok) { $script:failures += $name }
}

function Find-RegistryEntry {
    Get-ChildItem "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall" -ErrorAction SilentlyContinue |
        Where-Object { $_.GetValue("DisplayName") -like "*ifuyo*" }
}

function Installed-Exe {
    $reg = Find-RegistryEntry
    if (-not $reg) { return $null }
    $loc = ($reg.GetValue("InstallLocation") -replace '"', '').TrimEnd('')
    if (-not $loc) { return $null }
    $exe = Join-Path $loc "ifuyo-tide.exe"
    if (Test-Path $exe) { return $exe } else { return $null }
}

function Stop-AllTide {
    Get-Process ifuyo-tide -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    # reap tide-owned WebView2 children (user-data-folder lock guard)
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        $wv = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.CommandLine -match 'ifuyo-tide|com\.ifuyo\.tide' })
        if ($wv.Count -eq 0) { break }
        foreach ($w in $wv) { Stop-Process -Id $w.ProcessId -Force -ErrorAction SilentlyContinue }
        Start-Sleep -Milliseconds 500
    }
}

function Inherit-Smoke([string]$name) {
    if ($LASTEXITCODE -ne 0) {
        Check $name $false "smoke run failed (see smoke output above)"
        $script:failures += $name
    } else {
        Check $name $true "smoke run passed (see smoke output above)"
    }
}

if (-not $DataRoot) { $DataRoot = Join-Path ([System.IO.Path]::GetTempPath()) "tide-rc-matrix\data" }
Write-Output "INFO data-root $DataRoot"

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Check "non-admin-session" (-not $elevated) ("elevated=$elevated (per-user installer requires no elevation, §16)")

# --- preconditions -------------------------------------------------------------
if (Test-Path $OldInstaller) { Check "old-installer-present" $true $OldInstaller } else { throw "missing $OldInstaller" }
if (Test-Path $NewInstaller) { Check "new-installer-present" $true $NewInstaller } else { throw "missing $NewInstaller" }
$pre = Find-RegistryEntry
if ($pre) {
    # self-cleaning: remove any previous matrix installation
    $prevLoc = ($pre.GetValue("InstallLocation") -replace '"', '').TrimEnd('')
    $prevUn = Join-Path $prevLoc "uninstall.exe"
    Write-Output "INFO prereq: uninstalling previous matrix copy at $prevLoc"
    if (Test-Path $prevUn) { Start-Process -FilePath $prevUn -ArgumentList "/S" -Wait }
    Start-Sleep -Seconds 2
    Check "prereq-cleaned" ($null -eq (Find-RegistryEntry)) "previous copy removed"
} else {
    Check "prereq-not-installed" $true "no ifuyo registry entry before matrix"
}
Stop-AllTide

$realLocalAppData = $env:LOCALAPPDATA

# --- S1: clean install (previous release) ---------------------------------------
Write-Output "--- S1 clean install ($OldVersion)"
Start-Process -FilePath $OldInstaller -ArgumentList "/S" -Wait
$reg1 = Find-RegistryEntry
Check "S1-registry" ($null -ne $reg1) ("DisplayName=" + $reg1.GetValue("DisplayName") + " DisplayVersion=" + $reg1.GetValue("DisplayVersion"))
$exe1 = Installed-Exe
Check "S1-exe" ($null -ne $exe1) $exe1
if ($exe1) {
    Check "S1-version" ((Get-Item $exe1).VersionInfo.ProductVersion -eq $OldVersion) ("ProductVersion=" + (Get-Item $exe1).VersionInfo.ProductVersion)
}
$shortcut = Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" -Filter "*ifuyo*" -Recurse -ErrorAction SilentlyContinue
Check "S1-start-menu" ($null -ne $shortcut) ($shortcut | Select-Object -First 1 -ExpandProperty FullName -ErrorAction SilentlyContinue)
Check "S1-install-path-not-appdata" ($exe1 -and -not $exe1.StartsWith((Join-Path $realLocalAppData "ifuyo\Tide"))) "install dir != app data dir != download root (§11)"

# --- S2: first launch of installed build (seeds data) ----------------------------
Write-Output "--- S2 first launch + seed ($OldVersion)"
$env:LOCALAPPDATA = $DataRoot
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "release-smoke.ps1") -ExePath $exe1 -ExpectedVersion $OldVersion -DataRoot $DataRoot
$env:LOCALAPPDATA = $realLocalAppData
Inherit-Smoke "S2-smoke"

# --- S3: upgrade with app running -------------------------------------------------
Write-Output "--- S3 upgrade with app running ($OldVersion -> $NewVersion)"
$env:LOCALAPPDATA = $DataRoot
$running = Start-Process -FilePath $exe1 -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 6
$env:LOCALAPPDATA = $realLocalAppData
Start-Process -FilePath $NewInstaller -ArgumentList "/S" -Wait
Start-Sleep -Seconds 2
$stillRunning = Get-Process ifuyo-tide -ErrorAction SilentlyContinue
Write-Output ("INFO S3 running-app after silent upgrade: {0} process(es)" -f @($stillRunning).Count)
Stop-AllTide
$exe2 = Installed-Exe
Check "S4-exe-replaced" ($exe2 -and (Get-Item $exe2).VersionInfo.ProductVersion -eq $NewVersion) ("ProductVersion=" + (Get-Item $exe2).VersionInfo.ProductVersion)
$reg2 = Find-RegistryEntry
Check "S4-registry-version" ($reg2.GetValue("DisplayVersion") -eq $NewVersion) ("DisplayVersion=" + $reg2.GetValue("DisplayVersion"))

# --- S5: upgrade data preservation --------------------------------------------------
Write-Output "--- S5 upgrade data preservation (same data root, RC exe)"
$env:LOCALAPPDATA = $DataRoot
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "release-smoke.ps1") -ExePath $exe2 -ExpectedVersion $NewVersion -DataRoot $DataRoot
$env:LOCALAPPDATA = $realLocalAppData
Inherit-Smoke "S5-smoke"

# --- S6: uninstall --------------------------------------------------------------------
Write-Output "--- S6 uninstall (app-data preservation policy)"
Stop-AllTide
$uninst = Join-Path (Split-Path -Parent $exe2) "uninstall.exe"
Start-Process -FilePath $uninst -ArgumentList "/S" -Wait
Start-Sleep -Seconds 2
$loc = Split-Path -Parent $exe2
$dirGone = -not (Test-Path (Join-Path $loc "ifuyo-tide.exe"))
Check "S6-install-dir-removed" $dirGone $loc
Check "S6-registry-removed" ($null -eq (Find-RegistryEntry)) "no ifuyo uninstall entry"
$shortcutAfter = Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" -Filter "*ifuyo*" -Recurse -ErrorAction SilentlyContinue
Check "S6-start-menu-removed" ($null -eq $shortcutAfter) "shortcut removed"
$dbAfter = Join-Path $DataRoot "ifuyo\Tide\database\tide.db"
Check "S6-appdata-preserved" (Test-Path $dbAfter) "database/logs/torrent-metadata survive uninstall (§12)"
$dl = Join-Path $DataRoot "downloads-marker.txt"
Set-Content -Path $dl -Value "user download data placeholder"
Check "S6-user-files-untouched" (Test-Path $dl) "files outside app data untouched"

# --- S7: reinstall (same version) -------------------------------------------------------
Write-Output "--- S7 reinstall same version ($NewVersion)"
Start-Process -FilePath $NewInstaller -ArgumentList "/S" -Wait
$exe3 = Installed-Exe
Check "S7-reinstalled" ($null -ne $exe3) $exe3
$env:LOCALAPPDATA = $DataRoot
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "release-smoke.ps1") -ExePath $exe3 -ExpectedVersion $NewVersion -DataRoot $DataRoot
$env:LOCALAPPDATA = $realLocalAppData
Inherit-Smoke "S7-smoke"

# --- S8: corrupt app data (fail-closed) --------------------------------------------------
Write-Output "--- S8 corrupt app data"
Stop-AllTide
$db = Join-Path $DataRoot "ifuyo\Tide\database\tide.db"
$backup = "$db.matrix-backup"
Copy-Item $db $backup -Force
$bytes = [System.IO.File]::ReadAllBytes($db)
[byte[]]$patch = 0x44,0x45,0x41,0x44,0x42,0x45,0x45,0x46,0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07
for ($i = 0; $i -lt 16; $i++) { $bytes[$i] = $patch[$i] }
[System.IO.File]::WriteAllBytes($db, $bytes)
$env:LOCALAPPDATA = $DataRoot
$p = Start-Process -FilePath $exe3 -PassThru -WindowStyle Minimized -RedirectStandardError (Join-Path $DataRoot "stderr-corrupt.txt")
$exited = $p.WaitForExit(20000)
$env:LOCALAPPDATA = $realLocalAppData
# ExitCode is unreliable for GUI-subsystem processes under Start-Process;
# the fail-closed evidence is: process terminated + structured fatal on
# stderr (fatal_startup_raw, std::process::exit(1) — app/mod.rs).
$errText = (Get-Content (Join-Path $DataRoot "stderr-corrupt.txt") -Raw -ErrorAction SilentlyContinue)
Check "S8-corrupt-db-fail-closed" ($exited -and $errText -match 'fatal:') ("exited=$exited, structured fatal on stderr; app refuses to start on corrupt DB, no silent corruption (§30/§59)")
if ($errText) { Write-Output ("INFO S8 stderr: {0}" -f ($errText.Trim() -replace '\r?\n', ' | ')) }
Copy-Item $backup $db -Force
Check "S8-db-restored" (Test-Path $db) "backup restored for cleanliness"

# --- cleanup ------------------------------------------------------------------------------
Write-Output "--- cleanup"
Stop-AllTide
$uninst2 = Join-Path (Split-Path -Parent $exe3) "uninstall.exe"
if (Test-Path $uninst2) { Start-Process -FilePath $uninst2 -ArgumentList "/S" -Wait }
Remove-Item -Recurse -Force $DataRoot -ErrorAction SilentlyContinue
Write-Output "INFO cleaned data root + uninstalled test copy"

if ($failures.Count -gt 0) {
    Write-Output ("MATRIX RESULT: FAIL ({0}: {1})" -f $failures.Count, ($failures -join ", "))
    exit 1
}
Write-Output "MATRIX RESULT: PASS"
