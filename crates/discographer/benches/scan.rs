/// Benchmarks for metadata extraction and file scanning operations.
///
/// These benchmarks measure the cost of scanning a directory for audio files
/// and extracting their metadata. Key measurements:
///
/// 1. **Metadata structure creation** — How fast we build TrackMetadata objects
/// 2. **String operations** — Normalizing artist/album/title tags
/// 3. **Path construction** — Building canonical file paths during scan
///
/// These feed into the overall scanning performance goal (Phase B.2):
/// Currently: ~200-500 files/second
/// Target: 1000+ files/second
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use discographer::TrackMetadata;

// ============================================================================
// Metadata Structure Creation (SCANNER HOT PATH)
// ============================================================================
//
// When scanning a directory, we extract metadata from each file and create
// a TrackMetadata object. This is called once per audio file found.
//
// With a library of 10k+ tracks, small optimizations here compound.

fn metadata_creation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_creation");

    // ========================================================================
    // Minimal metadata (just required fields)
    // ========================================================================
    // This is the fast path: if the scanner can't extract rich metadata,
    // it falls back to minimal info (filename-based).

    group.bench_function("minimal_metadata", |b| {
        b.iter(|| {
            let _metadata = TrackMetadata {
                title: black_box("Track Title".to_string()),
                artist: black_box("Artist Name".to_string()),
                album: black_box("Album Title".to_string()),
                duration: black_box(180000), // milliseconds
                track_number: black_box(None),
            };
        });
    });

    // ========================================================================
    // Complete metadata (all fields populated)
    // ========================================================================
    // This is the enriched path: we got good data from tags.

    group.bench_function("complete_metadata", |b| {
        b.iter(|| {
            let _metadata = TrackMetadata {
                title: black_box("Bohemian Rhapsody".to_string()),
                artist: black_box("Queen".to_string()),
                album: black_box("A Night at the Opera".to_string()),
                duration: black_box(354000), // 354 seconds in ms
                track_number: black_box(Some(1)),
            };
        });
    });

    // ========================================================================
    // Realistic metadata with tag variations
    // ========================================================================
    // Real-world files have inconsistent metadata:
    // - Different length artist names (1 char → 100+ chars)
    // - Album art, compilation flags, genre, year (all optional)
    // - Unicode in various normalizations

    group.bench_function("metadata_with_unicode", |b| {
        b.iter(|| {
            let _metadata = TrackMetadata {
                title: black_box("Straße (Street)".to_string()),
                artist: black_box("Künstler (Artist) 藝人".to_string()),
                album: black_box("Альбом (Album) 專輯".to_string()),
                duration: black_box(240000),
                track_number: black_box(Some(5)),
            };
        });
    });

    group.finish();
}

// ============================================================================
// Tag Parsing and Normalization (METADATA EXTRACTION)
// ============================================================================
//
// Real metadata comes from file tags (ID3, Vorbis, FLAC, etc.).
// The scanner extracts these and normalizes them.

fn tag_normalization_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("tag_normalization");

    // ========================================================================
    // Trim whitespace (very common in raw tags)
    // ========================================================================

    group.bench_function("normalize_trim_short", |b| {
        let tag = black_box("  Artist Name  ".to_string());
        b.iter(|| tag.trim().to_string());
    });

    group.bench_function("normalize_trim_long", |b| {
        let tag =
            black_box("   The Longest Artist Name With Lots Of Spaces All Around   ".to_string());
        b.iter(|| tag.trim().to_string());
    });

    // ========================================================================
    // Case normalization (lowercase for comparison)
    // ========================================================================

    group.bench_function("normalize_to_lowercase", |b| {
        let tag = black_box("QUEEN".to_string());
        b.iter(|| tag.to_lowercase());
    });

    // ========================================================================
    // Extract track number from various formats
    // ========================================================================
    // Real files have inconsistent track numbering:
    // "1", "01", "1/12", "Track 1"

    group.bench_function("parse_track_number_simple", |b| {
        let tag = black_box("1".to_string());
        b.iter(|| tag.parse::<u32>().ok());
    });

    group.bench_function("parse_track_number_with_total", |b| {
        let tag = black_box("1/12".to_string());
        b.iter(|| tag.split('/').next().and_then(|s| s.parse::<u32>().ok()));
    });

    group.finish();
}

// ============================================================================
// Path Construction (FILE SYSTEM OPERATIONS)
// ============================================================================
//
// The scanner builds canonical file paths for each track.
// This involves string concatenation and path normalization.

fn path_construction_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_operations");
    group.sample_size(100);

    // ========================================================================
    // Build path from components (directory + filename)
    // ========================================================================

    group.bench_function("build_short_path", |b| {
        b.iter(|| {
            let dir = black_box("/music");
            let filename = black_box("track.mp3");
            let _path = format!("{}/{}", dir, filename);
        });
    });

    group.bench_function("build_long_path", |b| {
        b.iter(|| {
            let dir = black_box("/music/Classical/Composers/Beethoven");
            let filename = black_box("Symphony No. 9 in D minor, Op. 125 'Choral'.flac");
            let _path = format!("{}/{}", dir, filename);
        });
    });

    // ========================================================================
    // Canonicalize paths (resolve symlinks, normalize separators)
    // ========================================================================
    // This is more expensive but sometimes necessary for deduplication.

    group.bench_function("path_canonicalization", |b| {
        b.iter(|| {
            use std::path::Path;
            let p = Path::new("/music/././Artist/../Artist/Album/track.mp3");
            let _normalized = black_box(p);
        });
    });

    group.finish();
}

// ============================================================================
// Batch Metadata Creation (SCANNING WORKLOAD)
// ============================================================================
//
// During a full library scan, we might create metadata for 100+ files
// from a single directory batch. We measure this combined workload.

fn batch_scanning_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_scanning");
    group.sample_size(50);

    // ========================================================================
    // Simulate scanning 50 files from one album
    // ========================================================================
    // This is realistic: albums can have 10-50+ tracks.

    group.throughput(Throughput::Elements(50));
    group.bench_function("scan_album_50_tracks", |b| {
        b.iter(|| {
            let mut metadata_list = Vec::with_capacity(50);
            for i in 0..50 {
                metadata_list.push(TrackMetadata {
                    title: black_box(format!("Track {}", i)),
                    artist: black_box("Test Artist".to_string()),
                    album: black_box("Test Album".to_string()),
                    duration: black_box(180000u64 + (i as u64 * 1000)),
                    track_number: black_box(Some(i as u32)),
                });
            }
            black_box(metadata_list)
        });
    });

    group.finish();
}

// ============================================================================
// Register all benchmark groups
// ============================================================================

criterion_group!(
    benches,
    metadata_creation_benchmarks,
    tag_normalization_benchmarks,
    path_construction_benchmarks,
    batch_scanning_benchmarks
);
criterion_main!(benches);
