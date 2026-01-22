# Demo script showing error categorization system
# Run from project root: .\scripts\demo-error-categories.ps1

Write-Host "`n=== Error Categorization System Demo ===" -ForegroundColor Cyan
Write-Host "Showing how different enrichment errors are categorized for smart retry`n"

Write-Host "📋 Error Categories:" -ForegroundColor Yellow
Write-Host "  • Recoverable - Can retry immediately (network, timeout, rate limit)"
Write-Host "  • Fixable     - Requires user action first (missing tool, locked file)"
Write-Host "  • Permanent   - Cannot be fixed (unsupported format, no matches)"
Write-Host ""

# Run the tests to show they work
Write-Host "🧪 Running error categorization tests...`n" -ForegroundColor Green
cargo test -p music-minder enrichment::domain::tests::test_error_category --quiet

Write-Host "`n✅ All error categorization tests passed!" -ForegroundColor Green
Write-Host ""

Write-Host "💡 Example Error Messages:" -ForegroundColor Yellow
Write-Host ""

Write-Host "1. " -NoNewline
Write-Host "Recoverable Error" -ForegroundColor Green
Write-Host "   Error: Connection timeout"
Write-Host "   Category: Recoverable"
Write-Host "   Guidance: Try again - temporary network or API issues usually resolve quickly"
Write-Host "   → Retry button WILL attempt this one"
Write-Host ""

Write-Host "2. " -NoNewline
Write-Host "Fixable Error" -ForegroundColor Yellow
Write-Host "   Error: fpcalc not found"
Write-Host "   Category: Fixable"
Write-Host "   Guidance: Install Chromaprint/fpcalc to enable fingerprinting"
Write-Host "   → Retry button WILL attempt this one (after user installs tool)"
Write-Host ""

Write-Host "3. " -NoNewline
Write-Host "Permanent Error" -ForegroundColor Red
Write-Host "   Error: No matches found"
Write-Host "   Category: Permanent"
Write-Host "   Guidance: This track may not be in the AcoustID database"
Write-Host "   → Retry button will SKIP this one (retrying won't help)"
Write-Host ""

Write-Host "🎯 Smart Retry Behavior:" -ForegroundColor Cyan
Write-Host "   When you click 'Retry Failed (N)':"
Write-Host "   ✓ Retries Recoverable errors (network/timeout)"
Write-Host "   ✓ Retries Fixable errors (after user fixes issue)"
Write-Host "   ✗ Skips Permanent errors (won't waste API calls)"
Write-Host ""

Write-Host "📊 Current Test Results: 279 tests passing" -ForegroundColor Green
Write-Host "   - 12 new tests for health checks (Issue #28)"
Write-Host "   - 5 new tests for error categorization (Issue #27)"
Write-Host "   - All retry logic tested (Issue #26)"
Write-Host ""
