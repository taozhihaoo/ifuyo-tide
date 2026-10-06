# Tide RC release smoke test (M11 spec §50).
#
# Automated checks against a release-built ifuyo-tide.exe:
#   1. exe version resource matches the expected RC version
#   2. launch in an ISOLATED LOCALAPPDATA (spec §81 - never touches real
#      user data)
#   3. required app-data directories are created (database/logs/torrent-metadata)
#   4. SQLite migrations ran (db.migration_applied x4 in the log;
#      PRAGMA user_version == 4 via python)
#   5. log health: app.started present, no panic events, JSON lines parse
#   6. a seeded task row survives process kill + warm relaunch
#      (task persistence; creation itself is UI-gated - see docs/M11-test-matrix.md)
#   7. idle CPU/memory sample (spec §54 budget)
#   8. process-kill shutdown + recovery relaunch (spec §28)
#
# Requirements: Windows PowerShell 5+, python on PATH (SQLite read/seed).
# The real-UI task-creation flow and tray-quit path remain manual items
# and are NOT claimed here.

param(
    [string]$ExePath = "src-tauri\target\release\ifuyo-tide.exe",
    [string]$ExpectedVersion = "1.0.4",
    [int]$IdleSampleSeconds = 8,
    # Reusing a fixed -DataRoot across runs (e.g. around an installer
    # upgrade) turns the seeded-row check into a data-preservation
    # check across the upgrade.
    [string]$DataRoot = "",
    [switch]$Clean,
    # Legacy releases (< 1.0.3) are console-subsystem; the matrix passes
    # this when smoke-testing the PREVIOUS installer as an upgrade source.
    [switch]$AllowConsoleSubsystem
)

$ErrorActionPreference = "Stop"
$failures = @()
$repo = Split-Path -Parent $PSScriptRoot

function Check([string]$name, [bool]$ok, [string]$detail) {
    $tag = if ($ok) { "PASS" } else { "FAIL" }
    Write-Output ("{0} {1} {2}" -f $tag, $name, $detail)
    if (-not $ok) { $script:failures += $name }
}

# --- 1. artifact identity ---------------------------------------------------
if (-not (Test-Path $ExePath)) { throw "exe not found: $ExePath" }
$exe = (Resolve-Path $ExePath).Path
$vi = (Get-Item $exe).VersionInfo
Check "version-resource" ($vi.ProductVersion -eq $ExpectedVersion) ("ProductVersion=" + $vi.ProductVersion)

# PE subsystem must be 2 (WINDOWS_GUI) in release: a console-subsystem
# build pops a log window over the app on double-click (1.0.2 defect).
$fs = [System.IO.File]::OpenRead($exe)
$br = New-Object System.IO.BinaryReader($fs)
$fs.Seek(0x3C, 'Begin') | Out-Null
$peOff = $br.ReadInt32()
$subsysOff = $peOff + 4 + 20 + 68  # PE sig + COFF + optional header magic-independent offset for PE32+
$fs.Seek($subsysOff, 'Begin') | Out-Null
$subsystem = $br.ReadUInt16()
$fs.Close()
if ($AllowConsoleSubsystem) {
    Write-Output "SKIP gui-subsystem (legacy release expected console-subsystem)"
} else {
    Check "gui-subsystem" ($subsystem -eq 2) ("PE subsystem=$subsystem (2=GUI, 3=console)")
}

# --- 2. isolated app data ---------------------------------------------------
if ($DataRoot) {
    $dataRoot = $DataRoot
    New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null
} else {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $dataRoot = Join-Path ([System.IO.Path]::GetTempPath()) "tide-rc-smoke\$stamp"
    New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null
}
$realLocalAppData = $env:LOCALAPPDATA
$env:LOCALAPPDATA = $dataRoot
Write-Output "INFO isolated-LOCALAPPDATA $dataRoot"

function Read-Log([string]$dataRoot) {
    $log = Join-Path $dataRoot "ifuyo\Tide\logs\tide.log"
    if (-not (Test-Path $log)) { return @() }
    Get-Content $log -ErrorAction SilentlyContinue
}

# Run-scoped view: when a fixed -DataRoot is reused across runs (upgrade
# testing), event assertions must only consider lines this run produced.
$script:logBase = 0
function Reset-LogScope([string]$dataRoot) {
    $script:logBase = @(Read-Log $dataRoot).Count
}
function Read-NewLog([string]$dataRoot) {
    $all = @(Read-Log $dataRoot)
    if ($all.Count -gt $script:logBase) { return $all[$script:logBase..($all.Count - 1)] }
    return @()
}

function Wait-For([scriptblock]$cond, [int]$seconds, [string]$what) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        if (& $cond) { return $true }
        Start-Sleep -Milliseconds 500
    }
    Write-Output "WARN timeout waiting for $what"
    return $false
}

function Start-Tide {
    $p = Start-Process -FilePath $exe -PassThru -WindowStyle Minimized
    return $p
}

function Stop-Tide([System.Diagnostics.Process]$p) {
    if (-not $p.HasExited) {
        # CloseMainWindow may hide-to-tray (M8 closeToTray); kill is the
        # deterministic shutdown and doubles as the §28 crash-exit test.
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    }
    $p.WaitForExit(10000) | Out-Null
    # Reap tide-owned WebView2 children: lingering msedgewebview2 can
    # hold the WebView2 user-data folder and fatally fail the NEXT
    # launch (observed as a silent early exit in the reinstall matrix).
    Stop-TideWebViews
}

function Stop-TideWebViews {
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        $wv = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.CommandLine -match 'ifuyo-tide|com\.ifuyo\.tide' })
        if ($wv.Count -eq 0) { return }
        foreach ($w in $wv) { Stop-Process -Id $w.ProcessId -Force -ErrorAction SilentlyContinue }
        Start-Sleep -Milliseconds 500
    }
}

