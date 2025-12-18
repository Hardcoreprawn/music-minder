# Setup script for development environment
# Run this once after cloning the repository

param(
    [switch]$SkipHooks
)

Write-Host "🔧 Setting up Music Minder development environment..." -ForegroundColor Cyan

# Install git hooks
if (-not $SkipHooks) {
    Write-Host "📎 Installing git hooks..." -ForegroundColor Yellow
    
    $repoRoot = Split-Path $PSScriptRoot -Parent
    $hooksDir = Join-Path $repoRoot ".git\hooks"
    $preCommitDest = Join-Path $hooksDir "pre-commit"
    
    # Create hooks directory if it doesn't exist
    if (-not (Test-Path $hooksDir)) {
        New-Item -ItemType Directory -Path $hooksDir -Force | Out-Null
    }
    
    # Create a wrapper script that calls our PowerShell hook
    $hookContent = @"
#!/bin/sh
# Auto-generated hook - calls PowerShell pre-commit script
powershell.exe -ExecutionPolicy Bypass -File "`$(git rev-parse --show-toplevel)/scripts/pre-commit.ps1"
"@
    
    Set-Content -Path $preCommitDest -Value $hookContent -NoNewline
    Write-Host "  ✅ Pre-commit hook installed" -ForegroundColor Green
}

# Check for required tools
Write-Host "🔍 Checking required tools..." -ForegroundColor Yellow

$tools = @(
    @{ Name = "cargo"; Check = { cargo --version } },
    @{ Name = "rustfmt"; Check = { cargo fmt --version } },
    @{ Name = "clippy"; Check = { cargo clippy --version } }
)

$allGood = $true
foreach ($tool in $tools) {
    try {
        $null = & $tool.Check 2>&1
        Write-Host "  ✅ $($tool.Name) found" -ForegroundColor Green
    } catch {
        Write-Host "  ❌ $($tool.Name) not found" -ForegroundColor Red
        $allGood = $false
    }
}

if (-not $allGood) {
    Write-Host ""
    Write-Host "Some tools are missing. Install them with:" -ForegroundColor Yellow
    Write-Host "  rustup component add rustfmt clippy" -ForegroundColor White
}

Write-Host ""
Write-Host "✨ Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "The pre-commit hook will automatically run before each commit to check:" -ForegroundColor Cyan
Write-Host "  • Code formatting (cargo fmt)" -ForegroundColor White
Write-Host "  • Lint warnings (cargo clippy)" -ForegroundColor White
Write-Host ""
Write-Host "To skip hooks temporarily, use: git commit --no-verify" -ForegroundColor Gray
