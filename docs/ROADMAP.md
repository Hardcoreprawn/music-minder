# Music Minder Roadmap

## 🎯 Vision: Winamp for the Modern Era

**Music Minder is a love letter to Winamp** — the legendary audio player that defined a generation. We're building a native, fast, beautiful music player that captures that early-2000s magic while leveraging modern Rust for rock-solid performance.

### Core Principles

1. **Audio First**: Playback is sacred. Nothing interrupts the music.
2. **It Just Works**: Scan a folder, press play. No cloud accounts, no subscriptions.
3. **Retro Soul, Modern Tech**: Winamp's spirit with 2024's engineering.
4. **Native & Fast**: No Electron. No web views. Pure Rust performance.
5. **CLI-First, GUI-Second**: Every feature works from the command line first.

### The Winamp DNA

What made Winamp special:

- **Instant startup** — Ready before you blink
- **Tiny footprint** — Runs on anything
- **Visualization** — Mesmerizing spectrum analyzers
- **Global hotkeys** — Control from anywhere
- **"It really whips the llama's ass"** — Personality and fun

---

## Current Status: v0.2.0 (Phase A.5 Complete)

**237 tests passing** | **0 clippy warnings** | **5-crate modular architecture**

---

## 📋 Next Steps: Integrated Task List (v0.2.1 → v0.3.0)

> **Strategy:** Balance performance optimization, security hardening, and feature polish
>
> **Mix of types:** Testing infrastructure (T), Performance (P), Security (S), Features (F)

### Immediate (This Week)

| Priority | Task | Type | Est. Time | Block | Notes |
| -------- | ---- | ---- | --------- | ----- | ----- |
| 1️⃣ | B.6.1: ✅ Add cargo-deny to dependency scanning | S | DONE | — | Prevents legal liability, supply-chain attacks |
| 2️⃣ | B.5.1: ✅ Benchmark compilation check in CI | T | DONE | — | Already integrated, non-blocking |
| 3️⃣ | B.1: ✅ Start profiling startup time (flamegraph baseline) | P | DONE | — | `profile.profiling` added, `samply` scripts, startup benchmarks |
| 4️⃣ | ✅ Establish deny.toml policy (licenses, crate bans) | S | DONE | — | Config complete: allows MIT/Apache-2.0, denies GPL/AGPL, bans openssl |

### Next Sprint (Next 1-2 Weeks)

| Priority | Task | Type | Est. Time | Depends | Notes |
| -------- | ---- | ---- | --------- | ------- | ----- |
| 5️⃣ | B.1: ✅ Implement startup optimizations (lazy-loading) | P | DONE | Task 3 | Player created on first play, not at startup |
| 6️⃣ | B.2: Profile scanning speed bottlenecks | P | 2h | — | Identify sync I/O, metadata parsing, DB write costs |
| 7️⃣ | B.2: Implement scanning optimizations (Rayon, batching) | P | 6h | Task 6 | Parallel file reads, batch DB writes |
| 8️⃣ | B.6.3: Add cargo-outdated check (informational) | S | 30m | — | Quarterly dependency health check |
| 9️⃣ | C.1: Batch enrichment improvements (parallel identify) | F | 4h | — | Rate-limit API calls, show progress |

### Following Sprint (2-3 Weeks Out)

| Priority | Task | Type | Est. Time | Depends | Notes |
| -------- | ---- | ---- | --------- | ------- | ----- |
| 🔟 | B.3: Profile audio pipeline (SIMD, resampler) | P | 2h | — | Measure current performance, find micro-optimizations |
| 1️⃣1️⃣ | B.3: SIMD and resampler optimizations | P | 6h | Task 10 | Pre-allocate buffers, optimize hot loops |
| 1️⃣2️⃣ | B.5.2: Collect benchmark baselines (release workflow) | T | 2h | Tasks 5,7,11 | Document before/after improvements |
| 1️⃣3️⃣ | B.6.2: Run cargo-udeps, remove unused deps | S | 1h | — | Reduce attack surface, clean deps |
| 1️⃣4️⃣ | Code coverage setup (rust-tarpaulin baseline) | T | 1h | — | Establish coverage baseline for regression tracking |

### Later (Post v0.2.1)

