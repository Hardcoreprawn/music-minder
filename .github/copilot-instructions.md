# GitHub Copilot Instructions for Music Minder

This document provides guidelines for GitHub Copilot when working on the Music Minder repository.

## Project Overview

Music Minder is a native cross-platform music player and library manager built with Rust. It's a love letter to Winamp, focusing on native performance, low-latency audio playback, and a CLI-first development approach.

## Core Principles

### 🎵 Audio-First Design

The primary goal is excellent audio playback. When working on audio code:

- **Never block in the audio callback** - use atomics only, no locks, no allocation
- **Maintain real-time safety** - the audio pipeline must be deterministic
- **Bit-perfect playback** - avoid unnecessary DSP unless explicitly enabled
- Use the lock-free ring buffer (`rtrb`) for producer-consumer communication
- All audio state management uses `AudioSharedState` with atomics

### 🔧 CLI-First Development

**Every feature is built CLI-first, then wrapped with GUI.** When adding new features:

1. Implement core logic as a library function
2. Expose via `clap` CLI with `--verbose`, `--json`, `--dry-run` flags
3. Add `tracing` instrumentation at key decision points
4. Wire GUI as thin layer calling the same logic

This enables AI-assisted development, testability, debuggability, and composability.

## Code Style & Conventions

### Rust Style

- **Edition**: Rust 2024
- **Formatting**: Run `cargo fmt --all` before committing
- **Linting**: Run `cargo clippy --all-targets -- -D warnings` and fix all warnings
- **Error Handling**: Use `anyhow::Result` for application errors, `thiserror` for library errors
- **Async Runtime**: Use `tokio` for async operations
- **Logging**: Use `tracing` crate with appropriate spans and events

### Naming Conventions

- Use descriptive names that match the domain (music, audio, playback)
- Prefer full words over abbreviations unless the abbreviation is standard (e.g., FFT, DSP)
- Module names should be lowercase with underscores (snake_case)

### Comments

- Add doc comments (`///`) for all public APIs
- Use inline comments sparingly - prefer self-documenting code
- Add comments for non-obvious audio optimizations or real-time safety considerations

## Workspace Structure

This is a Cargo workspace with multiple crates:

- **`music-minder`**: Main application (GUI + CLI)
- **`symphonium`**: Audio playback engine (decoder, resampler, output)
- **`soundstore`**: Database layer for music library
- **`discographer`**: Metadata enrichment (MusicBrainz, AcoustID)
- **`music_journo`**: Metadata reading/writing (lofty wrapper)

When making changes, consider which crate the code belongs in to maintain separation of concerns.

## Building, Testing & Linting

### Build Commands

```bash
# Debug build (faster compilation, includes debug symbols)
cargo build

# Release build (optimized)
cargo build --release

# Fast release build (for iteration, less optimization)
cargo build --profile release-fast
```

### Testing

```bash
# Run all tests (recommended: install cargo-nextest for faster test execution)
cargo nextest run --all-targets

# Run tests for a specific crate
cargo test -p music-minder

# Run tests with logging output
RUST_LOG=debug cargo test
```

### Linting

```bash
# Check formatting (CI will fail if not formatted)
cargo fmt --all -- --check

# Run clippy (CI will fail on warnings)
cargo clippy --all-targets -- -D warnings

# Auto-fix clippy warnings where possible
cargo clippy --all-targets --fix
```

### System Dependencies

- **Linux**: `libasound2-dev libdbus-1-dev pkg-config`
- **Windows**: Visual Studio Build Tools
- **macOS**: Xcode Command Line Tools

## Commit Conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic versioning:

```
<type>(<scope>): <description>
```

### Common Types

- `feat`: New feature (bumps minor version)
- `fix`: Bug fix (bumps patch version)
- `perf`: Performance improvement (bumps patch version)
- `refactor`: Code refactoring (no version bump)
- `docs`: Documentation only (no version bump)
- `test`: Adding/updating tests (no version bump)
- `chore`: Maintenance tasks (no version bump)
- `ci`: CI/CD changes (no version bump)

### Examples

```bash
feat(player): add equalizer with 10-band control
fix(scanner): handle files with special characters in names
refactor(audio): unify playback state management
docs: update installation instructions for Linux
```

For breaking changes, add `!` after the type or include `BREAKING CHANGE:` in the footer.

## Common Patterns

### Error Handling

```rust
use anyhow::{Context, Result};

pub fn process_audio_file(path: &Path) -> Result<()> {
    let metadata = read_metadata(path)
        .context("Failed to read audio metadata")?;
    // ...
    Ok(())
}
```

