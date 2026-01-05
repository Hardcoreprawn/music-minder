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

**198 tests passing** | **0 clippy warnings** | **5-crate modular architecture**

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

## 🚧 Next Sprint: v0.2.0 (Performance & Polish)

> **Strategy:** Now that architecture is solid, focus on performance optimizations, enhanced testing, and feature polish. Build confidence for 0.3.0 with streaming integration.

---

## 🏗️ Phase B: Performance Optimization (Next Priority)

> **Focus:** Profiling, optimization, and enhanced testing coverage.

### B.0 Enhanced Test Coverage (Immediate)

**Current State:** 198 tests across 5 crates
- Unit tests: DB, organizer, file operations, enrichment APIs ✅
- Integration tests: Scanner, organizer, CLI commands ✅
- Contract tests: MusicBrainz, CoverArt API DTOs ✅
- Mock infrastructure: AcoustID, MusicBrainz, CoverArt ✅

**Gaps to Address:**

1. **End-to-end CLI tests** — Test full workflows (scan → identify → organize)
2. **Concurrent access tests** — Multi-threaded database access patterns
3. **Error recovery tests** — Network failures, corrupted files, permission errors
4. **Performance benchmarks** — Scanning speed, decode throughput, API latency

**Action Items:**
- [ ] Add wiremock for HTTP mocking (AcoustID, MusicBrainz real responses)
- [ ] Write end-to-end test scenarios in `tests/e2e/`
- [ ] Add criterion benchmarks in `benches/` subdirectories
- [ ] Profile startup time and library load performance

---

### B.1 Startup Performance Optimization

**Current Metrics:**
- GUI startup: ~2ms
- Initial 200 tracks: 14.5ms
- Full library (11.6k tracks): ~133ms

**Target:** <100ms time-to-interactive for 50k+ tracks

**Action Items:**

- [ ] Profile with `cargo flamegraph` (add `flamegraph` dev dependency)
- [ ] Lazy-load player on first play (don't enumerate audio devices at startup)
- [ ] Add incremental database queries (load visible tracks first)
- [ ] Benchmark against baseline with `criterion`

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

**Status: DONE** — Criterion benchmarking framework established

**Completed:**

```bash
# All benchmarks available to run
cargo bench -p symphonium      # Audio calculations
cargo bench -p soundstore      # Database models
cargo bench -p discographer    # Metadata operations
```

**Benchmarks Implemented:**

1. **symphonium/benches/decode.rs** (3 benchmarks)
   - Time calculation for 0s, 180s, 3600s
   - Foundation for audio pipeline optimization

2. **soundstore/benches/db_insert.rs** (3 benchmarks)
   - Artist creation
   - Album creation  
   - Track creation
   - Foundation for database performance tracking

3. **discographer/benches/scan.rs** (1 benchmark)
   - Metadata structure creation
   - Foundation for scanning optimization

**HTML Reports:** Generated automatically with `--plotting-backend gnuplot`

**Next:** Use benchmarks as baseline for B.0-B.3 optimizations

---

## Phase C: Feature Polish (After B.0-B.4)

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
