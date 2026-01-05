# Security Tooling Strategy

## Overview

This document outlines the security tools integrated into Music Minder's CI/CD pipeline and development workflow.

**Philosophy:** Defense in depth with automated checks at multiple layers.

---

## Current Tools

### ✅ cargo-audit (DEPENDENCY SCANNING)

**What it does:** Scans Cargo.lock for known vulnerabilities in dependencies

**Status:** Active in CI

- Runs on: `push` to main (skipped on PRs to save time)
- Ignores: Unmaintained crates (configurable)
- Blocks: CI if vulnerability found

**Usage:**

```bash
cargo audit --ignore unmaintained
```

---

## Tier 1: High Priority (IMPLEMENT NOW)

### cargo-deny

**What it does:**

- License compliance checking (ensure no GPL/conflicting licenses)
- Dependency policy enforcement (ban specific crates)
- Duplicate dependency detection
- Advisories (like audit but with additional policies)

**Why:** Essential for production software

- Ensures legal compliance
- Prevents dependency bloat
- Catches duplicate/conflicting versions early

**Installation:**

```bash
cargo install cargo-deny
```

**Configuration:** Create `.cargo/deny.toml`

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
unsound = "deny"

[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 OR MIT", "ISC"]
deny = ["GPL-2.0", "AGPL-3.0"]

[bans]
# Prevent duplicate dependencies
multiple-versions = "deny"
# Deny specific problematic crates
deny = [
  # Examples
  # { name = "openssl", version = "*" }  # Use rustls instead
]

[sources]
# Ensure all crates come from crates.io or allow list
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

**CI Integration:**

```yaml
- name: Check dependencies with cargo-deny
  uses: EmbarkStudios/cargo-deny-action@v1
  with:
    log-level: warn
```

**Timeline:** Add in next CI update (Phase B.6.1)

---

### cargo-udeps

**What it does:** Identifies unused dependencies in Cargo.toml

**Why:** Keeps dependency count minimal

- Smaller attack surface
- Faster builds
- Cleaner codebase

**Installation:**

```bash
cargo +nightly install cargo-udeps
```

**Usage:**

```bash
cargo +nightly udeps --all-targets
```

**Note:** Requires nightly, so run manually or in separate CI job

**Timeline:** Add as optional local tool (Phase B.6.2)

---

### cargo-outdated

**What it does:** Shows outdated dependencies and update paths

**Why:** Proactive security maintenance

- Know which dependencies need updates
- Plan upgrade strategy
- Avoid falling far behind

**Installation:**

```bash
cargo install cargo-outdated
```

**Usage:**

```bash
cargo outdated
```

**CI Integration:**

```yaml
- name: Check for outdated dependencies
  run: cargo outdated --exit-code 1 --root-deps-only
  continue-on-error: true  # Warn but don't fail
```

**Timeline:** Add as informational check (Phase B.6.3)

---

## Tier 2: Medium Priority (ADD LATER)

### rust-tarpaulin

**What it does:** Measures code coverage

**Why:** Understand which code paths are tested

- Visual regression detection
- Identify untested critical paths
- Track coverage trends

**Installation:**

```bash
cargo install cargo-tarpaulin
```

**Usage:**

```bash
cargo tarpaulin --out Html --output-dir coverage
```

**CI Integration:**

```yaml
- name: Generate code coverage
  run: cargo tarpaulin --out Xml
  
- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: ./cobertura.xml
```

**Timeline:** Add after benchmarking stabilizes (Phase B.7)

---

## Tier 3: Advanced (OPTIONAL)

### cargo-fuzz

**What it does:** Property-based fuzzing for finding edge cases

**Why:** Audio decoding and file parsing are high-value targets

- Symphonia decoder robustness
- File scanner edge cases
- Organizer path handling

**Installation:**

```bash
cargo install cargo-fuzz
```

**Setup:**

```bash
cargo fuzz init
```

**Example fuzz target** (`fuzz/fuzz_targets/decode_fuzz.rs`):

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use symphonium::AudioDecoder;

fuzz_target!(|data: &[u8]| {
    // Try to decode random data
    let _ = AudioDecoder::from_bytes(data);
});
```

**Timeline:** Add after core stability (Phase B.8, optional)

**Note:** Fuzzing is powerful but requires dedicated infrastructure. Consider starting with a subset (e.g., just decoder).

---

### miri

**What it does:** Detects undefined behavior in Rust code

**Why:** Catches subtle memory safety issues

**Installation:**

```bash
rustup +nightly component add miri
```

**Usage:**

```bash
cargo +nightly miri test
```

**Limitations:** Some features not supported; mainly useful for core algorithms

**Timeline:** Evaluate later if needed (Phase B.9, optional)

---

## Tier 4: Infrastructure (FUTURE)

### Trivy

**What it does:** Scans container images, filesystems, git repos for vulnerabilities

**Why:** If distributing Docker images or additional artifacts

**Installation:**

```bash
trivy image your-image:latest
```

**Timeline:** Only needed if containerizing the application (Post-v0.3.0)

---

## CI Job Structure (Proposed)

```yaml
jobs:
  security:
    name: Security Checks
    runs-on: ubuntu-latest
    steps:
      # Dependency vulnerabilities
      - name: cargo-audit
        run: cargo audit --ignore unmaintained
      
      # Dependency policies
      - name: cargo-deny
        run: cargo deny check advisories licenses bans
      
      # Outdated dependencies (informational)
      - name: cargo-outdated
        run: cargo outdated --root-deps-only
        continue-on-error: true
      
      # Optional: Code coverage
      - name: cargo-tarpaulin
        run: cargo tarpaulin --out Xml
        continue-on-error: true
```

---

## Maintenance Plan

### Monthly

- [ ] Review cargo-audit results
- [ ] Update `deny.toml` policies as needed

### Quarterly

- [ ] Run `cargo outdated` and plan updates
- [ ] Review code coverage trends
- [ ] Evaluate new security tools

### As-needed

- [ ] Address any fuzzing failures
- [ ] Review clippy warnings for security implications

---

## References

- [cargo-audit](https://docs.rs/cargo-audit/)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
- [cargo-udeps](https://github.com/est31/cargo-udeps)
- [cargo-outdated](https://github.com/kbknapp/cargo-outdated)
- [rust-tarpaulin](https://github.com/xd009642/tarpaulin)
- [cargo-fuzz](https://docs.rs/cargo-fuzz/)
- [OWASP Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/rust)

---

## Security Roadmap Integration

See [ROADMAP.md](ROADMAP.md) for integration into Phase B.6 (Security Hardening).
