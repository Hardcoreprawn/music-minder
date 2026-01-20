<# 
.SYNOPSIS
    Profile Music Minder library scanning to identify bottlenecks.

.DESCRIPTION
    This script runs the scan command with timing instrumentation to identify
    where time is spent during library scanning:
    - File discovery (walkdir traversal)
    - Metadata parsing (lofty/tag reading)  
    - Database writes (SQLite inserts)

.PARAMETER Path
    Path to scan. Defaults to test_music folder.

.PARAMETER Verbose
    Show detailed per-file timing.

.EXAMPLE
    .\scripts\profile-scan.ps1
    # Profile scan of test_music folder

.EXAMPLE
    .\scripts\profile-scan.ps1 -Path "D:\Music" -Verbose
    # Profile scan of large music library with detail
#>

param(
    [string]$Path = "test_music",
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

function Write-Step { param($msg) Write-Host ">>> $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "✓ $msg" -ForegroundColor Green }

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
Write-Host "  Music Minder Scan Profiler" -ForegroundColor Blue
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
Write-Host ""

# Count files first
Write-Step "Counting audio files..."
$audioExtensions = @("*.mp3", "*.flac", "*.ogg", "*.wav", "*.m4a")
$fileCount = 0
foreach ($ext in $audioExtensions) {
    $fileCount += (Get-ChildItem -Path $Path -Filter $ext -Recurse -ErrorAction SilentlyContinue).Count
}
Write-Host "  Found $fileCount audio files"
Write-Host ""

# Build with profiling symbols
Write-Step "Building with debug symbols..."
cargo build -p music-minder --release 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}
Write-Success "Build completed"

# Set up logging for timing
$env:RUST_LOG = "music_minder=debug,soundstore=debug,discographer=debug,musicographer=debug"

Write-Host ""
Write-Step "Running profiled scan..."
Write-Host "  Path: $Path"
Write-Host ""

# Time the scan
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

# Use a temp database to avoid polluting the real one
$tempDb = [System.IO.Path]::GetTempFileName() -replace '\.tmp$', '.db'
$env:MUSIC_MINDER_DB = $tempDb

try {
    # Run the scan and capture output
    $output = & cargo run -p music-minder --release -- scan $Path 2>&1
    $stopwatch.Stop()
    
    # Display output
    $output | ForEach-Object { Write-Host $_ }
    
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
    Write-Host "  Scan Results" -ForegroundColor Blue  
    Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
    Write-Host ""
    
    $totalMs = $stopwatch.ElapsedMilliseconds
    $totalSec = $totalMs / 1000.0
    $filesPerSec = if ($fileCount -gt 0) { $fileCount / $totalSec } else { 0 }
    
    Write-Host "  Total time:       $([math]::Round($totalSec, 2)) seconds"
    Write-Host "  Files scanned:    $fileCount"
    Write-Host "  Throughput:       $([math]::Round($filesPerSec, 1)) files/second"
    Write-Host ""
    
    # Performance assessment
    if ($filesPerSec -ge 1000) {
        Write-Host "  Status: ✅ EXCELLENT (≥1000 files/sec target met)" -ForegroundColor Green
    } elseif ($filesPerSec -ge 500) {
        Write-Host "  Status: ⚠️ GOOD (500-999 files/sec)" -ForegroundColor Yellow
    } elseif ($filesPerSec -ge 200) {
        Write-Host "  Status: ⚠️ ACCEPTABLE (200-499 files/sec)" -ForegroundColor Yellow
    } else {
        Write-Host "  Status: ❌ SLOW (<200 files/sec)" -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "  Current baseline: 200-500 files/second"
    Write-Host "  Target:           1000+ files/second"
    Write-Host ""
    
} finally {
    # Clean up temp database
    if (Test-Path $tempDb) {
        Remove-Item $tempDb -Force
    }
    $env:MUSIC_MINDER_DB = $null
}
