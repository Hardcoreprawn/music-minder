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

## 🚧 Current Sprint: Polish Release (v0.2.0)

### Priority 1: User Feedback (Quick Wins)

These are missing UX essentials that make the app feel unfinished.

#### 1.1 Toast Notifications ✅

Non-blocking feedback for async operations.

- [x] Toast component (horizontal bar at bottom, auto-dismiss after 4s)
- [x] Success states: "Tags written", "Scan complete", "Organized X files"
- [x] Error states: "Failed to write tags"
- [x] Warning states: "Scan stopped", "Low confidence matches"
- [x] Info states: Batch enrichment results

**Files:** `src/ui/views/toast.rs`, `src/ui/state.rs`, `src/ui/mod.rs`, `src/ui/update/*.rs`

#### 1.2 Empty States ✅

Helpful messages when lists are empty.

- [x] Library: "No tracks in library. Scan a folder to get started!"
- [x] Library search: "No results for '{query}'"
- [x] Queue: "Queue is empty — add tracks from the Library"
- [x] Enrich selection: "No tracks selected. Add tracks from the Library"

**Files:** `src/ui/views/library/track_list.rs`, `src/ui/views/layout.rs`, `src/ui/views/enrich/selection.rs`

#### 1.3 Loading States ✅

Visual feedback during async operations with fun, personality-filled messages.

- [x] Loading module with `LoadingContext` enum (Library, Scanning, Identifying, etc.)
- [x] Serious messages: "Loading library...", "Generating audio fingerprint..."
- [x] Silly messages (SimCity/Winamp inspired): "Reticulating splines...", "Whipping the llama's ass..."
- [x] Message rotation: alternates serious/silly every ~3 seconds
- [x] Spinner animation with context-appropriate icons
- [x] Library loading state with rotating messages
- [x] Scan progress with fun messages + file count
- [x] Enrich progress with fun messages

**Files:** `src/ui/views/loading.rs` (new), `src/ui/views/library/track_list.rs`, `src/ui/views/library/mod.rs`, `src/ui/views/enrich/mod.rs`

---

### Priority 2: Alternative Matches UI ✅

**COMPLETED** - When enrichment returns multiple album matches, users can now review and select the correct version.

#### Implementation Summary

**Data Model** (`src/ui/state.rs`):

- `AlternativeMatch` struct with album, year, confidence, release_type, and full identification
- Extended `EnrichmentResult` with `alternatives`, `show_alternatives`, and `selected_alternative` fields

**Service Layer** (`src/enrichment/service.rs`):

- New `identify_track_with_alternatives()` method that:
  - Scores all matches using smart path/metadata hints
  - Returns best + 2-3 alternatives per session
  - Enriches all with MusicBrainz (respecting rate limits)

**State Management** (`src/ui/update/enrichment.rs`):

- New message `EnrichBatchIdentifyWithAlts` carries best + alternatives
- `EnrichToggleAlternatives(idx)` - expand/collapse alternatives list
- `EnrichSelectAlternative(result_idx, alt_idx)` - switch to different album
- Smart selection: auto-picks best match, considers folder names and file metadata

**UI Component** (`src/ui/views/enrich/results.rs`):

- Review button shows "▼" when alternatives exist, "▲" when expanded
- Click to expand shows alternatives with confidence scores (color-coded)
- "Select" button on each alternative switches the identification
- Smooth expand/collapse with nested panel styling

#### User Flow

```text
Identify → Best match + alternatives displayed
         → Click "Review ▼" → Alternatives expand
         → Click "Select" on different album → Updates display
         → Click "Write" → Writes selected version's metadata
```

#### Smart Matching Logic

- Boosts matches where album name appears in folder path
- Boosts matches aligning with existing file metadata
- Penalizes undesirable types (karaoke, remixes) unless expected
- Prioritizes original studio albums

#### Session-Only Storage

- Alternatives generated only during batch identification
- Kept to 2-3 best options
- Discarded when batch finishes (no persistence)
- Recreated fresh on next identify cycle

**Files:** `src/ui/state.rs`, `src/enrichment/service.rs`, `src/ui/views/enrich/results.rs`, `src/ui/update/enrichment.rs`, `src/ui/messages.rs`, `src/ui/mod.rs`

**Tests:** 207 tests passing, all enrichment tests validated

---

### Priority 3: Startup Performance ⚡ Phase 1-2 Complete

"Instant startup — ready before you blink" is core to the Winamp DNA. Current startup is sluggish.

#### Phase 1: Deferred Initialization & Instrumentation ✅ COMPLETE

