# Commit message hook for music-minder (PowerShell version for Windows)
# Validates commit messages follow Conventional Commits format

param(
    [Parameter(Mandatory=$true)]
    [string]$CommitMessageFile
)

# Read the commit message
$commitMessage = Get-Content $CommitMessageFile -Raw

# Skip merge commits and revert commits
if ($commitMessage -match "^Merge (branch|pull request)|^Revert ") {
    exit 0
}

# Conventional Commits pattern
# Format: type(optional scope): description
# Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore
$pattern = '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9\-]+\))?!?: .{1,72}'

if ($commitMessage -notmatch $pattern) {
    Write-Host ""
    Write-Host "[X] Invalid commit message format!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Commit messages must follow Conventional Commits format:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  type(scope): description" -ForegroundColor White
    Write-Host ""
    Write-Host "Valid types:" -ForegroundColor Yellow
    Write-Host "  feat:     A new feature" -ForegroundColor White
    Write-Host "  fix:      A bug fix" -ForegroundColor White
    Write-Host "  docs:     Documentation changes" -ForegroundColor White
    Write-Host "  style:    Code style changes (formatting, etc.)" -ForegroundColor White
    Write-Host "  refactor: Code refactoring" -ForegroundColor White
    Write-Host "  perf:     Performance improvements" -ForegroundColor White
    Write-Host "  test:     Test changes" -ForegroundColor White
    Write-Host "  build:    Build system changes" -ForegroundColor White
    Write-Host "  ci:       CI/CD changes" -ForegroundColor White
    Write-Host "  chore:    Other changes (dependencies, etc.)" -ForegroundColor White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  feat(ui): add dark mode support" -ForegroundColor Green
    Write-Host "  fix(db): eliminate race condition in get_or_create" -ForegroundColor Green
    Write-Host "  docs: update README with installation steps" -ForegroundColor Green
    Write-Host "  ci: restore release-please workflow" -ForegroundColor Green
    Write-Host ""
    Write-Host "Scope is optional, description is required (1-72 chars)." -ForegroundColor Yellow
    Write-Host "Use '!' after type/scope for breaking changes: feat!: breaking change" -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

# Check description length (first line)
$firstLine = ($commitMessage -split "`n")[0]
if ($firstLine.Length -gt 100) {
    Write-Host ""
    Write-Host "[!] Warning: Commit message first line is very long (>100 chars)." -ForegroundColor Yellow
    Write-Host "    Consider making it more concise." -ForegroundColor Yellow
    Write-Host ""
    # Don't fail, just warn
}

exit 0
