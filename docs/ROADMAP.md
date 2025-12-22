# Music Minder Roadmap

## 🎯 Vision: Winamp for the Modern Era

**Music Minder is a love letter to Winamp** — the legendary audio player that defined a generation. We're building a native, fast, beautiful music player that captures that early-2000s magic while leveraging modern Rust for rock-solid performance.

### Core Principles

1. **Audio First**: Playback is sacred. Nothing interrupts the music.
2. **It Just Works**: Scan a folder, press play. No cloud accounts, no subscriptions.
3. **Retro Soul, Modern Tech**: Winamp's spirit with 2024's engineering.
4. **Native & Fast**: No Electron. No web views. Pure Rust performance.
5. **Learning Project**: A playground for exploring Rust, audio, and UI.
6. **CLI-First, GUI-Second**: Every feature works from the command line first.

### The Winamp DNA

What made Winamp special:

- **Instant startup** — Ready before you blink
- **Tiny footprint** — Runs on anything
- **Visualization** — Mesmerizing spectrum analyzers
- **Skins** — Express yourself (future goal)
- **Global hotkeys** — Control from anywhere
- **"It really whips the llama's ass"** — Personality and fun

---

## Design Philosophy

### CLI-First Development

**Every feature should be testable from the command line.** This enables:

- **AI-assisted development**: Copilot/agents can run commands, see outputs, iterate
- **Debuggability**: Isolate issues without GUI complexity
- **Composability**: Chain commands, script workflows, automate testing
- **Separation of concerns**: Core logic is decoupled from UI

**Pattern:**

```text
1. Build the feature as a library function
2. Expose it via CLI command with --verbose/--json flags
3. Add tracing/logging at key decision points
4. Wire up GUI as a thin layer over the same logic
```

**CLI conventions:**

- `--dry-run` / `--preview`: Show what would happen without doing it
- `--verbose` / `-v`: Enable debug logging
- `--json`: Machine-readable output for scripting/AI parsing
- `--quiet` / `-q`: Suppress non-essential output
- Exit codes: 0 = success, 1 = error, 2 = partial success

**Existing CLI commands:**

```bash
music-minder scan <path>           # Scan directory for music
music-minder list                  # List all tracks in database
music-minder identify <file>       # Fingerprint + identify track
music-minder enrich <path>         # Batch metadata enrichment
music-minder organize <path>       # Organize files by pattern
music-minder write-tags <file>     # Write metadata to file tags
music-minder check [path]          # Check file health status
music-minder check-tools           # Verify fpcalc is installed
music-minder diagnose              # Audio system diagnostics (--json)
music-minder watch <path>          # Watch directory for changes (-v, --db, --scan-first)
```

**Planned CLI commands:**

```bash
# Playback control (headless player daemon)
music-minder play <file|path>      # Play a file or folder
music-minder pause                 # Pause playback
music-minder next                  # Skip to next track
music-minder prev                  # Go to previous track
music-minder seek <position>       # Seek to position (0.0-1.0 or mm:ss)
music-minder volume <level>        # Set volume (0-100)
music-minder status                # Show player state (--json for scripting)

# Queue management
music-minder queue                 # Show current queue
music-minder queue add <file>      # Add to queue
music-minder queue clear           # Clear queue
music-minder queue shuffle         # Shuffle queue

# Library queries
music-minder search <query>        # Search library (artist/album/title)
music-minder info <file>           # Show file metadata (--json)
music-minder stats                 # Library statistics
```

### Tracing & Observability

**Use `tracing` crate with structured logging:**

```rust
// Good: Structured, filterable, includes context
tracing::debug!(target: "player::events", event = ?event, "Received player event");
tracing::info!(target: "scanner::progress", count = 100, "Scanned 100 tracks");

// Bad: Unstructured, hard to filter
println!("Received event: {:?}", event);
```

**Log target naming convention:**

```text
module::subsystem
  ├── player::events      # Audio thread events
  ├── player::decoder     # Decode operations  
  ├── ui::commands        # UI command dispatch
  ├── ui::events          # UI event handling
  ├── scanner::progress   # Scan progress
  ├── scanner::files      # File discovery
  ├── enrichment::api     # External API calls
  ├── enrichment::match   # Match scoring
  ├── cover::resolver     # Cover art resolution
  └── health::db          # Health record updates
```

**When adding a new feature:**

1. Choose a log target following the `module::subsystem` pattern
2. Add `debug!` for internal state changes
3. Add `info!` for significant operations (file processed, API called)
4. Add `warn!` for recoverable issues
5. Add `error!` for failures (but prefer `Result` over panics)

**Run with specific log targets:**

```powershell
$env:RUST_LOG="player::events=debug,ui::commands=debug"
cargo run --release
```

### Metadata: File-First, DB for Library

**The audio file is the source of truth for metadata.** The database serves as an index for fast library browsing, but:

- Playback reads metadata directly from file tags (via `lofty`)
- Enrichment writes to file tags, not just the database
- Database is rebuilt on rescan (cache, not canonical store)
- This ensures metadata travels with the files

---

## Completed Phases

### Phase 1: Foundation ✅

