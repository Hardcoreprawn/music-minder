<# 
.SYNOPSIS
    Profile Music Minder startup with flamegraph/samply.

.DESCRIPTION
    This script builds the application with profiling symbols and runs it
    under a profiler to generate flame graphs for startup analysis.

.PARAMETER Profiler
    The profiler to use: 'samply' (recommended) or 'dtrace' (experimental).
    Default: samply

.PARAMETER OutputDir
    Directory for profiling output. Default: target/profiles

.PARAMETER DurationSeconds
    How long to run the app before stopping (for startup profiling).
    Default: 5 seconds

.EXAMPLE
    .\scripts\profile-startup.ps1
    # Builds and profiles with samply

.EXAMPLE
    .\scripts\profile-startup.ps1 -DurationSeconds 10
    # Profile for 10 seconds to capture more of the startup
#>

param(
    [ValidateSet("samply", "dtrace")]
    [string]$Profiler = "samply",
    
    [string]$OutputDir = "target\profiles",
    
    [int]$DurationSeconds = 5
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Step { param($msg) Write-Host ">>> $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Warning { param($msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Error { param($msg) Write-Host "✗ $msg" -ForegroundColor Red }

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
Write-Host "  Music Minder Startup Profiler" -ForegroundColor Blue
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
Write-Host ""

# Check for required tools
Write-Step "Checking prerequisites..."

if ($Profiler -eq "samply") {
    $samplyPath = Get-Command samply -ErrorAction SilentlyContinue
    if (-not $samplyPath) {
        Write-Warning "samply not found. Installing..."
        cargo install samply
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install samply"
            exit 1
        }
    }
    Write-Success "samply is available"
}

# Ensure output directory exists
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    Write-Success "Created output directory: $OutputDir"
}

# Build with profiling profile
Write-Step "Building with profiling profile (release + debug symbols)..."
cargo build --profile profiling -p music-minder
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}
Write-Success "Build completed"

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$binaryPath = "target\profiling\music-minder.exe"

if (-not (Test-Path $binaryPath)) {
    Write-Error "Binary not found at $binaryPath"
    exit 1
}

Write-Host ""
Write-Step "Starting profiler..."
Write-Host "  Profiler: $Profiler"
Write-Host "  Binary: $binaryPath"
Write-Host "  Duration: $DurationSeconds seconds"
Write-Host ""

switch ($Profiler) {
    "samply" {
        # samply records and opens the Firefox Profiler UI automatically
        Write-Host "Running samply - the Firefox Profiler will open in your browser..." -ForegroundColor Yellow
        Write-Host "Press Ctrl+C or close the app after $DurationSeconds seconds to capture startup" -ForegroundColor Yellow
        Write-Host ""
        
        # Run with environment variable for timing output
        $env:RUST_LOG = "music_minder=debug"
        
        # samply record will capture and open the profile
        samply record --save-only --output "$OutputDir\startup-$timestamp.json" -- $binaryPath
        
        if (Test-Path "$OutputDir\startup-$timestamp.json") {
            Write-Success "Profile saved to: $OutputDir\startup-$timestamp.json"
            Write-Host ""
            Write-Host "To view the profile:" -ForegroundColor Cyan
            Write-Host "  samply load `"$OutputDir\startup-$timestamp.json`"" -ForegroundColor White
        }
    }
    
    "dtrace" {
        Write-Warning "DTrace profiling on Windows requires ETW (Event Tracing for Windows)"
        Write-Host "Consider using samply instead for easier flame graph generation"
    }
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
Write-Host "  Profiling Complete" -ForegroundColor Blue
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Blue
Write-Host ""

# Print current baseline metrics
Write-Host "Current Baseline Metrics (from docs/STARTUP_OPTIMIZATION_PHASE_1.md):" -ForegroundColor Cyan
Write-Host "  • GUI startup: ~2ms"
Write-Host "  • Initial 200 tracks: ~14.5ms"  
Write-Host "  • Full library (11.6k tracks): ~133ms"
Write-Host ""
Write-Host "Target: <100ms time-to-interactive for 50k+ tracks"
Write-Host ""
