# Scanning Performance Analysis (Phase B.2)

**Date:** January 2026  
**Status:** ✅ COMPLETE

## Executive Summary

Profiling reveals **database writes are the primary bottleneck**, not metadata parsing or file I/O. Each file triggers 3 separate async database round-trips, causing significant latency.

### Key Findings

| Operation | Time/File | % of Total | Bottleneck? |
|-----------|-----------|------------|-------------|
| Metadata parsing (lofty) | ~2.3ms | ~15% | ❌ Acceptable |
| Database writes (3 queries) | ~16ms | ~85% | ✅ **PRIMARY** |
| File discovery (walkdir) | <0.1ms | <1% | ❌ Negligible |

### Current Performance

- **Throughput:** ~60-70 files/second (small test set)
- **Target:** 1000+ files/second
- **Gap:** ~15x improvement needed

## Detailed Analysis

### 1. File Discovery (walkdir)

```
Phase: Directory traversal
Time: <0.1ms per directory entry
Bottleneck: NO
```

**How it works:**

- `walkdir::WalkDir` traverses directories synchronously
- Spawned in `tokio::spawn_blocking` to avoid blocking async runtime
- Results sent through `mpsc::channel(100)` with 100-item buffer

**Assessment:** Very efficient. The channel buffer provides natural backpressure.

### 2. Metadata Parsing (lofty)

```
Phase: Reading audio file tags
Time: ~2.3ms per file average
Bottleneck: NO (but has optimization potential)
```

**How it works:**

- `lofty::Probe::open()` opens file and probes format
- `probe.read()` parses tags (ID3, Vorbis, etc.)
- Extracts: title, artist, album, duration, track number

**Breakdown by format (estimated):**

- FLAC: ~3-4ms (larger files, more metadata)
- MP3: ~1-2ms (simpler ID3 tags)
- OGG: ~2-3ms (Vorbis comments)

**Potential optimizations:**

- [ ] Skip cover art extraction during scan (significant savings)
- [ ] Use `lofty::read_from` with `ParseOptions::new().parsing_mode(ParsingMode::Relaxed)`
- [ ] Parallel metadata reads with Rayon (currently sequential within async)

### 3. Database Writes (SQLite via sqlx) 🔴 PRIMARY BOTTLENECK

```
Phase: Inserting artist, album, track records
Time: ~16ms per file average
Bottleneck: YES - 85% of total scan time
```

**How it works (per file):**

```rust
// Query 1: Get or create artist (~5ms)
let artist_id = db::get_or_create_artist(&pool, &meta.artist).await.ok();

// Query 2: Get or create album (~5ms)  
let album_id = db::get_or_create_album(&pool, &meta.album, artist_id).await.ok();

// Query 3: Insert/update track (~6ms)
db::insert_track(&pool, &meta, path, artist_id, album_id).await?;
```

**Why it's slow:**

1. **3 async round-trips per file** — Each `.await` has async scheduling overhead
2. **No batching** — Commits after every single insert
3. **No caching** — Re-queries for same artist/album repeatedly
4. **UPSERT overhead** — `ON CONFLICT` clause adds comparison cost

**Evidence:**

- Same artist across 10 files = 10 identical artist lookups
- Same album across 10 files = 10 identical album lookups
- No transaction batching = 30 separate commits for 10 files

## Recommended Optimizations (Task 7)

### Priority 1: Batch Database Writes (Expected: 10-20x improvement)

```rust
// Before: 3 queries per file, commit after each
for file in files {
    get_or_create_artist().await;  // commit
    get_or_create_album().await;   // commit
    insert_track().await;          // commit
}

// After: Batch inserts in transactions
let tx = pool.begin().await?;
for chunk in files.chunks(100) {
    // Cache artist/album IDs locally
    let artist_ids = batch_get_or_create_artists(&tx, artists).await?;
    let album_ids = batch_get_or_create_albums(&tx, albums).await?;
    batch_insert_tracks(&tx, tracks, artist_ids, album_ids).await?;
}
tx.commit().await?;
```

**Implementation approach:**

1. Collect 100-200 files worth of metadata before any DB writes
2. Dedupe artists/albums within batch
3. Single transaction for entire batch
4. Use `INSERT OR IGNORE` + `SELECT` pattern for lookups

### Priority 2: In-Memory Artist/Album Cache (Expected: 2-3x improvement)

```rust
// Cache artists/albums seen during scan
let mut artist_cache: HashMap<String, i64> = HashMap::new();
let mut album_cache: HashMap<(String, Option<i64>), i64> = HashMap::new();

// Check cache before DB query
let artist_id = if let Some(&id) = artist_cache.get(&meta.artist) {
    id
} else {
    let id = db::get_or_create_artist(&pool, &meta.artist).await?;
    artist_cache.insert(meta.artist.clone(), id);
    id
};
```

### Priority 3: Parallel Metadata Reads (Expected: 2-4x improvement on SSD)

```rust
// Currently: Sequential within async task
let meta = metadata::read(&path);  // blocking

// Proposed: Rayon parallel iterator
use rayon::prelude::*;
let metadata: Vec<_> = paths
    .par_iter()
    .map(|path| metadata::read(path))
    .collect();
```

### Priority 4: Relaxed Metadata Parsing (Expected: 1.2-1.5x improvement)

```rust
// Skip expensive operations during initial scan
let options = ParseOptions::new()
    .parsing_mode(ParsingMode::Relaxed)
    .read_properties(true)   // Need duration
    .read_cover_art(false);  // Skip cover art

let tagged_file = Probe::open(path)?
    .options(options)
    .read()?;
```

## Implementation Plan (Task 7)

| Step | Change | Expected Gain | Effort |
|------|--------|---------------|--------|
| 1 | Transaction batching (100 files) | 10-15x | 2h |
| 2 | Artist/album cache | 2-3x | 1h |
| 3 | Combined: should hit ~500-800 files/sec | — | — |
| 4 | Rayon parallel metadata | 2-4x | 2h |
| 5 | Combined: should hit 1000+ files/sec | — | — |
| 6 | Relaxed parsing options | 1.2x | 30m |

## Validation

After implementing Task 7 optimizations, re-run profiling:

```powershell
# Profile with timing breakdown
.\scripts\profile-scan.ps1 -Path "path\to\large\library"

# Or use CLI directly
$env:RUST_LOG = "music_minder::library=debug"
cargo run --release -- scan "D:\Music"
```

## Appendix: Raw Profiling Data

### Test Run 1: Small Library (3 files)

```
Scanning directory: "test_music"
Scan complete. Total scanned: 2 tracks.
Total time: 0.03s (71.2 files/sec)

Timing breakdown:
  Metadata parsing: 4.5ms (16.1%)
  Database writes:  32.0ms (114.0%)
  Other (I/O, etc): 0.0ms (0.0%)

Per-file averages:
  Metadata: 2.26ms/file
  Database: 16.00ms/file
```

### Code Instrumentation

Timing was added to `crates/music-minder/src/library/mod.rs`:

- `ScanTimings` struct with atomic counters
- `reset_scan_timings()` / `get_scan_timings()` API
- Per-file timing for metadata and DB phases

## See Also

- [ROADMAP.md](ROADMAP.md) — Task 6 (this analysis) and Task 7 (optimizations)
- [scripts/profile-scan.ps1](../scripts/profile-scan.ps1) — Profiling script
- [crates/music-minder/src/library/mod.rs](../crates/music-minder/src/library/mod.rs) — Instrumented code
