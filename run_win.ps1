$env:CARGO_TARGET_DIR="target_win"
# Add cargo to path for this session if it's missing
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
}
cargo run
