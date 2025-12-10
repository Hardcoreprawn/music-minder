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

### 7.1 Smart Background Scanning ✅

Never interrupt playback. Keep the library fresh automatically.

- [x] **Watch directories**: Use `notify` crate to detect file changes
- [x] **Incremental updates**: Only rescan changed/new files (mtime-based)
- [x] **Background thread**: Watcher runs on dedicated thread, events via channel
- [x] **Startup scan**: Watch paths auto-start on app launch
- [x] **Scan indicator**: Subtle "● Watching" / "⟳ Syncing" in sidebar
- [x] **Never interrupt audio**: File changes queued, processed in batches
- [x] **CLI command**: `music-minder watch <path> -v --db <db> --scan-first`
- [ ] **Manual refresh**: Button to force full rescan if needed (optional)

### 7.2 Library Search & Filter (High Priority)

- [ ] **Search bar**: Filter tracks by typing (searches title, artist, album)
- [ ] **Instant filtering**: Results update as you type (no Enter needed)
- [ ] **Column sorting**: Click column headers to sort (Artist, Album, Title, Duration)
- [ ] **Sort indicator**: Visual arrow showing sort direction
- [ ] **Filter chips**: Quick filters for format (FLAC/MP3), lossless, etc.
- [ ] **Artist/Album grouping**: Collapsible groups in library view

### 7.3 Queue Management (High Priority)

The queue infrastructure exists but UI controls are missing:

- [ ] **Queue panel**: Visible queue in Now Playing view (scrollable list)
- [ ] **Current track highlight**: Visual indicator of what's playing
- [ ] **Double-click to jump**: Click any queued track to play it immediately
- [ ] **Remove from queue**: X button or swipe to remove tracks
- [ ] **Reorder queue**: Drag-and-drop to rearrange (reorder() exists)
- [ ] **Clear queue**: Button to clear entire queue
- [ ] **Repeat modes**: Off / All / One with visual toggle (cycle_repeat exists)
- [ ] **Shuffle toggle**: Shuffle on/off button (set_shuffle exists)
- [ ] **Play next**: Right-click → "Play Next" (add_next exists)

### 7.4 Keyboard Shortcuts (Medium Priority)

Winamp's global hotkeys were legendary. Start with in-app, then go global.

- [ ] **Space**: Play/pause toggle
- [ ] **←/→**: Previous/next track
- [ ] **Shift+←/→**: Seek backward/forward 5s
- [ ] **↑/↓**: Volume up/down
- [ ] **Ctrl+F**: Focus search box
- [ ] **Enter**: Play selected track
- [ ] **Delete**: Remove selected from queue
- [ ] **Escape**: Clear search / close panels
- [ ] **Global hotkeys** (future): Control playback from any app

### 7.5 Now Playing Enhancements (Medium Priority)

- [ ] **Read metadata from file**: Use decoder metadata, not just DB lookup
- [ ] **Metadata fallback chain**: File tags → DB cache → filename
- [ ] **Track info panel**: Expandable details (format, bitrate, file path)
- [ ] **Queue count display**: "Track 3 of 25" indicator

### 7.5 Code Cleanup (Low Priority)

Remove or wire up unused code identified in review:

- [ ] Wire up `PlayQueue::cycle_repeat()` to UI button
- [ ] Wire up `PlayQueue::set_shuffle()` to UI toggle
- [ ] Wire up `PlayQueue::remove()` and `reorder()` to queue panel
- [ ] Remove or use `Visualizer::set_bands()`, `set_smoothing()`, `reset()`
- [ ] Remove or use `AudioDecoder::metadata()` (decide: file vs DB)
- [ ] Consolidate duplicate `format_duration()` functions

---

## Phase 8: GUI Enrichment & Batch Operations

- [ ] **Enrichment tab in UI**: Select tracks, identify, preview changes
- [ ] **Batch progress**: Progress bar for multi-file enrichment
- [ ] **Write tags button**: Apply metadata changes to files
- [ ] **Cover art preview**: Show fetched cover before applying
- [ ] **Conflict resolution**: Handle multiple matches, let user choose

---

## Phase 9: Audio Features (Winamp-inspired)

- [ ] **Equalizer**: 10-band EQ with presets (Rock, Pop, Jazz, etc.)
- [ ] **Gapless playback**: Seamless album playback without gaps
- [ ] **ReplayGain**: Volume normalization across tracks
- [ ] **Crossfade**: Smooth transitions between tracks (configurable 0-12s)
- [ ] **Playlist management**: Save/load playlists (.m3u8 format)

## Phase 10: UI Polish (Winamp Aesthetic)

Capture that iconic late-90s/early-2000s look with modern rendering.

### Visual Design

- [ ] **Dark theme default**: Deep grays, not pure black
- [ ] **Accent color**: Classic Winamp green (#00FF00) or customizable
- [ ] **Beveled edges**: Subtle 3D effect on panels (like classic skins)
- [ ] **LED-style displays**: Time display with that digital clock look
- [ ] **Compact mode**: Tiny player bar (like Winamp's windowshade mode)

### Controls

- [ ] **Icon-based transport**: Proper play/pause/stop/prev/next icons
- [ ] **Scrubber bar**: Rounded progress bar with position indicator
- [ ] **Volume slider**: Classic horizontal slider with notches
- [ ] **Balance control**: (optional) L/R balance like original Winamp

### Visualization

- [ ] **Visualization window**: Dedicated viz area in Now Playing
- [ ] **Multiple viz modes**: Spectrum bars, oscilloscope, VU meters
- [ ] **Smoothing options**: Expose `set_smoothing()` in settings
- [ ] **Color schemes**: Classic green, fire, rainbow presets
- [ ] **Fullscreen viz**: Press V to go fullscreen visualization

### Polish

- [ ] **Hover states**: Visual feedback on all clickables
- [ ] **Click feedback**: Brief flash/depression on buttons
- [ ] **Smooth scrolling**: Buttery library scrolling
- [ ] **Loading states**: Subtle spinners, never frozen UI
- [ ] **Tooltips**: Helpful hints on hover

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

### Integration

- [ ] Streaming radio (SHOUTcast/Icecast)
- [ ] Lyrics display (via external API)
- [ ] Scrobbling (Last.fm / ListenBrainz)
- [ ] Discord Rich Presence

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