- [x] Project setup with Rust 2024 edition
- [x] Dependencies: Iced 0.13, Tokio, SQLx, Lofty, Clap
- [x] SQLite database with migrations
- [x] Optimized build configuration

### Phase 2: Scanning & Library ✅

- [x] Recursive directory scanner (MP3, FLAC, OGG, WAV, M4A)
- [x] Metadata extraction (Artist, Album, Title, Track#, Duration)
- [x] Virtualized library view (10k+ tracks)
- [x] Live scan progress updates

### Phase 3: Organization ✅

- [x] Pattern-based file organization
- [x] Streaming preview with parallel file checks
- [x] Virtualized preview list
- [x] Undo support with JSON persistence
- [x] Batch database updates

### Phase 4: Enrichment ✅

- [x] **AcoustID Integration**: Audio fingerprinting via fpcalc + API lookup
- [x] **Smart Matching**: Prefers correct album based on path/metadata hints
- [x] **MusicBrainz Lookup**: Fetch detailed metadata by recording ID
- [x] **Cover Art**: Download from Cover Art Archive
- [x] **Enrichment Service**: High-level orchestration with rate limiting

### Phase 5: CLI Integration ✅

- [x] **CLI `identify` command**: Single file identification with smart matching
- [x] **CLI `write-tags` command**: Write metadata to files with preview mode
- [x] **Metadata Writing**: `--write` and `--fill-only` flags on identify
- [x] **CLI `enrich` command**: Batch enrichment with health tracking, dry-run, recursive scan

### Phase 6: Playback & UX ✅

- [x] **Audio playback**: Play tracks from library
- [x] **Now Playing view**: Track info, progress bar, queue display
- [x] **Cover art resolution**: Embedded, sidecar, cached, remote (non-blocking)
- [x] **Cover art display**: Album art in Now Playing view (200x200 with source indicator)
- [x] **Visualization modes**: Spectrum, Waveform, VU Meter

### Phase 7: System Integration ✅

- [x] **OS media controls**: Windows SMTC / Linux MPRIS / macOS via `souvlaki` crate
- [x] **Refactored playback architecture**: Single command path, event-driven state

---

## 🎯 Current Phase: Library UX & Queue Management

This phase focuses on making the library actually usable for large collections.

### 7.1 Smart Background Scanning ⚠️

Never interrupt playback. Keep the library fresh automatically.

- [x] **Watch directories**: Use `notify` crate to detect file changes
- [x] **Incremental updates**: Only rescan changed/new files (mtime-based)
- [x] **Background thread**: Watcher runs on dedicated thread, events via channel
- [x] **Startup scan**: Watch paths auto-start on app launch
- [x] **Scan indicator**: Subtle "● Watching" / "⟳ Syncing" in sidebar
- [x] **Never interrupt audio**: File changes queued, processed in batches
- [x] **CLI command**: `music-minder watch <path> -v --db <db> --scan-first`
- [x] **Manual refresh**: Button to force full rescan if needed
- [x] **Re-architect watcher subscription**: Migrated to `tokio::sync::mpsc` with async `.recv().await` to avoid blocking the runtime

### 7.2 Library Search & Filter (High Priority)

- [x] **Search bar**: Filter tracks by typing (searches title, artist, album)
- [x] **Instant filtering**: Results update as you type (no Enter needed)
- [x] **Column sorting**: Click column headers to sort (Artist, Album, Title, Duration)
- [x] **Sort indicator**: Visual arrow showing sort direction
- [x] **Filter chips**: Quick filters for format (FLAC/MP3), lossless, etc.

### 7.3 Queue Management (High Priority) ✅

- [x] **Queue panel**: Visible queue in Now Playing view (scrollable list)
- [x] **Current track highlight**: Visual indicator of what's playing
- [x] **Click to jump**: Click any queued track to play it immediately
- [x] **Remove from queue**: X button to remove tracks
- [ ] **Reorder queue**: Drag-and-drop to rearrange (deferred - needs custom widget)
- [x] **Clear queue**: Button to clear entire queue
- [x] **Repeat modes**: Off / All / One with visual toggle
- [x] **Shuffle toggle**: Shuffle on/off button
- [x] **Play next**: Right-click → "Play Next" (add_next exists)

### 7.4 Keyboard Shortcuts (Medium Priority) ✅

Winamp's global hotkeys were legendary. Start with in-app, then go global.

- [x] **Space**: Play/pause toggle
- [x] **←/→**: Previous/next track
- [x] **Shift+←/→**: Seek backward/forward 5s
- [x] **↑/↓**: Navigate selection (Alt+↑/↓ for volume)
- [x] **Ctrl+F**: Focus search box (clears search)
- [x] **Enter**: Play selected track
- [x] **Delete**: Remove selected from queue
- [x] **Escape**: Clear search / close panels
- [ ] **Global hotkeys** (future): Control playback from any app

### 7.5 Now Playing Enhancements (Medium Priority) ✅

- [x] **Queue count display**: "Track 3 of 25" indicator
- [x] **Track info panel**: Format, bitrate, file path display
- [x] **Read metadata from file**: Decoder reads tags and sends via `TrackLoaded` event
- [x] **Metadata fallback chain**: DB → file tags → filename (via `current_track_display()`)

### 7.6 Code Cleanup (Low Priority)

Remove or wire up unused code identified in review:

- [x] Wire up `PlayQueue::cycle_repeat()` to UI button
- [x] Wire up `PlayQueue::set_shuffle()` to UI toggle
- [x] Wire up `PlayQueue::remove()` to queue panel
- [ ] Wire up `PlayQueue::reorder()` to queue panel (needs drag-drop)
- [ ] Remove or use `Visualizer::set_bands()`, `set_smoothing()`, `reset()`
- [ ] Remove or use `AudioDecoder::metadata()` (decide: file vs DB)
- [ ] Consolidate duplicate `format_duration()` functions

---

## Phase 8: GUI Enrichment & Batch Operations ✅

- [x] **Enrichment tab in UI**: Select tracks, identify, preview changes
- [x] **Batch progress**: Progress bar for multi-file enrichment
- [x] **Write tags button**: Apply metadata changes to files

**Implementation Notes (Phase 8):**

- Enrich pane created in `src/ui/views/enrich/` with 4 submodules
- Batch handlers in `src/ui/update/enrichment.rs` with `handle_enrich_pane()`
- Sequential processing with 500ms delay between tracks for rate limiting
- Auto-confirms high-confidence matches (≥70%), manual review for lower scores
- "Fill Only" option to preserve existing tags
- Results stored with full `TrackIdentification` for metadata writing
- Export report logs to tracing output

---

## Phase 8.25: Match Review & Cover Art Preview

**Philosophy**: High-confidence matches auto-apply (current behavior), but give users easy escape hatches when the auto-pick is wrong—without drowning them in choices.

### 8.25.1: Alternative Matches (Low-Effort Corrections)

When identification returns multiple candidates, keep the top 3-4 alternatives available for quick switching.

**Current State:**

- AcoustID returns multiple recordings per fingerprint (same song on different albums)
- `to_identifications()` in adapter.rs already expands recordings to multiple `TrackIdentification` objects
- Service picks "best" via `calculate_match_score()` (confidence + metadata hints)
- Alternatives are discarded

**Goal:** Keep alternatives, surface them when user clicks "Review"

**Data Model Changes:**

```rust
// In EnrichmentResult (state.rs)
pub struct EnrichmentResult {
    // ... existing fields ...
    
    /// The chosen identification (best match)
    pub identification: Option<TrackIdentification>,
    
    /// Alternative matches (top 2-3, for quick switching)
    /// Sorted by score, excludes the chosen one
    pub alternatives: Vec<TrackIdentification>,
    
    /// If true, show expanded view with alternatives
    pub show_alternatives: bool,
}
```

**Service Changes:**

```rust
// New return type that preserves alternatives
pub struct IdentificationWithAlternatives {
    pub best: TrackIdentification,
    pub alternatives: Vec<TrackIdentification>, // Top 3, filtered
}

// In EnrichmentService
pub async fn identify_track_with_alternatives(
    &self,
    path: &Path,
) -> Result<IdentificationWithAlternatives, EnrichmentError>
```

**UI Flow:**

```text
┌─────────────────────────────────────────────────────────────┐
│ ✓ Back in Black                              92%  [Review]  │
│   Artist: AC/DC  •  Album: Back in Black (1980)             │
└─────────────────────────────────────────────────────────────┘
                           ↓ Click Review
┌─────────────────────────────────────────────────────────────┐
│ ✓ Back in Black                              92%  [Collapse]│
│   Artist: AC/DC  •  Album: Back in Black (1980)    ◉ USE    │
├─────────────────────────────────────────────────────────────┤
│   Also found on:                                            │
│   ○ AC/DC Live (1992)                             88%       │
│   ○ Who Made Who (1986)                           85%       │
│   ○ [Search manually...]                                    │
└─────────────────────────────────────────────────────────────┘
```

**Tasks:**

- [ ] Add `alternatives: Vec<TrackIdentification>` to `EnrichmentResult`
- [ ] Add `show_alternatives: bool` to `EnrichmentResult`  
- [ ] Create `identify_track_with_alternatives()` that returns best + top 3
- [ ] Filter alternatives: skip if score < 0.5, skip duplicates, max 3
- [ ] UI: Expand/collapse alternatives on "Review" click
- [ ] UI: Radio buttons to switch selection
- [ ] Message: `EnrichSwitchAlternative(result_index, alt_index)`
- [ ] "Search manually" link → future manual search feature

### 8.25.2: Cover Art Preview

Show cover art thumbnail during review, before writing tags.

**Current State:**

- Cover art is fetched on-demand via `get_cover_art(release_id)`
- Requires MusicBrainz release ID (from `identification.track.release_id`)
- Not shown during enrichment preview

**Goal:** Fetch and display cover thumbnail when user reviews a result

**Approach:** Lazy loading - don't fetch covers for all results, only when user expands to review.

**Data Model Changes:**

```rust
// In EnrichmentResult
pub struct EnrichmentResult {
    // ... existing fields ...
    
    /// Cover art for the chosen identification (lazy loaded)
    pub cover_art: Option<CoverArtPreview>,
    
    /// Cover loading state
    pub cover_loading: bool,
}

pub struct CoverArtPreview {
    pub thumbnail: Vec<u8>,  // Small size (250px)
    pub release_id: String,  // For cache key
}
```

**UI Flow:**

```text
┌─────────────────────────────────────────────────────────────┐
│ ┌──────┐  ✓ Back in Black                    92%  [Collapse]│
│ │      │    Artist: AC/DC                                   │
│ │ 🖼️  │    Album: Back in Black (1980)           ◉ USE     │
│ │      │                                                    │
│ └──────┘  Also found on:                                    │
│           ○ AC/DC Live (1992)                   88%         │
│           ○ Who Made Who (1986)                 85%         │
└─────────────────────────────────────────────────────────────┘
```

**Tasks:**

- [ ] Add `cover_art: Option<CoverArtPreview>` to `EnrichmentResult`
- [ ] Add `cover_loading: bool` to `EnrichmentResult`
- [ ] On "Review" expand, trigger cover fetch if `release_id` exists
- [ ] Message: `EnrichCoverArtLoaded(result_index, Result<CoverArtPreview, String>)`
- [ ] UI: Show 64x64 thumbnail in expanded review section
- [ ] Show placeholder/spinner while loading
- [ ] Cache covers by release_id (reuse existing cover cache)

### 8.25.3: Manual Search (For Edge Cases)

For rare editions, wrong matches, or obscure tracks - let user search manually.

**Approach:** Simple MusicBrainz recording search, not full fingerprinting.

**UI Flow:**

```text
┌─────────────────────────────────────────────────────────────┐
│ Search MusicBrainz:                                         │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ back in black ac/dc                                     │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                              [Search]       │
├─────────────────────────────────────────────────────────────┤
│ Results:                                                    │
│ ○ Back in Black - AC/DC - Back in Black (1980)      [Use]  │
│ ○ Back in Black - AC/DC - Live at Donington (1992)  [Use]  │
│ ○ Back in Black - AC/DC - Iron Man 2 OST (2010)     [Use]  │
│                                     [Load More] [Cancel]    │
└─────────────────────────────────────────────────────────────┘
```

**Tasks:**

- [ ] Add MusicBrainz search endpoint to `MusicBrainzClient`
- [ ] Message: `EnrichManualSearchOpen(result_index)`
- [ ] Message: `EnrichManualSearchQuery(String)`
- [ ] Message: `EnrichManualSearchSelect(result_index, recording_id)`
- [ ] UI: Modal or inline search panel
- [ ] Pre-populate search with existing title/artist from result
- [ ] Rate limit: 1 req/sec to MusicBrainz

### Implementation Priority

| Feature | Complexity | Value | Order |
|---------|------------|-------|-------|
| Alternative matches (data) | Low | High | 1st |
| Alternative matches (UI) | Medium | High | 2nd |
| Cover art preview | Medium | Medium | 3rd |
| Manual search | Medium | Medium | 4th |

**Phase 8.25 can be tackled incrementally:**

1. First, just preserve alternatives in data model + show in results
2. Then add cover preview when reviewing  
3. Manual search is a "power user" escape hatch, lowest priority

## Phase 8.5: Library Gardener (Metadata Quality Nurturing) ✅

A background "gardener" that tends to your music library over time, identifying files
that could benefit from enrichment without being intrusive.

- [x] **Quality assessment types**: `QualityFlags` bitflags, `TrackQuality` struct
- [x] **Quality scorer**: Evaluates metadata completeness (missing artist/album/year, filename-as-title, generic placeholders)
- [x] **Database schema**: Added `quality_score`, `quality_flags`, `quality_checked_at` to tracks
- [x] **Background gardener**: `QualityGardener` processes tracks gradually during idle time
- [x] **UI indicators**: Quality badge (★●◐○?) in track list with tooltip showing issues
- [x] **Verification system**: Compare metadata against fingerprint results to detect mislabeled files
- [x] **Alternative matches storage**: DB tables for candidate matches and releases
- [x] **Watcher integration**: Auto-queue quality checks when files are added/modified
- [x] **Verification flags**: `TITLE_MISMATCH`, `ARTIST_MISMATCH`, `ALBUM_MISMATCH`, `POSSIBLY_MISLABELED`, `VERIFIED`, `MULTI_ALBUM`

**Quality Scoring:**

| Score | Tier | Meaning |
|-------|------|---------|
| 90-100 | ★ Excellent | Fully tagged, high confidence |
| 70-89 | ● Good | Minor gaps but usable |
| 50-69 | ◐ Fair | Significant metadata missing |
| 0-49 | ○ Poor | Needs attention |
| null | ? | Not yet analyzed |

**Quality Flags (Metadata):**

- `MISSING_ARTIST`, `MISSING_ALBUM`, `MISSING_YEAR`, `MISSING_TRACK_NUM`
- `TITLE_IS_FILENAME` - Title matches filename
- `GENERIC_METADATA` - "Unknown Artist", "Track 01", etc.
- `NO_MUSICBRAINZ_ID` - No verified ID
- `LOW_CONFIDENCE` - Fingerprint match <70%
- `NEVER_CHECKED` - Not yet analyzed

**Quality Flags (Verification):**

- `TITLE_MISMATCH` - Title doesn't match fingerprint result
- `ARTIST_MISMATCH` - Artist doesn't match fingerprint result
- `ALBUM_MISMATCH` - Album differs (may be compilation)
- `POSSIBLY_MISLABELED` - Significant mismatch, needs review
- `VERIFIED` - Metadata confirmed against fingerprint
- `AMBIGUOUS_MATCH` - Multiple good matches exist
- `MULTI_ALBUM` - Recording appears on multiple albums

---

## Phase 9: Audio Features (Winamp-inspired)

- [ ] **Equalizer**: 10-band EQ with presets (Rock, Pop, Jazz, etc.)
- [ ] **Gapless playback**: Seamless album playback without gaps
- [ ] **ReplayGain**: Volume normalization across tracks
- [ ] **Crossfade**: Smooth transitions between tracks (configurable 0-12s)
- [ ] **Playlist management**: Save/load playlists (.m3u8 format)

## Phase 10: UI Polish & UX Excellence

**Design philosophy**: Winamp's *spirit* (fast, fun, focused on audio) with a *modern* premium aesthetic. No retro skeuomorphism—instead, clean lines, thoughtful spacing, and small delightful surprises.

**Reference**: See `ENRICHMENT_UI_DESIGN.md` for detailed specs.

### 10.1 Design System Foundation ✅

- [x] **Theme constants file**: Colors, spacing, typography in `src/ui/theme.rs`
- [x] **Dark theme default**: Deep grays (#121215 base, #1a1a1f surfaces)
- [x] **Accent color**: Indigo primary (#6366f1)
- [x] **Consistent spacing**: 4px base unit system (XS/SM/MD/LG/XL scale)
- [x] **Typography scale**: Clear hierarchy (Hero → Tiny)
- [x] **Button variants**: Primary, Secondary, Ghost styles

### 10.2 Player Bar ✅

- [x] **72px bottom bar**: Fixed height, always visible
- [x] **Mini cover art**: 48x48 with rounded corners, clipped
- [x] **Track info**: Title + "Artist • Album" stacked
- [x] **Transport controls**: Prev/Play-Pause/Next with styled buttons
- [x] **Flexible seek bar**: Stretches to fill available space
- [x] **Volume section**: Icon (mute/low/high) + slider
- [x] **Device picker**: Dynamic icon (headphones/speaker) + dropdown, fixed width
- [x] **Shuffle/Repeat**: Icon buttons in player bar

### 10.3 Now Playing Pane ✅

- [x] **Cover art display**: Large cover with track info beside it
- [x] **Queue section**: Scrollable list with current track highlighted
- [x] **Queue controls**: Shuffle, repeat, clear in header
- [x] **Track position**: "Track X of Y" indicator
- [x] **Remove from queue**: X button per track (with scrollbar spacing)

### 10.4 Sidebar Polish ✅

- [x] **Styled nav items**: Icon + label, active state with Primary bg
- [x] **Hover states**: Surface-2 background on hover
- [x] **Status section**: "● Watching" indicator, track count stats
- [x] **Dividers**: Subtle separators between sections
- [x] **Collapsible mode**: 60px icon-only mode (toggle button)

### 10.5 Library Pane Refresh ✅

- [x] **Prominent search bar**: Styled input with search icon, always visible
- [x] **Filter chips**: Pill-shaped toggles (FLAC, MP3, Lossless)
- [x] **Track count display**: "3,428 tracks (showing 1,247)" when filtered
- [x] **Sort dropdown**: Replace column header clicks with clean dropdown
- [x] **Row hover states**: Subtle background change
- [x] **Format badges**: FLAC (green), MP3 (muted) inline badges
- [x] **Collapsible Organize section**: Hide by default, expand when needed

### 10.6 Settings Pane Cleanup ✅

- [x] **Organized sections**: Audio, Library, Enrichment, Appearance, About
- [x] **Section dividers**: Clear visual separation
- [x] **Input styling**: Consistent text inputs, dropdowns
- [x] **Toggle switches**: Styled boolean settings (status indicators)
- [x] **Version/tagline**: "Music Minder v0.1.4" with whimsical tagline

### 10.7 Enrich Pane (New) ✅

- [x] **Status indicators**: fpcalc ready, API key configured, rate limit status
- [x] **Track selection list**: Checkboxes, remove buttons, selection count
- [x] **Options section**: Fill-only vs overwrite, fetch cover art toggle
- [x] **Progress display**: Determinate bar with "2/4" count
- [x] **Results list**: Success/warning/error states with confidence scores
- [x] **Batch actions**: "Write All Confirmed", "Export Report"

### 10.8 Context Panel (Future)

- [ ] **Slide-in panel**: 320px from right edge
- [ ] **Selection summary**: "2 tracks selected"
- [ ] **Quick actions**: Identify, Write Tags, Play Next
- [ ] **Before/After preview**: Show metadata changes
- [ ] **Close button**: X to dismiss

### 10.9 Feedback & Polish

- [ ] **Toast notifications**: Non-blocking success/error messages
- [ ] **Confirmation dialogs**: Destructive action warnings
- [ ] **Empty states**: Helpful messages for empty library/queue/search
- [ ] **Loading states**: Spinners for async operations
- [ ] **Error states**: Friendly messages with recovery suggestions

### 10.10 Interaction Polish

- [ ] **Hover states**: All interactive elements respond
- [ ] **Focus indicators**: Keyboard navigation support
- [ ] **Smooth transitions**: 100-200ms for state changes
- [ ] **Playing indicator**: Gentle pulse on current track (subtle)

### 10.11 Delightful Touches

- [x] **Volume goes to 11**: Keep the Spinal Tap reference
- [ ] **Startup tagline**: Random "It really whips..." in console
- [ ] **Easter egg**: Hidden classic green theme unlock

## Backlog

### Winamp Nostalgia Features

- [ ] **Skin support**: Load classic Winamp skins (.wsz) or custom themes
- [ ] **Marquee scrolling**: Long titles scroll like the original
- [ ] **EQ presets**: Rock, Pop, Jazz, Classical, etc.
- [ ] **Milkdrop-style viz**: Advanced procedural visualizations
- [ ] **"Winamp, it really whips..."**: Easter egg on startup

### Audio Quality

- [ ] Bit-perfect / exclusive mode (WASAPI Exclusive, CoreAudio Integer)
- [ ] Hi-res audio indicator (24-bit, >48kHz)
- [ ] Dithering options for bit-depth conversion
- [ ] ASIO support (Windows, for pro audio interfaces)

### Library Features

- [ ] Duplicate detection
- [ ] Bulk metadata editing
- [ ] MusicBrainz release ID in database (for remote cover fetching)
- [ ] Smart playlists (auto-generated based on rules)
- [ ] Album view with grid layout
- [ ] Artist/Album grouping with collapsible sections (needs custom iced widget)
- [ ] Queue drag-and-drop reorder (needs custom iced widget)

### Integration

- [ ] Streaming radio (SHOUTcast/Icecast)
- [ ] Lyrics display (via external API)
- [ ] Scrobbling (Last.fm / ListenBrainz)
- [ ] Discord Rich Presence

---

## Phase 11: Streaming Service Integration (Future)

**Vision**: Bridge your legacy library with modern streaming—use your actual music taste (not just streaming history) to drive discovery, and always play the best quality source available.

### The Problem We're Solving

1. **"Spotify doesn't know my taste"**: 20 years of curated MP3s/FLACs represent real preferences, but streaming services only know what you've streamed
2. **"My old MP3s are 128kbps garbage"**: Some legacy files are worse than streaming quality
3. **"AI DJ is lame"**: Spotify's DJX optimizes for engagement, not genuine discovery based on your actual collection
4. **"Discovery feels disconnected"**: Recommendations don't account for what you already own and love

### Phase 11.1: Local Library as Taste Profile (Foundation)

Build the intelligence layer that understands your musical taste from your library.

- [ ] **Taste analysis engine**: Analyze library for genre/mood/era distribution
- [ ] **Audio feature extraction**: Tempo, energy, danceability, key (local analysis or via API)
- [ ] **Artist graph**: Build relationship map from your library (shared albums, collaborations)
- [ ] **Listening weight**: Track play counts/skips to weight preferences
- [ ] **Taste vector**: Generate embeddings representing your musical identity

### Phase 11.2: Spotify Discovery Integration

Use Spotify's API to find music you'll love based on your *actual* library.

- [ ] **Spotify OAuth flow**: Connect account via PKCE flow
- [ ] **Library matching**: Match local tracks to Spotify IDs (via ISRC, MusicBrainz, fuzzy search)
- [ ] **Audio features fetch**: Get Spotify's audio features for matched tracks
- [ ] **Recommendation engine**: Query Spotify recs seeded by your library's taste profile
- [ ] **Discovery queue**: "New music you might like" based on library analysis
- [ ] **"I own this" indicator**: Show which recommendations you already have locally
- [ ] **Wishlist**: Save discoveries for later acquisition

### Phase 11.3: Quality-Based Source Routing

Always play the best available version of a track.

- [ ] **Quality comparison**: Compare local quality vs Spotify lossless
  - Local FLAC 24-bit > Spotify Lossless > Local FLAC 16-bit > Spotify Lossless > Local 320kbps > Local 128kbps
- [ ] **Source indicator**: Show "Playing: Local FLAC" or "Playing: Spotify" in UI
- [ ] **Auto-upgrade**: Option to prefer Spotify when local < streaming quality
- [ ] **Manual override**: "Always play local" / "Always play Spotify" per-track
- [ ] **Quality report**: "47 tracks in your library have better versions on Spotify"

### Phase 11.4: Hybrid Playback (Experimental)

Seamless switching between local and Spotify playback.

- [ ] **Spotify Connect control**: Send playback commands to Spotify app/device
- [ ] **Queue handoff**: When queue transitions Local→Spotify, hand off gracefully
- [ ] **Crossfade bridge**: Use 1-2s crossfade to mask player switches
- [ ] **Unified queue view**: Single queue showing both local and Spotify tracks
- [ ] **Playback mode toggle**: "Local Only" / "Spotify Only" / "Hybrid (Best Quality)"

**Known Limitations:**

- No true gapless across player boundary
- Requires Spotify Premium for playback control
- Two "now playing" states to synchronize
- Media controls may behave unexpectedly during handoff

### Phase 11.5: AI DJ (Better Than DJX)

A DJ that actually knows your taste because it's trained on your real library.

- [ ] **Library-seeded generation**: Build playlists from "70% owned, 30% discovery"
- [ ] **Mood/energy flow**: Sequence tracks for energy arc (build up, peak, cool down)
- [ ] **Time-aware**: Morning chill, afternoon energy, evening wind-down
- [ ] **Context-aware**: "More like this" based on current track
- [ ] **Avoid overplay**: Don't repeat recent plays, diversify within taste
- [ ] **Explanation**: "Playing this because you have 12 albums by similar artists"
- [ ] **Feedback loop**: Thumbs up/down to refine taste model

### Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                      Music Minder                           │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌───────────────┐ │
│  │ Local Library │    │ Spotify API  │    │   AI DJ       │ │
│  │ + MusicBrainz │◄──►│ + Features   │◄──►│ Taste Model   │ │
│  │ + Quality DB  │    │ + Recs       │    │ + Sequencing  │ │
│  └──────┬───────┘    └──────┬───────┘    └───────┬───────┘ │
│         │                   │                    │         │
│         ▼                   ▼                    ▼         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Unified Playback Router                    ││
│  │  • Compare quality scores (local vs streaming)          ││
│  │  • Route to local player OR Spotify Connect             ││
│  │  • Crossfade handoff at source boundaries               ││
│  │  • Single queue abstraction                             ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Alternative Services

Spotify isn't the only option—architecture should be service-agnostic:

| Service | API Quality | Lossless | Notes |
|---------|-------------|----------|-------|
| **Spotify** | Excellent | Yes (2025) | Largest catalog, best recs API |
| **Tidal** | Good | Yes (MQA/FLAC) | Audiophile-focused |
| **Qobuz** | Good | Yes (Hi-Res) | Best quality, smaller catalog |
| **YouTube Music** | Decent | No | Huge catalog including obscure |
| **ListenBrainz** | Open | N/A | Open-source recs, no playback |

### Implementation Priority

| Feature | Difficulty | Value | Priority |
|---------|------------|-------|----------|
| Taste analysis from library | Medium | High | 1st |
| Spotify OAuth + matching | Medium | High | 2nd |
| Recommendations API | Easy | Very High | 3rd |
| Quality comparison | Easy | High | 4th |
| Source routing (manual) | Medium | High | 5th |
| Hybrid playback | Hard | Medium | 6th |
| AI DJ | Hard | Very High | 7th |
| Gapless handoff | Very Hard | Low | Later |

### Open Questions

- [ ] **Spotify TOS**: Is quality-based routing (avoiding Spotify playback) acceptable?
- [ ] **Offline taste model**: Can we do audio feature extraction locally (no API)?
- [ ] **Plugin architecture**: Should streaming services be plugins?
- [ ] **ListenBrainz integration**: Use open-source recs as Spotify alternative?

### Advanced

- [ ] Waveform seek preview
- [ ] Audio device hot-swap detection
- [ ] Headphone/speaker profiles
- [ ] DSP plugin architecture

---

## Technical Debt: Duplicate Load+Play Paths

**Status**: ✅ Fixed

### The Solution

Created a single `load_and_play_current()` method in `Player` that is the ONLY place sending `Load` + `Play` commands:

```rust
fn load_and_play_current(&mut self) -> Result<(), PlayerError> {
    if let Some(item) = self.queue.current() {
        self.command_tx
            .send(PlayerCommand::Load(item.path.clone()))
            .map_err(|_| PlayerError::ChannelClosed)?;
        self.command_tx
            .send(PlayerCommand::Play)
            .map_err(|_| PlayerError::ChannelClosed)?;
    }
    Ok(())
}
```

All three entry points now use this single method:

- `play_file()` → clear, add, `jump_to(0)`, `load_and_play_current()`
- `skip_forward()` → `queue.skip_forward()`, `load_and_play_current()`
- `previous()` → `queue.previous()` (or seek if >3s), `load_and_play_current()`

---

## Technical Debt: Background Service Initialization Pattern

**Status**: 🔄 Partial (Gardener uses new pattern, Watcher/Diagnostics use old)

### The Problem

Background services (watcher, gardener, diagnostics) use inconsistent initialization patterns:

1. **Watcher**: Uses Iced subscription pattern, complex stream-based lifecycle
2. **Gardener**: Starts during app init, stores `command_tx` in state (simpler)
3. **Diagnostics**: Ad-hoc, manual lifecycle management

### The Solution

Standardize on the Gardener pattern:

```rust
// During app initialization
let gardener = QualityGardener::new(pool.clone());
let gardener_tx = gardener.command_sender();
gardener.start();

// Store in state
state.gardener_state.command_tx = Some(gardener_tx);
```

### Refactoring Tasks

- [ ] **Watcher**: Refactor from subscription to init-time start pattern
  - Move `FileWatcher::new_async()` to `init_db_and_services()`
  - Store `watcher` handle and event receiver in state
  - Use simple `Task::stream` to poll events (not `iced::Subscription`)
- [ ] **Diagnostics**: Add background diagnostics service
  - Periodic system checks (audio device changes, CPU load)
  - Command channel for on-demand full diagnosis
- [ ] **Unified service manager**: Consider `ServiceManager` struct
  - Single place to start/stop all background services
  - Graceful shutdown coordination

---

## Deferred & Incomplete Items

Items marked complete at the phase level but with outstanding sub-tasks. These are tracked here for visibility and prioritization.

### From Phase 7: Library UX & Queue Management

**7.3 Queue Management:**

- [ ] **Reorder queue**: Drag-and-drop to rearrange *(needs custom Iced widget)*

**7.4 Keyboard Shortcuts** *(complete)*:

- [x] **Space**: Play/pause toggle
- [x] **←/→**: Previous/next track  
- [x] **Shift+←/→**: Seek backward/forward 5s
- [x] **↑/↓**: Navigate selection (Alt+↑/↓ for volume)
- [x] **Ctrl+F**: Focus search box (clears search)
- [x] **Enter**: Play selected track
- [x] **Delete**: Remove selected from queue
- [x] **Escape**: Clear search / close panels
- [ ] **Global hotkeys**: Control playback from any app *(future)*

**7.5 Now Playing Enhancements** *(complete)*:

- [x] **Queue count display**: "Track 3 of 25" indicator
- [x] **Track info panel**: Format, bitrate, file path display
- [x] **Read metadata from file**: Decoder reads tags via `TrackLoaded` event
- [x] **Metadata fallback chain**: DB → file tags → filename

**7.6 Code Cleanup:**

- [ ] Wire up `PlayQueue::reorder()` *(needs drag-drop)*
- [ ] Remove or use `Visualizer::set_bands()`, `set_smoothing()`, `reset()`
- [x] ~~Remove or use `AudioDecoder::metadata()`~~ → Now used for fallback metadata
- [ ] Consolidate duplicate `format_duration()` functions

### From Phase 10: UI Polish

**10.8 Context Panel:**

- [ ] **Slide-in panel**: 320px from right edge
- [ ] **Selection summary**: "2 tracks selected"
- [ ] **Quick actions**: Identify, Write Tags, Play Next
- [ ] **Before/After preview**: Show metadata changes
- [ ] **Close button**: X to dismiss

**10.9 Feedback & Polish:**

- [ ] **Toast notifications**: Non-blocking success/error messages
- [ ] **Confirmation dialogs**: Destructive action warnings
- [ ] **Empty states**: Helpful messages for empty library/queue/search
- [ ] **Loading states**: Spinners for async operations
- [ ] **Error states**: Friendly messages with recovery suggestions

**10.10 Interaction Polish:**

- [ ] **Hover states**: All interactive elements respond
- [ ] **Focus indicators**: Keyboard navigation support
- [ ] **Smooth transitions**: 100-200ms for state changes
- [ ] **Playing indicator**: Gentle pulse on current track

**10.11 Delightful Touches:**

- [ ] **Startup tagline**: Random "It really whips..." in console
- [ ] **Easter egg**: Hidden classic green theme unlock

### Technical Debt (Partial)

- [ ] **Watcher refactor**: Migrate from Iced subscription to init-time start pattern
- [ ] **Diagnostics service**: Background service with periodic checks
- [ ] **Unified service manager**: Single ServiceManager for all background services

---

## Priority Matrix: Incomplete Items

Ranked by impact vs effort for deciding what to tackle next.

### Quick Wins (Low Effort, High Impact)

| Item | Effort | Notes |
|------|--------|-------|
| ~~Keyboard: Space for play/pause~~ | ~~Low~~ | ✅ Done |
| ~~Keyboard: ←/→ for prev/next~~ | ~~Low~~ | ✅ Done |
| Startup tagline | Low | Fun, adds personality |
| Consolidate `format_duration()` | Low | Code cleanup |
| Toast notifications | Medium | Useful for all actions |

### Medium Effort, High Value

| Item | Effort | Notes |
|------|--------|-------|
| ~~All keyboard shortcuts~~ | ~~Medium~~ | ✅ Done (8/9 - global hotkeys deferred) |
| Empty states | Medium | Better UX for new users |
| Loading/error states | Medium | Polish across app |
| Alternative matches (8.25.1) | Medium | Already scoped above |

### Harder But Important

| Item | Effort | Notes |
|------|--------|-------|
| Queue drag-drop reorder | Hard | Needs custom widget |
| Context panel | Hard | New UI surface |
| ~~Metadata fallback chain~~ | ~~Medium~~ | ✅ Done |
| Cover art preview (8.25.2) | Medium | Async + caching |

### Lower Priority / Future

| Item | Effort | Notes |
|------|--------|-------|
| Global hotkeys | Hard | Platform-specific |
| Visualizer API cleanup | Low | Only if we extend viz |
| Service manager unification | Medium | Architectural cleanup |
| Easter egg theme | Low | Fun but not urgent |
