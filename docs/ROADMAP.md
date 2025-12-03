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

## Phase 6: Playback & UX (Current)

- [x] **Audio playback**: Play tracks from library (already complete)
- [x] **Now Playing view**: Track info, progress bar (already complete)
- [x] **Cover art resolution**: Embedded, sidecar, cached, remote (non-blocking)
- [ ] **Cover art display**: Show album art in Now Playing view
- [ ] **Keyboard shortcuts**: Play/pause, next/prev, volume

## Backlog

- [ ] Dark mode theme
- [ ] Playlist management
- [ ] Duplicate detection
- [ ] Bulk metadata editing
- [ ] MusicBrainz release ID in database (for remote cover fetching)
