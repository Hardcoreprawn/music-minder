# Project Status Summary

## Completed Features ✅

### Phase 1: Foundation

- ✅ Cargo project with Rust 2024 edition
- ✅ Dependencies: Iced 0.13 (UI), Tokio (async), SQLx (SQLite), Lofty (metadata), Clap (CLI)
- ✅ Database schema (Artists, Albums, Tracks)
- ✅ Automatic migrations on startup
- ✅ Optimized build configuration (LTO, reduced features)

### Phase 2: Scanning & Library

- ✅ **Scanner Module** (`src/scanner/mod.rs`)
  - Recursively scans directories for audio files
  - Supports: MP3, FLAC, OGG, WAV, M4A
  - Async streaming architecture
  - Unit tested with tempfiles
  
- ✅ **Metadata Reader** (`src/metadata/mod.rs`)
  - Uses `lofty` crate for tag reading
  - Extracts: Artist, Album, Title, Duration, Track Number
  - Handles missing tags gracefully with defaults
  - Unit tested for error cases

- ✅ **Database Layer** (`src/db/mod.rs`, `src/model/mod.rs`)
  - `get_or_create_artist` - Upsert logic for artists
  - `get_or_create_album` - Upsert logic for albums
  - `insert_track` - Upsert with `ON CONFLICT`
  - `get_all_tracks` - Fetch complete library
  - `batch_update_track_paths` - Batch path updates in single transaction
  - Comprehensive test coverage (6 tests)
  
- ✅ **CLI Interface** (`src/main.rs`)
  - `scan <path>` - Scan directory and add to DB
  - `list` - Show all tracks
  - `organize` - Organize files with pattern
  - No arguments - Launch GUI

### Phase 3: Organization

