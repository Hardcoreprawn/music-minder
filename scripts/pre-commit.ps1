# Pre-commit hook for music-minder (PowerShell version for Windows)
# Runs formatting and lint checks before allowing commits

$ErrorActionPreference = "Stop"

Write-Host "🔍 Running pre-commit checks..." -ForegroundColor Cyan

# Check formatting
Write-Host "📝 Checking formatting..." -ForegroundColor Yellow
$fmtResult = cargo fmt --all -- --check 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "❌ Formatting check failed!" -ForegroundColor Red
    Write-Host "   Run 'cargo fmt' to fix formatting issues." -ForegroundColor Red
    Write-Host $fmtResult
    exit 1
}

# Run clippy
Write-Host "📎 Running clippy..." -ForegroundColor Yellow
$clippyResult = cargo clippy --all-targets -- -D warnings 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "❌ Clippy found warnings!" -ForegroundColor Red
    Write-Host "   Fix the issues above before committing." -ForegroundColor Red
    Write-Host $clippyResult
    exit 1
}

Write-Host "✅ All pre-commit checks passed!" -ForegroundColor Green
exit 0