### Async Operations

```rust
use tokio::task;

// Spawn CPU-intensive work on blocking thread pool
let result = task::spawn_blocking(move || {
    heavy_computation()
}).await?;
```

### Tracing

```rust
use tracing::{info, debug, warn, error, instrument};

#[instrument(skip(data))]
pub async fn process_batch(items: Vec<Item>) -> Result<()> {
    info!("Processing batch of {} items", items.len());
    debug!("Starting batch processing");
    // ...
    Ok(())
}
```

## Audio Code Guidelines

When working on audio-related code in the `symphonium` crate:

1. **Never use `Mutex` or `RwLock` in the audio callback** - use atomics only
2. **No heap allocation in the audio callback** - pre-allocate buffers
3. **No blocking I/O in the audio callback** - decoder runs in separate thread
4. **Test on multiple sample rates** - ensure resampling works correctly
5. **Profile performance** - audio code should be highly optimized
6. **Use SIMD when beneficial** - check `player/simd.rs` for patterns

## Dependencies

When adding dependencies:

- Prefer well-maintained crates with active communities
- Check license compatibility (MIT, Apache-2.0, BSD are preferred)
- Add to `[workspace.dependencies]` in root `Cargo.toml` to avoid duplication
- Consider binary size impact for release builds

## Security

- Never commit secrets or API keys
- Use `reqwest` with `rustls-tls` (not native TLS) for consistent behavior
- Sanitize user input for file paths (prevent directory traversal)
- Be cautious with `unsafe` code - document why it's necessary and safe

## Performance Considerations

- Use `rayon` for CPU-parallel operations on collections
- Profile before optimizing - use `cargo flamegraph` or `perf`
- Consider `SmallVec` for small, known-size vectors to avoid heap allocation
- Use `parking_lot` locks if `std::sync` locks show contention

## UI Guidelines (Iced)

- Follow the Elm architecture: Model-Update-View
- Keep `Message` enum variants clear and descriptive
- Use `Command` for async operations that produce messages
- Prefer composition of widgets over monolithic view functions

## Documentation

- Update relevant docs in `docs/` for architectural changes
- Keep README.md accurate with feature changes
- Update ROADMAP.md when completing milestone features
- Add ADRs (Architecture Decision Records) for significant design decisions

## CI/CD

The CI pipeline runs:

1. **Formatting check** - `cargo fmt --check`
2. **Clippy linting** - `cargo clippy -- -D warnings`
3. **Tests** - `cargo nextest run` on Linux (every PR/push) and Windows (main only)
4. **Security audit** - `cargo audit` (on main branch only)
5. **CodeQL analysis** - Weekly schedule + security-sensitive changes only

All checks must pass before merge. Use the pre-commit hook (installed via `scripts/setup.ps1`) to catch issues early.

### CI Optimization Guidelines

**Use Pull Requests for Sprint Work** - Instead of pushing directly to main:
- Batch related commits into a feature branch
- Create a PR to run CI once on the complete changeset
- This reduces CI runs by ~10x and saves significant compute time
- Direct pushes to main trigger full CI (Linux + Windows) on every commit; CodeQL runs on its own schedule and security-focused PR triggers

**When to skip CI:**
- Use `[ci skip]` in commit messages for docs-only changes
- Markdown files and `docs/` folder are already excluded via `paths-ignore`

**Windows tests are expensive:**
- Windows runners cost 2x Linux runners
- Windows tests only run on main pushes, not on every PR
- Ensure platform-specific code is well-tested locally before pushing

**CodeQL is for security, not everyday bugs:**
- Runs weekly + on dependency/workflow changes only
- Rust's compiler catches most issues CodeQL would find in other languages
- Don't rely on CodeQL for general code quality - use clippy instead

## Release Process

Releases are automated via Release Please:

- Merging PRs with `feat:` or `fix:` commits creates a release PR
- The release PR updates CHANGELOG.md and version numbers
- Merging the release PR creates a GitHub release and builds installers

Don't manually edit version numbers in `Cargo.toml` - let Release Please handle it.

## Questions or Clarifications

If you're unsure about:

- **Architecture decisions**: Check `docs/ARCHITECTURE.md`
- **Current status**: Check `docs/STATUS.md`
- **Contributing workflow**: Check `CONTRIBUTING.md`
- **Roadmap priorities**: Check `docs/ROADMAP.md`

When in doubt, follow existing patterns in the codebase and prefer minimal, focused changes.
