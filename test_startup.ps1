# Test startup performance by redirecting stderr to a file
$env:RUST_LOG = "music_minder=debug,music_minder::db=debug,music_minder::ui=debug"

# Run the app and redirect stderr to a log file
Write-Host "Starting Music Minder with debug logging..."
Write-Host "Logs will be saved to startup_test.log"

$proc = Start-Process .\target\debug\music-minder.exe -NoNewWindow -RedirectStandardError startup_test.log -PassThru

# Let it run for 5 seconds
Start-Sleep -Seconds 5

# Stop the app
Stop-Process $proc -Force -ErrorAction SilentlyContinue

# Show relevant startup logs
Write-Host "`n=== STARTUP TIMINGS ===" 
Select-String -Path startup_test.log -Pattern "Startup|startup|Database|database|Loading|loading|Audio|audio|Device|device|Time|time" | Select-Object -First 30