- ✅ **Organizer Module** (`src/organizer/mod.rs`)
  - Pattern-based file moving: `{Artist}/{Album}/{TrackNum} - {Title}.{ext}`
  - Filename sanitization (removes `/`, `\`, `:`, etc.)
  - Cross-device copy+delete fallback
  - Undo log with JSON persistence
  - Comprehensive test coverage (6 tests)
  
- ✅ **Organize CLI Command**
  - `organize -d <dest> [-p <pattern>] [--dry-run]`
  - Dry-run mode for safety
  - Database path updates after moving

- ✅ **Organize GUI**
  - Destination folder picker
  - Pattern input with live preview
  - Streaming preview generation (batched, parallel)
  - Virtualized preview list (handles 10k+ files)
  - Progress tracking during organization
  - Undo last operation button

## Architecture Highlights

### Modular Design

```text
src/
├── main.rs          # Entry point + CLI routing
├── model/           # Database entities
├── scanner/         # File system traversal
├── metadata/        # Tag extraction (lofty)
├── db/              # SQLx queries
├── library/         # High-level scan operation
├── organizer/       # File moving logic
└── ui/
    ├── mod.rs       # Core app struct & dispatch (94 lines)
    ├── views.rs     # All view rendering functions
    ├── update.rs    # Message handlers (scan/organize/undo)
    ├── messages.rs  # Message enum
    ├── state.rs     # State types + virtualization constants
    ├── streams.rs   # Async subscription streams
    └── platform.rs  # Platform utilities
```

### Testing Strategy

- **17 unit tests** across modules
- DB tests: init, artist, album, track, batch update, get_all
- Organizer tests: preview, sanitize, move, undo log, undo restore
- Integration tests via CLI commands

### Dev Container Compatibility

- Pure Rust backend works in container
- GUI requires X11 forwarding or native Windows/WSL execution
- CLI is fully functional for headless testing

## Usage Examples

### Scan a music folder

```bash
cargo run -- scan /path/to/music
```

### View library

```bash
cargo run -- list
```

### Organize files (dry-run first!)

```bash
cargo run -- organize --destination /organized --dry-run
cargo run -- organize --destination /organized
```

### Custom pattern

```bash
cargo run -- organize -d /music -p "{Album}/{TrackNum}. {Artist} - {Title}.{ext}"
```

### Identify a track (requires ACOUSTID_API_KEY env var)

```bash
# Set API key (get one at https://acoustid.org/new-application)
export ACOUSTID_API_KEY=your_key_here

# Identify a single file
cargo run -- identify /path/to/song.mp3

# Identify and write tags to file
cargo run -- identify /path/to/song.mp3 --write

# Only fill empty fields (don't overwrite existing)
cargo run -- identify /path/to/song.mp3 --write --fill-only
```

### Write tags manually

```bash
# Preview what would be written
cargo run -- write-tags /path/to/song.mp3 --title "Song" --artist "Artist" --preview

# Write tags
cargo run -- write-tags /path/to/song.mp3 --title "Song" --artist "Artist" --album "Album"

# Only fill empty fields
cargo run -- write-tags /path/to/song.mp3 --title "Song" --fill-only
```

## What's Next?

### Phase 5: Integration (Current)

- [x] CLI `identify` command with smart matching
- [x] CLI `write-tags` command with preview mode
- [x] Metadata writing with `--write` and `--fill-only` flags
- [ ] CLI `enrich` command for batch processing
- [ ] GUI enrichment improvements (cover art display, batch processing)

### Phase 4: Enrichment ✅ (Complete)

- ✅ AcoustID fingerprinting (via fpcalc)
- ✅ AcoustID API client with smart URL encoding (see API Quirks below)
- ✅ Smart matching algorithm (prefers correct album based on path/metadata)
- ✅ MusicBrainz API client with DTOs and contract tests  
- ✅ Cover Art Archive client
- ✅ Enrichment service with rate limiting
- ✅ 58 tests total

### Backlog

- [ ] Dark mode theme
- [ ] Audio playback
- [ ] Playlist management
- [ ] Duplicate detection
- [ ] Bulk metadata editing

## Known Issues / Limitations

1. GUI requires Windows native or X11 forwarding in containers
2. Database paths not updated if files manually moved
3. No duplicate file detection

## API Quirks & Workarounds

### AcoustID Meta Parameter Encoding

**Problem**: The AcoustID API uses `+` as a separator in the `meta` parameter
(e.g., `meta=recordings+releasegroups`). Standard URL encoding converts `+` to `%2B`,
but the API does NOT recognize `%2B` as a separator. When encoded, the API returns
results without metadata fields.

**Solution**: Manually construct URLs with literal `+` characters instead of using
reqwest's `.query()` method which auto-encodes.

**Evidence**:

- `meta=recordings%2Breleasegroups` → Returns only `id` and `score`
- `meta=recordings+releasegroups` → Returns full recordings metadata

### AcoustID Duration Field

The API returns duration as a float (e.g., `353.0`) not an integer. Our DTOs use `f64`
to handle this correctly.

## Testing the App

### Run Tests

```bash
cargo test
```

### Quick Demo

```bash
# Create test files
mkdir -p test_music/unsorted
ffmpeg -f lavfi -i "sine=frequency=1000:duration=1" \
  -metadata title="Test Song" -metadata artist="Test Artist" \
  test_music/unsorted/test.mp3 -y

# Scan
cargo run -- scan test_music

# Organize (CLI)
cargo run -- organize -d organized_music --dry-run
cargo run -- organize -d organized_music

# Launch GUI
cargo run
```

## Performance Optimizations

- **Virtualized lists**: Only renders visible items (handles 10k+ tracks)
- **Parallel file checks**: Rayon for concurrent I/O
- **Batch DB operations**: Single transaction for multiple updates
- **SmallVec**: Stack allocation for small error lists
- **Optimized builds**: Thin LTO, reduced tokio/chrono features

## Smart Matching Algorithm

When identifying tracks, the AcoustID API often returns 100+ possible matches
(the same song appears on many albums, compilations, karaoke releases, etc.).
Our smart matching algorithm selects the best match using:

| Factor | Score Impact |
|--------|--------------|
| AcoustID confidence | Base score (0.0-1.0) |
| Album name in file path | +15% |
| Album matches embedded tags | +20% |
| Artist matches embedded tags | +10% |
| Compilation when path has "hits/greatest/best" | +10% |
| Karaoke release | -25% |
| Live (when path doesn't indicate live) | -10% |
| Remix (when path doesn't indicate remix) | -15% |
| Original studio album (no secondary types) | +5% |

This ensures that a file in `/Music/Queen/Greatest Hits I/` correctly identifies
as "Greatest Hits" rather than "Artist Karaoke Series: Queen".