| Priority | Task | Type | Est. Time | Depends | Notes |
| -------- | ---- | ---- | --------- | ------- | ----- |
| 1️⃣5️⃣ | B.6.4: Fuzzing infrastructure (cargo-fuzz) | S | 4h | — | Optional but high-value for decoder robustness |
| 1️⃣6️⃣ | C.2: UI/UX refinements (smooth transitions, focus) | F | 6h | — | Theme polish, keyboard navigation |
| 1️⃣7️⃣ | C.3: Advanced features (duplicate detection, playlists) | F | 8h | — | Smart playlists, content-hash deduplication |
| 1️⃣8️⃣ | Streaming integration (Spotify recommendations) | F | TBD | — | Vision item, post-v0.3.0 |

---

## Legend

- **Type:** S (Security), T (Testing/Infra), P (Performance), F (Feature)
- **Block:** What needs to happen first
- **Est. Time:** Rough estimate (can vary)

---

### ✅ Phase A: Architecture Refactoring (COMPLETE)

| Phase | Component | Status |
| ------- | ---------- | ------- |
| **A.0** | Workspace setup | ✅ |
| **A.1** | symphonium (audio pipeline) | ✅ |
| **A.2** | soundstore (database) | ✅ |
| **A.3** | discographer (file management) | ✅ |
| **A.4** | musicographer (scanner/watcher) | ✅ |
| **A.5** | music-minder (main app + enrichment) | ✅ |
| **A.6** | UI state handlers | ✅ |
| **A.7** | Repository pattern | ✅ |

**Architecture Achievement:**

- 5 independent crates with clear separation of concerns
- Metadata write functionality fully implemented
- Type-safe conversions between domain models
- 198 tests covering core functionality
- Workspace builds cleanly with zero warnings

### ✅ Completed Phases (1-8.5) + Alternative Matches UI

| Phase | Features |
| ------- | ---------- |
| **1. Foundation** | Rust 2024, Iced 0.13, SQLite, async runtime |
| **2. Scanning** | Recursive scanner (MP3/FLAC/OGG/WAV/M4A), metadata extraction, virtualized library |
| **3. Organization** | Pattern-based file moving, preview, undo support |
| **4. Enrichment** | AcoustID fingerprinting, MusicBrainz lookup, Cover Art Archive |
| **5. CLI** | `scan`, `list`, `identify`, `enrich`, `organize`, `write-tags`, `check`, `watch`, `diagnose` |
| **6. Playback** | Audio playback, visualization (spectrum/waveform/VU), cover art display |
| **7. Library UX** | Search/filter, column sorting, queue management, keyboard shortcuts, file watcher |
| **8. GUI Enrichment** | Batch enrichment pane, progress tracking, write tags button |
| **8.5 Library Gardener** | Quality scoring, verification flags, background maintenance |
| **10. UI Polish** | Theme system, player bar, sidebar, settings pane, enrich pane styling |
| **2.1 Alternative Matches** | Multi-album selection, smart matching by path/metadata, expandable UI |

---

## 🏗️ Phase B: Performance Optimization (Next Priority)

> **Focus:** Profiling, optimization, and enhanced testing coverage.

### B.0 Enhanced Test Coverage (Immediate)

**Current State:** 237 tests across 5 crates (was 198 at Phase A.5)

- Unit tests: DB, organizer, file operations, enrichment APIs ✅
- Integration tests: Scanner, organizer, CLI commands ✅
- Contract tests: MusicBrainz, CoverArt API DTOs ✅
- Mock infrastructure: AcoustID, MusicBrainz, CoverArt ✅
- Concurrent access tests: Multi-threaded database patterns ✅
- End-to-end CLI tests: Full workflow scenarios ✅
- Error recovery tests: Network, filesystem, database failures ✅

**Action Items Completed:**

- [x] Add wiremock for HTTP mocking (AcoustID, MusicBrainz real responses) — 3 tests
- [x] Write end-to-end test scenarios in `tests/e2e.rs` — 9 tests
- [x] Add concurrent access tests in `tests/concurrent_access.rs` — 5 tests
- [x] Add error recovery tests in `tests/error_recovery.rs` — 22 tests
- [x] Add criterion benchmarks in `benches/` subdirectories
- [x] Add benchmark compilation check to CI (Phase B.5.1)
- [x] Profile startup time and library load performance (see B.1)