- [x] Add detailed timing instrumentation to startup path
- [x] Defer audio device enumeration to background task
- [x] Parallelize DB, diagnostics, and device enumeration tasks
- [x] Add tracing to database and track loading operations
- [x] Compile and validate changes

See [STARTUP_OPTIMIZATION_PHASE_1.md](STARTUP_OPTIMIZATION_PHASE_1.md) for implementation details.

#### Phase 2: Progressive Library Loading ✅ COMPLETE

- [x] Add `get_tracks_paginated(limit, offset)` database function
- [x] Add `count_tracks()` to determine total library size
- [x] Modify UI to load first batch of tracks (200) immediately
- [x] Load remaining tracks in background after initial batch
- [x] Test with large libraries (11k+ tracks verified)

**Results (11,638 track library):**

- Initial 200 tracks loaded in **14.5ms** (UI responsive immediately)
- Remaining 11,438 tracks loaded in **118.3ms** (background)
- Total time: ~133ms vs previous ~58ms for all-at-once

**Implementation:**

- New messages: `TracksLoadedInitial`, `TracksLoadedMore`
- New state field: `tracks_total` for progress tracking
- Progressive loading tasks: `load_tracks_initial_task`, `load_tracks_remaining_task`

#### Phase 3: Further Optimization (Future)

- [ ] Profile startup with `cargo build --timings` and runtime tracing
- [ ] Lazy player initialization (defer audio until first play)
- [ ] Demand-based loading (load more on scroll for very large libraries)
- [ ] Measure time-to-first-paint vs time-to-interactive

#### Current Performance Metrics

| Metric | Before | After | Notes |
| -------- | ------- | ------ | ------- |
| Startup to GUI ready | ~2-3s | ~2ms | Time to `application()` call |
| Database init | ~3.5ms | ~3.5ms | Already fast |
| Initial tracks visible | ~58ms | ~14.5ms | 200 tracks immediately |
| Full library loaded | ~58ms | ~133ms | Split across 2 loads |
| UI responsive | After full load | After 14.5ms | User can interact immediately |

#### Likely Remaining Culprits

| Area | Issue | Status |
| ------ | ------- | -------- |
| Audio device enumeration | CPAL enumerates synchronously in Player::new() | Could defer player init |
| Iced window creation | ~1s between main() and first paint | Framework limitation |
| Font/theme loading | Iced resource loading | Embedded fonts help |

**Files:** `src/main.rs`, `src/ui/mod.rs`, `src/ui/update/mod.rs`, `src/ui/update/db.rs`, `src/ui/messages.rs`, `src/ui/state.rs`, `src/db/mod.rs`

---

### Priority 4: Async & Throughput Optimization

Make scanning and external APIs as fast as possible through proper concurrency.

#### 4.1 Scanning Speed

Current: Sequential file processing. Goal: Maximize disk throughput.

| Task | Description |
| ---- | ----------- |
| [ ] Profile current scanner | Measure time in I/O vs metadata parsing vs DB writes |
| [ ] Parallel file discovery | Use `rayon` or `tokio::spawn_blocking` for directory walks |
| [ ] Batch DB inserts | Collect 50-100 tracks, single transaction instead of per-file |
| [ ] Streaming metadata reads | Start processing files before walk completes |
| [ ] Progress granularity | Report files/second, show ETA |

**Current architecture:** `scanner/mod.rs` uses `walkdir` synchronously, sends batches via channel.

**Target:** 1000+ files/second on SSD (currently ~200-500 depending on metadata complexity).

#### 4.2 External API Throughput

AcoustID, MusicBrainz, and CoverArt Archive all have rate limits. Maximize throughput within limits.

| API | Rate Limit | Current | Optimization |
| --- | ---------- | ------- | ------------ |
| AcoustID | 3 req/s | Sequential, 500ms delay | Pipeline: fingerprint while waiting for previous response |
| MusicBrainz | 1 req/s | On-demand | Cache responses, batch lookups where possible |
| CoverArt Archive | 1 req/s | On-demand | Pre-fetch during enrichment, aggressive caching |

**Tasks:**

- [ ] Pipeline fingerprinting with lookups (fingerprint track N+1 while waiting for API response for track N)
- [ ] Connection pooling for HTTP client (reuse TCP connections)
- [ ] Request coalescing: batch MusicBrainz lookups by release group
- [ ] Smarter retry: exponential backoff, circuit breaker for API outages
- [ ] Offline mode: queue requests when API unavailable, process when back

#### 4.3 Async Best Practices Audit

Ensure we're not blocking the async runtime anywhere.

