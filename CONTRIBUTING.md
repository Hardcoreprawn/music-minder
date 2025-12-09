# Contributing to Music Minder

Thank you for your interest in contributing! This document explains our development workflow.

## Development Workflow

We use a **PR-based workflow** with automated versioning:

1. **Create a feature branch** from `main`
2. **Make your changes** with conventional commits
3. **Open a Pull Request** against `main`
4. **CI checks run automatically** (tests, formatting, clippy)
5. **Get review and merge**
6. **Release Please** automatically creates release PRs

## Conventional Commits

We use [Conventional Commits](https://www.conventionalcommits.org/) for automatic versioning.
Your commit messages should follow this format:

```text
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description | Version Bump |
|------|-------------|--------------|
| `feat` | A new feature | Minor |
| `fix` | A bug fix | Patch |
| `perf` | Performance improvement | Patch |
| `refactor` | Code refactoring | None |
| `docs` | Documentation only | None |
| `test` | Adding/updating tests | None |
| `chore` | Maintenance tasks | None |
| `ci` | CI/CD changes | None |

### Breaking Changes

Add `!` after the type or include `BREAKING CHANGE:` in the footer:

```text
feat!: remove deprecated playback API

BREAKING CHANGE: The old playback API has been removed.
Use the new unified playback interface instead.
```

### Examples

```bash
# Feature (bumps minor version: 0.1.0 -> 0.2.0)
git commit -m "feat(player): add equalizer with 10-band control"

# Bug fix (bumps patch version: 0.1.0 -> 0.1.1)
git commit -m "fix(scanner): handle files with special characters"

# Refactoring (no version bump)
git commit -m "refactor(player): unify playback initiation"

# Documentation (no version bump)
git commit -m "docs: update installation instructions"
```

## Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/Hardcoreprawn/music-minder.git
cd music-minder

# Run setup script (installs git hooks)
# Windows PowerShell:
.\scripts\setup.ps1

# Or manually install hooks:
# The pre-commit hook runs cargo fmt and clippy before each commit

# Build in debug mode
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Build release
cargo build --release
```

## Pre-commit Hooks

We use pre-commit hooks to catch issues before they reach CI:

- **Formatting**: `cargo fmt --check`
- **Linting**: `cargo clippy -- -D warnings`

The hooks are installed automatically by `scripts/setup.ps1`. If a check fails,
the commit is blocked until you fix the issue.

To bypass hooks temporarily (not recommended):

```bash
git commit --no-verify
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- Add tests for new functionality
- Update documentation for public APIs

## Pull Request Process

1. **Branch naming**: Use descriptive names like `feat/equalizer` or `fix/scanner-unicode`

2. **Keep PRs focused**: One feature or fix per PR

3. **Write good descriptions**: Explain what and why

4. **Update tests**: Add or update tests as needed

5. **CI must pass**: All checks must be green before merge

## Release Process

Releases are automated via [Release Please](https://github.com/googleapis/release-please):

1. When PRs with `feat:` or `fix:` commits are merged, Release Please opens a release PR
2. The release PR accumulates changes and updates the version
3. When the release PR is merged, a new GitHub Release is created
4. The release workflow builds installers and uploads them

You don't need to manually tag versions or update Cargo.toml - it's all automated!

## Questions?

Open an issue or start a discussion on GitHub.
