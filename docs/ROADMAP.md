# Music Minder Roadmap

## Phase 1: Foundation ✅

- [x] Project setup with Rust 2024 edition
- [x] Dependencies: Iced 0.13, Tokio, SQLx, Lofty, Clap
- [x] SQLite database with migrations
- [x] Optimized build configuration

## Phase 2: Scanning & Library ✅

- [x] Recursive directory scanner (MP3, FLAC, OGG, WAV, M4A)
- [x] Metadata extraction (Artist, Album, Title, Track#, Duration)
- [x] Virtualized library view (10k+ tracks)
- [x] Live scan progress updates

## Phase 3: Organization ✅

- [x] Pattern-based file organization
- [x] Streaming preview with parallel file checks
- [x] Virtualized preview list
- [x] Undo support with JSON persistence
- [x] Batch database updates

## Phase 4: Enrichment ✅

- [x] **AcoustID Integration**: Audio fingerprinting via fpcalc + API lookup
- [x] **Smart Matching**: Prefers correct album based on path/metadata hints
- [x] **MusicBrainz Lookup**: Fetch detailed metadata by recording ID
- [x] **Cover Art**: Download from Cover Art Archive
- [x] **Enrichment Service**: High-level orchestration with rate limiting

## Phase 5: Integration ✅

- [x] **CLI `identify` command**: Single file identification with smart matching
- [x] **CLI `write-tags` command**: Write metadata to files with preview mode
- [x] **Metadata Writing**: `--write` and `--fill-only` flags on identify
- [x] **CLI `enrich` command**: Batch enrichment with health tracking, dry-run, recursive scan
- [ ] **GUI Enrichment**: Cover art display, batch processing in UI

## Phase 6: Playback & UX ✅

- [x] **Audio playback**: Play tracks from library
- [x] **Now Playing view**: Track info, progress bar, queue display
- [x] **Cover art resolution**: Embedded, sidecar, cached, remote (non-blocking)
- [x] **Cover art display**: Album art in Now Playing view (200x200 with source indicator)
- [x] **Visualization modes**: Spectrum, Waveform, VU Meter

## Phase 7: System Integration & Polish (Current)

- [x] **OS media controls**: Windows SMTC / Linux MPRIS / macOS via `souvlaki` crate
  - Media key support (play/pause/next/prev from keyboard)
  - System overlay with track info + album art
  - Bluetooth/headphone button controls
- [x] **Refactor: Unify playback initiation** (see below)
- [ ] **Keyboard shortcuts**: Play/pause (Space), next/prev (←/→), volume (↑/↓)
- [ ] **Search/filter**: Filter library by artist, album, or title
- [ ] **GUI Enrichment**: Batch processing with progress in UI

## Phase 8: Audio Features (Winamp-inspired)

- [ ] **Equalizer**: 10-band EQ with presets (Rock, Pop, Jazz, etc.)
- [ ] **Gapless playback**: Seamless album playback without gaps
- [ ] **ReplayGain**: Volume normalization across tracks
- [ ] **Crossfade**: Smooth transitions between tracks (configurable 0-12s)
- [ ] **Playlist management**: Save/load playlists (.m3u8 format)

## Phase 9: UI Polish

- [ ] **Icon-based controls**: Replace ASCII `|>` with proper SVG icons
- [ ] **Color theming**: Accent colors, proper dark mode
- [ ] **Progress bar redesign**: Rounded, colored, buffer indicator
- [ ] **Hover/active states**: Visual feedback on all interactive elements
- [ ] **Typography hierarchy**: Proper font sizes and weights
- [ ] **Mini player mode**: Compact floating window option
- [ ] **Animations**: Subtle transitions on state changes

## Backlog

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