**Remaining for Phase B.0:**

- Profile with flamegraph and optimize bottlenecks (B.1-B.3)
- Finalize benchmarking baselines for performance tracking (B.5.2-3)

---

### B.1 Startup Performance Optimization

**Current Metrics:**

- GUI startup: ~2ms
- Initial 200 tracks: 14.5ms
- Full library (11.6k tracks): ~133ms

**Target:** <100ms time-to-interactive for 50k+ tracks

**Profiling Infrastructure:** ✅ COMPLETE (January 2026)

- [x] Add `profile.profiling` build profile (release + debug symbols)
- [x] Create profiling script (`scripts/profile-startup.ps1`)
- [x] Add startup benchmarks (`crates/music-minder/benches/startup.rs`)
- [x] Document profiling workflow (`docs/PROFILING.md`)

**Lazy Loading Optimization (January 2026):** ✅ COMPLETE

- [x] Lazy-load player on first play (don't enumerate audio devices at startup)
- [x] Audio device enumeration deferred to background task
- [x] Added `deferred_operations` benchmark group to document savings
- [x] Unit tests for lazy initialization in `ui::state` and `ui::update::db`

**Remaining Optimization Work:**

- [ ] Add incremental database queries (load visible tracks first)
- [ ] Run flamegraph analysis and document bottlenecks

---

### B.2 Scanning Speed Optimization

**Current:** ~200-500 files/second  
**Target:** 1000+ files/second

**Profile Areas:**

- File I/O vs metadata parsing vs database writes
- Parallel metadata extraction (Rayon batching)
- Batch database inserts (500-1000 per transaction)

**Action Items:**

- [ ] Parallel file reads with Rayon
- [ ] Group metadata parsing by 100-track chunks
- [ ] Increase transaction batch size from 50 to 200
- [ ] Add `--fast` flag to skip expensive metadata fields

---

### B.3 Audio Pipeline Optimization

**symphonium crate improvements:**

- [ ] SIMD resampling (Rubato already does this)
- [ ] Ring buffer optimization (reduce allocations)
- [ ] Zero-copy FFT for visualization
- [ ] Pre-allocate decoder buffers for common formats

---

### B.4 Benchmarking Infrastructure ✅ (COMPLETE)

**Status: DONE** — Comprehensive criterion benchmarking framework with realistic workloads

**Completed:**

```bash
# All benchmarks available to run
cargo bench -p symphonium      # Audio calculations
cargo bench -p soundstore      # Database models
cargo bench -p discographer    # Metadata operations
```

**Benchmarks Implemented:**

1. **symphonium/benches/decode.rs** (9 benchmarks)
   - **SIMD Volume Scaling** (Audio Callback Hot Path) — Tests volume scaling at different frame sizes (256, 1024, 4096 samples)
   - **Time Formatting** (UI Rendering) — Measures time calculation overhead for playback display
   - **Ring Buffer Operations** (Audio Thread Communication) — Tests lock-free communication overhead
   - **Why these matter:** Volume scaling runs 48,000 times/second during playback; every nanosecond counts

2. **soundstore/benches/db_insert.rs** (11 benchmarks)
   - **Data Structure Creation** — Artist, Album, Track allocation overhead
   - **Batch Operations** — Creating 100 tracks at once (typical for album scanning)
   - **String Operations** — Clone, format, path building (tag data is string-heavy)
   - **Track Cloning** — Measures overhead for queue operations
   - **Why these matter:** Library scanning creates thousands of objects; allocation pressure impacts overall scan speed

3. **discographer/benches/scan.rs** (11 benchmarks)
   - **Metadata Creation** — Building TrackMetadata from tags (minimal vs. enriched)
   - **Tag Normalization** — Trim, lowercase, parse track numbers (common scanner operations)
   - **Path Operations** — String concatenation and path building
   - **Batch Scanning** — Creating 50 tracks at once (realistic album size)
   - **Why these matter:** Tag parsing is part of the critical scanning path; we measure the baseline for Phase B.2 optimization

**Baseline Metrics Established:**

| Operation | Measurement | Significance |
| --------- | ----------- | ------------ |
| Volume scaling (1024 samples) | ~168 ns | Audio callback hot path |
| Time formatting | ~260 ps | UI rendering (negligible) |
| Track creation (minimal) | ~69 ns | Database model allocation |
| Track creation (enriched) | ~136 ns | With metadata enrichment |
| Metadata creation | ~93-100 ns | Scanner baseline |
| Create 100 tracks | ~15.7 µs | Typical album import |
| Scan 50-track album | ~6.4 µs | Batch operation baseline |

**How to use these baselines:**

```bash
# Run benchmarks and compare to baseline (in target/criterion/)
cargo bench -p symphonium

# Save baseline for regression testing
# (Criterion automatically tracks changes between runs)
```

**Next:** Use benchmarks as regression tests during Phase B.1-B.3 optimizations. Run periodically to catch performance regressions.

---

### B.5.1 Benchmark Compilation Check ✅ (COMPLETE)

**Status: DONE** — Integrated into CI pipeline

**Completed:**

- [x] Add `cargo bench --no-run` step to `.github/workflows/ci.yml`
- [x] Runs on every PR/push to catch compiler errors
- [x] Non-blocking with `continue-on-error: true` (doesn't delay releases)

**What it does:** Compiles all benchmarks on every commit to catch code drift and compilation failures early. Takes ~30 seconds, uses cargo cache.

**How to verify:**

```bash
# Locally, run the same command CI runs
cargo bench --no-run
```

**Next:** Profile startup time and library load performance (B.1)

---

### B.5.2 Benchmark Baseline Collection (AFTER B.1-B.3)

**Status: PLANNED** — Depends on completion of optimization work

**Prerequisite:** Phases B.1-B.3 (startup, scanning, audio pipeline optimizations) must be completed first

**Action Items:**

- [ ] Add benchmark execution to `build-release.yml` workflow
- [ ] Upload results as artifacts for each release
- [ ] Document before/after metrics in release notes
- [ ] Collect 3-5 releases of historical baseline data

**Why after B.1-B.3:** You want to capture the improvement from optimizations; baseline collection proves the wins.

**Timeline:** Start when optimization work is complete

---

### B.5.3 Automated Regression Detection (OPTIONAL, v0.3.0+)

**Status: FUTURE** — Low priority, high complexity

**Prerequisites:**

- [ ] Complete Phase B.5.2 (have 3-5 releases of data)
- [ ] Defined regression thresholds per operation
- [ ] Owner assigned for investigating regressions

**Action Items:**

- [ ] Evaluate Codspeed or custom dashboard integration
- [ ] Set up automated alerting (e.g., >5-10% regression triggers alert)
- [ ] Create runbook for handling unavoidable regressions

**Why optional:** Only valuable once you have patterns, confidence in measurements, and someone to own the process.

**Timeline:** Consider for v0.3.0+ after baseline collection is mature

---

## Phase B.6: Security Hardening (Parallel to B.1-B.5)

> **Focus:** Dependency scanning, code quality, and vulnerability detection.
>
> **Can run in parallel with B.0-B.5** — Orthogonal to performance work
>
> **See:** [SECURITY_TOOLING.md](SECURITY_TOOLING.md) for detailed tool documentation

### B.6.1 Enhanced Dependency Scanning

**Status: ✅ COMPLETE** — January 2026

**Current:** `cargo-audit` (vulnerability scanning) + `cargo-deny` (license/policy enforcement)

**Completed:**

- [x] Add `cargo-deny` for license and policy enforcement
- [x] Create `.cargo/deny.toml` configuration
- [x] Add deny check to CI pipeline (non-blocking, `continue-on-error: true`)
- [x] Document allowed/denied license list

**Policy Highlights:**

- **Allowed licenses:** MIT, Apache-2.0, BSD-2/3-Clause, ISC, MPL-2.0, LGPL (weak copyleft), Zlib, CC0, BSL-1.0
- **Denied licenses:** GPL, AGPL (strong copyleft - implicit deny via allow-list)
- **Banned crates:** openssl, openssl-sys (use rustls instead)
- **Sources:** Only crates.io allowed; git dependencies denied

**Why:** Prevents legal liability and supply-chain attacks

---

### B.6.2 Unused Dependency Detection

**Status: PLANNED** — Medium priority, optional

**Tool:** `cargo-udeps`

**Action Items:**

- [ ] Document `cargo +nightly udeps --all-targets` command
- [ ] Run locally before releases
- [ ] Remove unused dependencies found

**Why:** Reduces attack surface, improves build time

**Note:** Requires nightly Rust (not blocking, run locally)

**Timeline:** After B.6.1

---

### B.6.3 Outdated Dependency Tracking

**Status: PLANNED** — Informational, quarterly

**Tool:** `cargo-outdated`

**Action Items:**

- [ ] Add `cargo outdated` as informational CI check (non-blocking)
- [ ] Establish quarterly update schedule
- [ ] Plan major updates (grouping patches with releases)

**Why:** Proactive security maintenance

**Timeline:** After B.6.1

---

### B.6.4 Fuzzing for Audio Decoder (OPTIONAL)

**Status: FUTURE** — Low priority, high value

**Tool:** `cargo-fuzz`

**Why:** Audio decoding is high-value target for fuzzing

- Catch edge cases in Symphonia decoder
- Test file scanner robustness
- Validate path canonicalization

**Action Items:**

- [ ] Set up cargo-fuzz infrastructure
- [ ] Create fuzz target for audio decoder
- [ ] Create fuzz target for file scanner
- [ ] Run continuously or before releases

**Timeline:** Post-Phase B.1-B.5, if resources permit

---

## Phase C: Feature Polish (After B.0-B.6)

### C.1 Batch Enrichment & Metadata Writing

- [ ] Parallel identify for multiple files (rate-limited to API limits)
- [ ] Batch write tags with progress indicator
- [ ] Smart path analysis for genre/compilation detection
- [ ] MusicBrainz release sorting by release date

### C.2 UI/UX Refinements

- [ ] Queue drag-drop auto-scrolling
- [ ] Smooth theme transitions
- [ ] Keyboard focus indicators
- [ ] Status bar with current operation
- [ ] Toast notifications for async operations

### C.3 Advanced Features

- [ ] Duplicate detection (by content hash, metadata)
- [ ] Smart playlists (rules: genre, year, artist)
- [ ] Gapless playback (pre-buffer next track)
- [ ] ReplayGain normalization
- [ ] Crossfade (0-12s configurable)

## 📋 Backlog

### Audio Features

- Gapless playback (pre-buffer next track)
- 10-band equalizer with presets
- ReplayGain normalization
- Crossfade (0-12s configurable)
- Playlist save/load (.m3u8)

### UI Polish

- Context panel (bulk selection actions)
- Smooth transitions (100-200ms)
- Keyboard navigation focus indicators
- Random startup taglines
- Easter egg theme (classic green Winamp unlock)

### Streaming Integration (Vision)

- Spotify recommendation seeding
- Quality routing (local FLAC vs streaming)
- AI DJ mode

### Maintenance

- Duplicate detection
- Bulk metadata editing
- Album grid layout
- Smart playlists (rule-based)
- Global hotkeys
- Last.fm / ListenBrainz scrobbling

---

## CLI Reference

```bash
# Library management
music-minder scan <path>           # Scan directory for music
music-minder list                  # List all tracks in database
music-minder watch <path>          # Watch directory for changes

# Metadata enrichment
music-minder identify <file>       # Fingerprint + identify track
music-minder enrich <path>         # Batch metadata enrichment
music-minder write-tags <file>     # Write metadata to file tags

# File operations
music-minder organize <path>       # Organize files by pattern
music-minder check [path]          # Check file health status

# Diagnostics
music-minder diagnose              # Audio system diagnostics
```

---

## Design Philosophy

### CLI-First Development

Every feature should work from command line first, then wire GUI as thin layer over the same logic.

### Metadata: File-First, Database for Search

- Audio file is canonical source (metadata read directly from tags)
- Database is fast index for browsing and searching
- Enrichment writes to both file tags and database
- Database rebuilds on rescan (cache, not source of truth)

---

## See Also

- [ADR_ARCHITECTURE_EXTRACTION.md](ADR_ARCHITECTURE_EXTRACTION.md) — Detailed architecture decisions
- [STATUS.md](STATUS.md) — Current implementation status
- [ARCHITECTURE.md](ARCHITECTURE.md) — Design patterns and module organization
