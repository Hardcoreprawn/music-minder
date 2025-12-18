# Pre-commit hook for music-minder (PowerShell version for Windows)
# Runs formatting and lint checks before allowing commits

# Check formatting
Write-Host "[*] Running pre-commit checks..." -ForegroundColor Cyan
Write-Host "[*] Checking formatting..." -ForegroundColor Yellow

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "[X] Formatting check failed!" -ForegroundColor Red
    Write-Host "    Run 'cargo fmt' to fix formatting issues." -ForegroundColor Red
    exit 1
}

# Run clippy
Write-Host "[*] Running clippy..." -ForegroundColor Yellow
cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "[X] Clippy found warnings!" -ForegroundColor Red
    Write-Host "    Fix the issues above before committing." -ForegroundColor Red
    exit 1
}

Write-Host "[OK] All pre-commit checks passed!" -ForegroundColor Green
exit 0
