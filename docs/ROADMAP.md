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

## Current Status: v0.1.7

**207 tests passing** | **0 clippy warnings** | **Alternative Matches UI complete**

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

## 🚧 Next Sprint: v0.2.0 (Architecture & Performance)

> **Strategy:** Extract reusable components into a monorepo workspace while maintaining v0.1 functionality. This unblocks performance optimizations and enables new architectures (CLI tools, servers, headless player).

---

## 🏗️ Phase A: Architecture Refactoring (Current Priority)

See [ADR_ARCHITECTURE_EXTRACTION.md](ADR_ARCHITECTURE_EXTRACTION.md) for detailed decisions.

**Testing-First Approach:** For each extraction task:

1. **Write CLI tests first** — Verify current behavior via CLI commands (`music-minder scan`, `music-minder identify`, etc.)
2. **Add tracing** — Enable `RUST_LOG=debug` to see what's happening
3. **Extract code** — Move to new crate with minimal behavior change
4. **Verify tests pass** — Same CLI behavior + tests should still pass
5. **Add crate-level tests** — Test extracted crate independently without GUI/DB

This ensures we have a safety net and can incrementally refactor without breaking functionality.

### A.1 Monorepo Workspace Setup (Week 1)

- [ ] Create root `Cargo.toml` with workspace definition
- [ ] Move existing code to `crates/music-minder/`
- [ ] Create skeleton crates: `symphonium/`, `discography/`, `librarian/`, `soundstore/`
- [ ] Verify all existing tests pass in workspace context

**Deliverable:** Workspace that compiles and tests identically to v0.1.7

**Files to create:**

```text
Cargo.toml (workspace)
crates/
  symphonium/Cargo.toml
  discography/Cargo.toml
  librarian/Cargo.toml
  soundstore/Cargo.toml
  music-minder/Cargo.toml (moved from root)
```

---

### A.2 Extract Audio Pipeline as `symphonium` (Week 2)

**Testing approach:**

1. [ ] Write CLI integration test: `music-minder play <file>` produces audio output
2. [ ] Add tracing for decode/resample/output pipeline
3. [ ] Extract audio code to symphonium crate
4. [ ] Verify CLI test still passes with new structure
5. [ ] Add symphonium unit tests for Decoder, Resampler, AudioPipeline

**Scope:** ~2000 LOC from `src/player/{audio.rs, decoder.rs, resampler.rs, state.rs, simd.rs}`

**What to extract:**

- `AudioSharedState` — Lock-free atomic playback state
- `Decoder` — Symphonia-based format detection and decoding
- `Resampler` — Rubato sample rate conversion
- `AudioCallback` — CPAL stream callback and ring buffer consumer
- `PlayerEvent` and `PlayerCommand` — Communication protocol
- `AudioPipeline` — Orchestrates full decode→resample→output flow

**What stays in music-minder:**

- Queue management (application layer)
- Visualization binding (UI layer)
- Media controls (OS integration)

**Tests:** All existing audio tests pass in symphonium crate; no behavior changes

**Benefits:** Can be published standalone; enables headless audio server; performance auditing without UI overhead

---

### A.3 Create Database Schema Crate `soundstore` (Week 2)

**Testing approach:**

1. [ ] Write CLI test: `music-minder list` returns correct tracks
2. [ ] Add tracing for database queries
3. [ ] Extract database code to soundstore crate
4. [ ] Verify CLI test still passes
5. [ ] Add soundstore unit tests for repository traits

**Scope:** ~1000 LOC from `src/db/`, `src/model/`

**What to extract:**

- Database initialization and migrations (`migrations/`)
- Entity models: `Track`, `Artist`, `Album`, `TrackHealth`
- Repository trait definitions
- SQLite implementation of repositories

**New trait:**

```rust
pub trait TrackRepository: Send + Sync {
    async fn get_track(&self, id: i64) -> Result<Track>;
    async fn get_tracks_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Track>>;
    async fn insert_track(&self, track: Track) -> Result<()>;
}
```

**Benefits:** Can evolve schema independently; enables PostgreSQL support later; database testable in isolation

---

### A.4 Extract File Management as `discographer` (Week 3)

**Testing approach:**

1. [ ] Write CLI test: `music-minder scan <path>` produces correct library
2. [ ] Write CLI test: `music-minder organize --dry-run` shows correct operations
3. [ ] Add tracing for scanner and organizer
4. [ ] Extract file management code to discographer crate
5. [ ] Verify CLI tests still pass
6. [ ] Add discographer unit tests for Scanner, Organizer, MetadataReader

**Scope:** ~1500 LOC from `src/scanner/`, `src/organizer/`, `src/metadata/`, `src/library/`

**What to extract:**

- File discovery via `walkdir`
- Metadata reading via `lofty`
- File organization and pattern matching
- Path handling with **`camino::Utf8Path`** (enforced UTF-8)

**New trait:**

```rust
pub trait Scanner {
    fn scan(path: &Utf8Path) -> Result<impl Iterator<Item = TrackMetadata>>;
}
```