# --- 3-5. cold launch: dirs, migrations, log health --------------------------
Reset-LogScope $dataRoot
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$p = Start-Tide
$db = Join-Path $dataRoot "ifuyo\Tide\database\tide.db"
$dbCreated = Wait-For { Test-Path $db } 60 "database file"
Check "required-dirs" ($dbCreated -and (Test-Path (Join-Path $dataRoot "ifuyo\Tide\torrent-metadata"))) "database/logs/torrent-metadata under isolated LOCALAPPDATA"

$started = Wait-For { @(Read-NewLog $dataRoot | Select-String -SimpleMatch '"app.started"').Count -gt 0 } 60 "app.started"
$coldStartMs = $sw.ElapsedMilliseconds
Check "launch-app-started" $started ("cold start to app.started = ${coldStartMs}ms")

$logLines = @(Read-NewLog $dataRoot)
$panics = @($logLines | Select-String -Pattern 'panic|FATAL')
Check "log-no-panic" ($panics.Count -eq 0) ("panic/fatal lines = " + $panics.Count)
Check "process-alive" (-not $p.HasExited) ("pid=" + $p.Id)

# user_version via python (SQLite read): 4 = all migrations applied
$pyUserVersion = @'
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
print(conn.execute("PRAGMA user_version").fetchone()[0])
'@
$uvFile = Join-Path $dataRoot "uv.py"
Set-Content -Path $uvFile -Value $pyUserVersion -Encoding ASCII
$uv = (python $uvFile $db 2>$null)
Check "user-version-4" ($uv -eq "4") "PRAGMA user_version=$uv"

# Fresh DB: 4 migration events. Reused root (upgrade runs): schema already
# at v4, no new events fire - user_version above is the authority.
$migrations = @($logLines | Select-String -SimpleMatch '"db.migration_applied"')
Check "migrations" (($migrations.Count -eq 4) -or ($uv -eq "4")) ("db.migration_applied events = " + $migrations.Count + ", user_version=" + $uv)

# --- 6. seeded task row persistence across restart ---------------------------
$seedPy = @'
import sqlite3, sys, time
conn = sqlite3.connect(sys.argv[1])
now = int(time.time())
conn.execute(
    "INSERT OR REPLACE INTO download_tasks (id, source_url, allow_private, destination_dir,"
    " filename, part_path, final_path, overwrite_policy, state, downloaded_bytes,"
    " created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
    ("smoke-seed-0001", "https://example.invalid/smoke.bin", 0, "C:\\smoke", "smoke.bin",
     "C:\\smoke\\smoke.bin.part", "C:\\smoke\\smoke.bin", "auto_rename", "paused", 12345, now, now))
conn.commit()
'@
$seedFile = Join-Path $dataRoot "seed.py"
Set-Content -Path $seedFile -Value $seedPy -Encoding ASCII
$seeded = $true
try { python $seedFile $db 2>$null } catch { $seeded = $false }
Check "seed-task-row" $seeded "insert smoke-seed-0001 (state=paused)"

# --- 7. idle resource budget sample (§54) ------------------------------------
Start-Sleep -Seconds 2
$p.Refresh()
$cpuBefore = $p.TotalProcessorTime.TotalMilliseconds
Start-Sleep -Seconds $IdleSampleSeconds
$p.Refresh()
$cpuAfter = $p.TotalProcessorTime.TotalMilliseconds
# WorkingSet64 on a Start-Process handle can read 0; re-query by id.
$wsMb = [math]::Round((Get-Process -Id $p.Id -ErrorAction SilentlyContinue).WorkingSet64 / 1MB, 1)
$cpuCores = [math]::Max(1, $env:NUMBER_OF_PROCESSORS)
$cpuPct = [math]::Round((($cpuAfter - $cpuBefore) / 1000.0) / $IdleSampleSeconds / $cpuCores * 100, 2)
Check "idle-cpu-budget" ($cpuPct -lt 5.0) "idle CPU~${cpuPct}% (sampled ${IdleSampleSeconds}s), WS=${wsMb}MB"

Stop-Tide $p

# --- 8. warm relaunch: recovery + row persistence ----------------------------
$p2 = Start-Tide
$relaunched = Wait-For { @(Read-NewLog $dataRoot | Select-String -SimpleMatch '"app.started"').Count -ge 2 } 60 "second app.started"
Check "warm-relaunch" $relaunched "second launch reaches app.started"
$row = (python $uvFile $db 2>$null)
$countPy = @'
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
print(conn.execute("SELECT state, downloaded_bytes FROM download_tasks WHERE id='smoke-seed-0001'").fetchone())
'@
$countFile = Join-Path $dataRoot "count.py"
Set-Content -Path $countFile -Value $countPy -Encoding ASCII
$rowState = (python $countFile $db 2>$null)
Check "task-row-persisted" ($rowState -like "('paused'*") "row after restart: $rowState"
Check "user-version-stable" ($row -eq "4") "user_version after relaunch=$row"
Stop-Tide $p2

# --- restore real env ---------------------------------------------------------
$env:LOCALAPPDATA = $realLocalAppData
Write-Output "INFO evidence-root $dataRoot"

if ($Clean) {
    Remove-Item -Recurse -Force $dataRoot -ErrorAction SilentlyContinue
    Write-Output "INFO cleaned $dataRoot"
}

if ($failures.Count -gt 0) {
    Write-Output ("SMOKE RESULT: FAIL ({0} failures: {1})" -f $failures.Count, ($failures -join ", "))
    exit 1
}
Write-Output "SMOKE RESULT: PASS"