| Area | Check |
| ---- | ----- |
| [ ] File I/O | All file reads via `spawn_blocking`, never in async context |
| [ ] Metadata parsing | `lofty`/`symphonia` calls wrapped in `spawn_blocking` |
| [ ] Database | All SQLx queries are truly async (no blocking calls) |
| [ ] UI updates | Heavy computations don't block the render loop |
| [ ] Thread pool sizing | `spawn_blocking` pool sized appropriately for workload |

**Files:** `src/scanner/mod.rs`, `src/enrichment/service.rs`, `src/metadata/mod.rs`

---

### Priority 5: Queue Drag-Drop Polish

Keyboard reordering works (Alt+↑/↓). Drag-drop needs finishing touches:

- [x] Keyboard reordering (Alt+↑/↓)
- [x] Drag handle UI (grip icon)
- [x] Basic drag state + drop target calculation
- [x] Visual feedback (dimming, cursor)
- [ ] Auto-scroll at edges during drag
- [ ] Cancel drag on focus loss / Escape / right-click

**Files:** `src/ui/views/player.rs`, `src/ui/update/selection.rs`, `src/ui/streams.rs`

---

## 📋 Backlog

### Phase 9: Audio Features

| Feature | Complexity | Notes |
| --------- | ------------ | ------- |
| Gapless playback | Medium | Pre-buffer next track, seamless transition |
| Equalizer (10-band) | Medium | Rock/Pop/Jazz/Classical presets |
| ReplayGain | Medium | Volume normalization scanning |
| Crossfade | Medium | 0-12s configurable transitions |
| Playlist save/load | Low | .m3u8 format support |

### Phase 10: Remaining UI Polish

| Feature | Complexity | Notes |
| ------- | ---------- | ----- |
| Context panel (slide-in) | Medium | Bulk selection actions, before/after preview |
| Smooth transitions | Low | 100-200ms for state changes |
| Focus indicators | Low | Keyboard navigation support |
| Startup tagline | Low | Random "It really whips..." messages |
| Easter egg theme | Low | Hidden classic green Winamp unlock |

### Phase 11: Streaming Integration (Future Vision)

Bridge your local library with streaming services for discovery:

- **Taste analysis**: Genre/mood/era distribution from your collection
- **Spotify recommendations**: Seeded by your actual music taste, not just streaming history
- **Quality routing**: Always play the best available version (local FLAC vs streaming)
- **AI DJ**: Playlists built from "70% owned, 30% discovery"

### Library Features (Backlog)

- Duplicate detection
- Bulk metadata editing
- Album view with grid layout
- Artist/Album grouping (collapsible sections)
- Smart playlists (rule-based auto-generation)

### Integration (Backlog)

- Global hotkeys (platform-specific implementation)
- Lyrics display (external API integration)
- Last.fm / ListenBrainz scrobbling
- Discord Rich Presence

---

## 🔧 Technical Debt

### Medium Priority

| Item | Notes |
| ---- | ----- |
| Atomic writes for cover art | `embed_cover_art()` and sidecar writes need atomic write-swap pattern |
| Retry mechanism | Exponential backoff for file locks, network timeouts |
| Cleanup stale temps | Remove orphaned `.tmp` files on startup |

### Low Priority

| Item | Notes |
| ----- | ----- |
| Watcher refactor | Migrate from Iced subscription to init-time start pattern |
| Duration nullability | Make `NOT NULL DEFAULT 0` (currently `Option<i64>`) |
| Dead code audit | 19 `#[allow(dead_code)]` annotations, mostly intentional |

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
music-minder check-tools           # Verify fpcalc is installed
music-minder diagnose              # Audio system diagnostics
```

**Common flags:** `--dry-run`, `--verbose`, `--json`, `--quiet`

---

## Design Philosophy

### CLI-First Development

Every feature should be testable from the command line:

1. Build the feature as a library function
2. Expose via CLI with `--verbose`/`--json` flags
3. Add tracing at key decision points
4. Wire GUI as thin layer over the same logic

### Metadata: File-First, DB for Library

**The audio file is the source of truth.** The database is an index for fast browsing:

- Playback reads metadata directly from file tags
- Enrichment writes to file tags, not just database
- Database rebuilds on rescan (cache, not canonical store)

### Tracing Targets

```powershell
$env:RUST_LOG="player::events=debug,ui::commands=debug"
cargo run --release
```

| Target | Purpose |
| ------ | ------- |
| `player::events` | Audio thread events |
| `ui::commands` | UI command dispatch |
| `scanner::progress` | Scan progress |
| `enrichment::api` | External API calls |
| `cover::resolver` | Cover art resolution |

---

## Archive

Detailed implementation notes for completed phases have been archived to [ROADMAP_OLD.md](ROADMAP_OLD.md).