**Ecosystem adoption:** Add `camino = "1.1"` for UTF-8 paths throughout

**Benefits:** CLI tools can scan/organize without database; UTF-8 type safety; reusable in backup tools

---

### A.5 Extract Enrichment Services as `music_journo` (Week 4)

**Testing approach:**

1. [ ] Write CLI test: `music-minder identify <file>` returns correct identification
2. [ ] Write CLI test: `music-minder enrich --dry-run <path>` shows enrichment results
3. [ ] Add tracing for fingerprinting, API calls, matching
4. [ ] Extract enrichment code to music_journo crate
5. [ ] Verify CLI tests still pass
6. [ ] Add music_journo unit tests for Fingerprinter, Identifier, EnrichmentService

**Scope:** ~2500 LOC from `src/enrichment/`, `src/cover/`

**What to extract:**

- Fingerprinting coordination (`fingerprint::Service`)
- Identification service (`identification::Service` for AcoustID + smart matching)
- Enrichment pipeline (`EnrichmentService` for MusicBrainz + Cover Art)
- API clients with rate limiting and caching

**New trait:**

```rust
pub trait EnrichmentService {
    async fn identify(&self, file: &Path) -> Result<TrackIdentification>;
    async fn enrich(&self, track: &Track) -> Result<EnrichedMetadata>;
}
```

**Benefits:** Standalone metadata server; batch enrichment tools; testable without GUI/DB coupling

---

### A.6 Refactor UI State Handlers (Week 5)

**Testing approach:**

1. [ ] Write UI integration test for each domain (library scan, player control, enrichment, organize)
2. [ ] Add tracing for message dispatch and state transitions
3. [ ] Split update/ into focused modules
4. [ ] Verify all UI tests still pass
5. [ ] Add unit tests for each update module's state transitions

**Scope:** Reorganize `src/ui/update/` into focused modules

**Current:** Single `mod.rs` handles all message types (mixing concerns)

**Target:**

```text
update/
├── mod.rs       # Router
├── library.rs   # Scan, Watcher, Track loading
├── player.rs    # Play/Pause, Seek, Queue management
├── enrichment.rs # Identify, Enrich, Write tags
└── organizer.rs # Organize, Preview, Execute
```

**Benefit:** Each module handles one domain's state transitions; easier to test; scales as new commands added

---

### A.7 Add Repository Pattern to Main App (Week 5)

**Testing approach:**

1. [ ] Write unit test mocking TrackRepository
2. [ ] Verify UI/enrichment tests still pass with mock repo
3. [ ] Replace direct SQLx calls with repository interface
4. [ ] Verify all tests still pass

**Scope:** Replace direct SQLx calls with repository interface

**Current:** UI code calls `sqlx::query!()` directly

**Target:**

```rust
let repo = SqliteTrackRepository::new(pool);
let track = repo.get_track(id).await?;
let all = repo.get_all_tracks_paginated(200, 0).await?;
```

**Benefit:** Type-safe queries; can add caching layer later; easier to test

---

### A.8 Benchmarking Infrastructure (Week 4)

**Ecosystem adoption:** Add `criterion` for performance tracking

**Benchmarks to add:**

- `benches/decode.rs` — MP3/FLAC decode speed for typical files
- `benches/resample.rs` — Sample rate conversion throughput
- `benches/fft.rs` — Spectrum visualization FFT performance

**Usage:**

```bash
cargo bench -p symphonium
cargo bench -p symphonium -- --compare  # Compare to baseline
```

**Benefit:** Track performance regressions across releases; publish benchmarks with crate

---

## Phase B: Performance Optimization 🚀 (After A.1-A.8)

### B.1 Startup Performance (Parallel with Phase A)

- [ ] Profile with `cargo build --timings` and runtime tracing
- [ ] Lazy player initialization (defer audio device enumeration until first play)
- [ ] Measure time-to-first-paint vs time-to-interactive

**Current metrics (Phase 2 complete):**

- Startup to GUI: ~2ms
- Initial 200 tracks: 14.5ms
- Full library load: ~133ms for 11.6k tracks

**Target:** <100ms time-to-interactive even for 50k+ track libraries

---

### B.2 Scanning Speed (After A.4)

**Current:** ~200-500 files/second  
**Target:** 1000+ files/second on SSD

- [ ] Profile I/O vs metadata parsing vs DB writes
- [ ] Parallel metadata extraction via Rayon
- [ ] Batch database inserts (50-100 tracks per transaction)

---

### B.3 API Throughput (After A.5)

**Current rate limits:**

- AcoustID: 3 req/s
- MusicBrainz: 1 req/s
- CoverArt: 1 req/s

**Target:** Pipelined requests (fingerprint N+1 while fetching API response for N)

---

## Phase C: Feature Polish (Parallel with Phase B)

### C.1 Queue Drag-Drop Finishing Touches

- [ ] Auto-scroll at drag edges
- [ ] Cancel drag on Escape / focus loss
- [ ] Smooth visual feedback

**Current:** Keyboard reordering (Alt+↑/↓) works; drag handle UI exists

---

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
