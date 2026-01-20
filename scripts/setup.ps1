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
    
    # Create hooks directory if it doesn't exist
    if (-not (Test-Path $hooksDir)) {
        New-Item -ItemType Directory -Path $hooksDir -Force | Out-Null
    }
    
    # Install pre-commit hook
    $preCommitDest = Join-Path $hooksDir "pre-commit"
    $preCommitContent = @"
#!/bin/sh
# Auto-generated hook - calls pre-commit script
# Detects OS and calls appropriate script

repo_root="`$(git rev-parse --show-toplevel)"

if command -v powershell.exe >/dev/null 2>&1; then
    # Windows with PowerShell
    powershell.exe -ExecutionPolicy Bypass -File "`$repo_root/scripts/pre-commit.ps1"
else
    # Unix-like systems (Linux, macOS)
    "`$repo_root/scripts/hooks/pre-commit"
fi
"@
    Set-Content -Path $preCommitDest -Value $preCommitContent -NoNewline
    Write-Host "  ✅ Pre-commit hook installed" -ForegroundColor Green
    
    # Install commit-msg hook
    $commitMsgDest = Join-Path $hooksDir "commit-msg"
    $commitMsgContent = @"
#!/bin/sh
# Auto-generated hook - calls commit-msg script
# Detects OS and calls appropriate script

repo_root="`$(git rev-parse --show-toplevel)"

if command -v powershell.exe >/dev/null 2>&1; then
    # Windows with PowerShell
    powershell.exe -ExecutionPolicy Bypass -File "`$repo_root/scripts/commit-msg.ps1" "`$1"
else
    # Unix-like systems (Linux, macOS)
    "`$repo_root/scripts/hooks/commit-msg" "`$1"
fi
"@
    Set-Content -Path $commitMsgDest -Value $commitMsgContent -NoNewline
    Write-Host "  ✅ Commit-msg hook installed" -ForegroundColor Green
    
    # Make bash scripts executable on Unix
    if ($IsLinux -or $IsMacOS) {
        chmod +x "$repoRoot/scripts/hooks/pre-commit"
        chmod +x "$repoRoot/scripts/hooks/commit-msg"
    }
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
Write-Host "Git hooks will automatically run before commits to check:" -ForegroundColor Cyan
Write-Host "  • Code formatting (cargo fmt)" -ForegroundColor White
Write-Host "  • Lint warnings (cargo clippy)" -ForegroundColor White
Write-Host "  • Commit message format (Conventional Commits)" -ForegroundColor White
Write-Host ""
Write-Host "Example valid commit messages:" -ForegroundColor Cyan
Write-Host "  feat(ui): add dark mode support" -ForegroundColor Green
Write-Host "  fix(db): eliminate race condition" -ForegroundColor Green
Write-Host "  docs: update README" -ForegroundColor Green
Write-Host ""
Write-Host "To skip hooks temporarily, use: git commit --no-verify" -ForegroundColor Gray
