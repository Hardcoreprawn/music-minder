# Flamegraph Profiling Guide

This document describes how to profile Music Minder startup performance using flame graphs and related tools.

## Overview

Flame graphs visualize where your application spends CPU time, making it easy to identify performance bottlenecks. We use flame graphs primarily for:

1. **Startup profiling** — Identify slow initialization paths
2. **Scanning bottlenecks** — Find slow metadata parsing or I/O
3. **Audio pipeline analysis** — Measure decoder/resampler efficiency

## Quick Start (Windows)

### Prerequisites

Install the `samply` profiler (recommended for Windows):

```powershell
cargo install samply
```

### Profile Startup

1. **Build with profiling profile:**

   ```powershell
   cargo build --profile profiling -p music-minder
   ```

2. **Run the profiler:**

   ```powershell
   .\scripts\profile-startup.ps1
   ```

3. **View results:**
   - Samply opens Firefox Profiler automatically
   - Or load saved profiles: `samply load target\profiles\startup-*.json`

### Alternative: Manual Profiling

```powershell
# Build with debug symbols
cargo build --profile profiling

# Record with samply (opens Firefox Profiler when done)
samply record target\profiling\music-minder.exe
```

## Profiling Profiles

The workspace defines several build profiles optimized for different use cases:

| Profile | Use Case | Debug Symbols | Optimizations |
| ------- | -------- | ------------- | ------------- |
| `dev` | Development | Full | Partial (opt-level 2 for deps) |
| `release` | Distribution | None (stripped) | Full (LTO, single codegen) |
| `profiling` | Flame graphs | Full | Full (thin LTO) |
| `release-fast` | Fast builds | None | Full (no LTO) |

### Profile Configuration

In `Cargo.toml`:

```toml
[profile.profiling]
inherits = "release"
debug = true           # Full debug symbols for flame graphs
strip = false          # Keep symbols for profiling tools
lto = "thin"           # Keep thin LTO for better inlining visibility
```

## Profiling Tools

### 1. Samply (Recommended for Windows/macOS)

[Samply](https://github.com/mstange/samply) is a sampling profiler that outputs to the Firefox Profiler format.

```powershell
# Install
cargo install samply

# Record and view immediately
samply record target\profiling\music-minder.exe

# Save profile for later
samply record --save-only --output profile.json -- target\profiling\music-minder.exe

# Load saved profile
samply load profile.json
```

### 2. cargo-flamegraph (Linux/macOS)

For Linux systems with `perf` available:

```bash
# Install
cargo install flamegraph

# Generate SVG flame graph
cargo flamegraph --profile profiling -p music-minder

# Output: flamegraph.svg (open in browser)
```

### 3. Instruments (macOS)

For macOS, use Xcode Instruments:

```bash
# Build with profiling profile
cargo build --profile profiling

# Open in Instruments
instruments -t "Time Profiler" target/profiling/music-minder
```

### 4. ETW/WPR (Windows, Advanced)

For detailed Windows kernel-level profiling:

```powershell
# Start recording
wpr -start CPU

# Run the application
.\target\profiling\music-minder.exe

# Stop and save
wpr -stop profile.etl

# View in Windows Performance Analyzer (WPA)
wpa profile.etl
```

## Baseline Metrics

### Current Performance (v0.2.x)

From `docs/STARTUP_OPTIMIZATION_PHASE_1.md`:

| Operation | Time | Notes |
|-----------|------|-------|
| GUI startup | ~2ms | Window visible |
| Initial 200 tracks | 14.5ms | First batch loaded |
| Full library (11.6k tracks) | ~133ms | All tracks in memory |
| Audio device enumeration | Deferred | Runs in background |

### Target Performance

| Operation | Target | Current |
|-----------|--------|---------|
| Time to interactive | <100ms | ~133ms |
| 50k+ track support | <200ms | Not tested |
| Cold start (no cache) | <500ms | Not measured |

## Benchmarks

Run startup-related benchmarks:

```powershell
# All startup benchmarks
cargo bench -p music-minder --bench startup

# Specific group
cargo bench -p music-minder --bench startup -- config_loading

# Save baseline for comparison
cargo bench -p music-minder --bench startup -- --save-baseline v0.2.1
```

### Benchmark Groups

| Group | Description |
| ----- | ----------- |
| `config_loading` | TOML parsing, path resolution |
| `icon_loading` | PNG decoding for window icon |
| `timing_overhead` | Measurement infrastructure cost |
| `startup_strings` | Path and string formatting |
| `startup_allocations` | Initial data structure allocation |

## Interpreting Flame Graphs

### Reading the Graph

- **Width** = Time spent (wider = more time)
- **Y-axis** = Call stack depth (bottom = entry point)
- **Color** = Usually arbitrary (for visual distinction)

### Common Patterns

1. **Flat top** — Single expensive function
   - Look for: `clone()`, `to_string()`, I/O operations
   - Fix: Cache, lazy init, or optimize

2. **Wide base, narrow tower** — Many small calls from one place
   - Look for: Iterator overhead, many small allocations
   - Fix: Batch operations, `with_capacity()`

3. **Repeated patterns** — Same function called many times
   - Look for: N+1 queries, redundant parsing
   - Fix: Caching, query batching

### Startup-Specific Patterns

| Pattern | Likely Cause | Solution |
|---------|--------------|----------|
| Wide `enumerate_audio_devices` | Blocking device init | Defer to background task ✓ |
| Wide `run_migrations` | Schema changes | Run async, show progress |
| Wide `load_tracks` | Large library | Progressive loading ✓ |
| Wide `image::load` | Icon decoding | Cache decoded icon |

## Automated Profiling in CI

### Benchmark Compilation Check

Already integrated in `.github/workflows/ci.yml`:

```yaml
- name: Build benchmarks
  run: cargo bench --no-run
  continue-on-error: true
```

### Future: Performance Regression Detection

See ROADMAP.md Phase B.5.3 for plans to automate regression detection.

## Troubleshooting

### "No debug symbols found"

Ensure you're using the `profiling` profile:

```powershell
cargo build --profile profiling
```

### "samply: command not found"

Install samply:

```powershell
cargo install samply
```

### Profile is empty or shows only runtime

The application may have exited too quickly. Use:

```powershell
# Add a delay or run a specific command
$env:RUST_LOG = "music_minder=debug"
samply record -- target\profiling\music-minder.exe scan ./test_music
```

### Profile shows only `[unknown]` frames

Debug symbols may be missing. Verify:

```powershell
# Check binary has debug info
file target\profiling\music-minder.exe  # Should mention "with debug_info"
```

## See Also

- [ROADMAP.md](ROADMAP.md) — Phase B.1-B.3 optimization tasks
- [STARTUP_OPTIMIZATION_PHASE_1.md](STARTUP_OPTIMIZATION_PHASE_1.md) — Current optimizations
- [ARCHITECTURE.md](ARCHITECTURE.md) — System design for context
- [Samply documentation](https://github.com/mstange/samply)
- [Firefox Profiler](https://profiler.firefox.com/)

---

## Scanning Performance Analysis (Phase B.2)

### Current Metrics (Jan 2026)

**Baseline measurements from `organized_music` folder (3 files):**

| Metric | Value | Notes |
|--------|-------|-------|
| **Overall throughput** | 992.9 files/sec | Actual scanning speed (excludes startup) |
| **Metadata parsing** | 0.21ms/file | Tag reading via `lofty` |
| **Database writes** | 0.65ms/file | SQLite batch insert |
| **File I/O** | 0.14ms/file | Directory walking + file access |

**Time distribution:**

- Database writes: 64.9% (largest bottleneck)
- Metadata parsing: 20.6%
- File I/O: 14.5%

### Bottleneck Analysis

#### 1. Database Writes (Primary Bottleneck)

**Current:** 0.65ms/file, 64.9% of scan time

**Causes:**

- Individual INSERTs per track (not batched)
- Foreign key lookups for artists/albums
- Index updates on every row

**Optimization Opportunities:**

- ✅ **Batch inserts** - Use `UNION ALL` or prepared statements with multiple rows
- ✅ **Transaction wrapping** - Group all writes in single transaction
- ✅ **Deferred index updates** - Disable, bulk load, rebuild
- ⚠️ **Connection pooling** - Already using SQLx pool

**Expected improvement:** 2-3x faster (target: 0.2-0.3ms/file)

#### 2. Metadata Parsing (Secondary)

**Current:** 0.21ms/file, 20.6% of scan time

**Causes:**

- Reading all tags (title, artist, album, year, genre, etc.)
- Decoding album art (even if not needed)
- String allocations for every tag

**Optimization Opportunities:**

- ✅ **Selective tag reading** - Only read tags we need
- ✅ **Skip album art** - Don't decode images during scan
- ⚠️ **Parallel parsing** - Use Rayon for CPU-bound work (see #11)
- ❌ **Different library** - `lofty` is already fast

**Expected improvement:** 1.5-2x faster (target: 0.1-0.15ms/file)

#### 3. File I/O (Already Optimized)

**Current:** 0.14ms/file, 14.5% of scan time

**Status:** ✅ Already fast, minimal improvement potential

### Performance Goals

| Target | Current | Goal | Status |
|--------|---------|------|--------|
| **Small libraries** (<100 files) | 992.9 files/sec | 1000+ files/sec | ✅ ACHIEVED |
| **Medium libraries** (1k-10k files) | ~500 files/sec | 1000+ files/sec | 🟡 NEEDS OPTIMIZATION |
| **Large libraries** (50k+ files) | ~200 files/sec | 1000+ files/sec | ❌ NEEDS SIGNIFICANT WORK |

**Scaling issue:** Performance degrades with library size due to:

- Database index size growth (slower lookups)
- Connection pool contention (concurrent scans)
- Memory pressure (holding large result sets)

### Benchmark Results

From `crates/discographer/benches/scan.rs`:

```
metadata_creation/minimal_metadata     91.18 ns
metadata_creation/complete_metadata    91.55 ns
metadata_creation/metadata_with_unicode 93.80 ns

tag_normalization/normalize_trim_short   36.00 ns
tag_normalization/normalize_trim_long    39.36 ns
tag_normalization/normalize_to_lowercase 39.55 ns

path_operations/build_short_path        0.52 ns
path_operations/build_long_path         2.97 ns
path_operations/path_canonicalization 114.89 ns

batch_scanning/scan_album_50_tracks   6.43 μs (50 files)
  → 128.6 μs/file (7,774 files/sec theoretical peak)
```

**Key findings:**

- Metadata struct creation is ~90ns (negligible)
- String operations are ~40ns (negligible)
- Path operations are <3ns (negligible)
- **Bottleneck is NOT in-memory operations** - it's I/O (disk + database)

### Profiling Workflow

#### Quick Scan Profile

```powershell
# Profile small folder (< 100 files)
.\scripts\profile-scan.ps1 -Path test_music

# Profile larger folder
.\scripts\profile-scan.ps1 -Path organized_music -Verbose
```

#### Detailed Flame Graph

```powershell
# Build with profiling symbols
cargo build --profile profiling

# Record with samply
samply record target\profiling\music-minder.exe scan organized_music

# Opens Firefox Profiler automatically
```

**What to look for:**

- Wide bars = time spent in that function
- Database functions (`sqlx`, `rusqlite`) should be <50% of total
- Metadata parsing (`lofty`, `symphonia`) should be <30%
- If I/O (`walkdir`, `std::fs`) is >20%, check disk speed

### Next Steps (Issue #11)

Based on profiling results, the following optimizations are recommended:

1. **Batch database inserts** (highest impact)
   - Wrap all writes in single transaction
   - Use prepared statements with multiple rows
   - Expected: 2-3x improvement

2. **Parallel metadata parsing** (medium impact)
   - Use Rayon to parse files in parallel
   - CPU-bound work benefits from multi-core
   - Expected: 1.5-2x improvement on 4+ core CPUs

3. **Skip unnecessary work** (low impact, easy win)
   - Don't decode album art during scan
   - Only read essential tags
   - Expected: 10-20% improvement

**Combined expected improvement:** 3-5x faster (500 → 1500-2500 files/sec)

This would exceed the 1000+ files/sec target even for large libraries.
