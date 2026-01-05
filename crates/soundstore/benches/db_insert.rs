/// Benchmarks for database operations and models.
///
/// These benchmarks measure performance of data structures and operations that
/// support the library indexing system. The goal is to:
///
/// 1. **Track model allocation** — How fast we create Track structs
/// 2. **Album/Artist creation** — Upsert operations used during scanning
/// 3. **String operations** — Tag data is string-heavy; ensure no unexpected overhead
///
/// These benchmarks inform scanning performance (Phase B.2) since database writes
/// are part of the critical path during library scanning.
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use soundstore::{Album, Artist, Track};

// ============================================================================
// Data Structure Creation (ALLOCATION BENCHMARKS)
// ============================================================================
//
// During library scanning, we create thousands of these objects.
// While the data structures themselves are fast, we measure allocation/clone
// overhead to understand memory pressure.

fn data_structure_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_creation");

    // ========================================================================
    // Artist Creation (simple: id + string)
    // ========================================================================
    // Artists have minimal data: just an ID and name string.
    // This is O(name.len()) due to string allocation.

    group.bench_function("artist_creation_short_name", |b| {
        b.iter(|| {
            let _artist = Artist {
                id: black_box(1),
                name: black_box("Queen".to_string()),
            };
        });
    });

    group.bench_function("artist_creation_long_name", |b| {
        b.iter(|| {
            let _artist = Artist {
                id: black_box(1),
                name: black_box(
                    "The Longest Artist Name We Could Ever Conceivably Have In A Database"
                        .to_string(),
                ),
            };
        });
    });

    // ========================================================================
    // Album Creation (id + strings + optional fields)
    // ========================================================================
    // Albums include title (required), artist_id (optional), and year (optional).
    // More fields = more allocation potential.

    group.bench_function("album_creation_minimal", |b| {
        b.iter(|| {
            let _album = Album {
                id: black_box(1),
                title: black_box("A Night at the Opera".to_string()),
                artist_id: black_box(None),
                year: black_box(None),
            };
        });
    });

    group.bench_function("album_creation_full", |b| {
        b.iter(|| {
            let _album = Album {
                id: black_box(1),
                title: black_box("Back in Black".to_string()),
                artist_id: black_box(Some(42)),
                year: black_box(Some(1980)),
            };
        });
    });

    // ========================================================================
    // Track Creation (heaviest structure)
    // ========================================================================
    // Tracks are the most complex: path string is typically 200+ chars,
    // plus many optional fields for quality tracking and metadata enrichment.
    //
    // This is important because during scanning, we create one Track per file.
    // With 10k+ tracks, allocation speed matters.

    group.bench_function("track_creation_minimal_tags", |b| {
        b.iter(|| {
            let _track = Track {
                id: black_box(1),
                title: black_box("Bohemian Rhapsody".to_string()),
                artist_id: black_box(Some(1)),
                album_id: black_box(Some(1)),
                path: black_box(
                    "/music/Queen/A Night at the Opera/01 - Bohemian Rhapsody.mp3".to_string(),
                ),
                duration: black_box(Some(354)),
                track_number: black_box(Some(1i64)),
                quality_score: black_box(None),
                quality_flags: black_box(None),
                quality_checked_at: black_box(None),
                acoustid_confidence: black_box(None),
                musicbrainz_recording_id: black_box(None),
            };
        });
    });

    group.bench_function("track_creation_with_enrichment", |b| {
        b.iter(|| {
            let _track = Track {
                id: black_box(1),
                title: black_box("Bohemian Rhapsody".to_string()),
                artist_id: black_box(Some(1)),
                album_id: black_box(Some(1)),
                path: black_box(
                    "/music/Queen/A Night at the Opera/01 - Bohemian Rhapsody.mp3".to_string(),
                ),
                duration: black_box(Some(354)),
                track_number: black_box(Some(1i64)),
                quality_score: black_box(Some(95)),
                quality_flags: black_box(Some(0b1111)), // All quality flags set
                quality_checked_at: black_box(Some("2025-01-05T12:00:00Z".to_string())),
                acoustid_confidence: black_box(Some(0.95)),
                musicbrainz_recording_id: black_box(Some(
                    "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
                )),
            };
        });
    });

    group.finish();
}

// ============================================================================
// Collection Operations (WORKLOAD PATTERNS)
// ============================================================================
//
// During scanning, we also batch-create collections of these objects.
// This measures the overhead of creating multiple objects in sequence.

fn batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    group.sample_size(50); // Smaller sample size due to larger workloads

    // ========================================================================
    // Create 100 tracks (typical for a single album's worth of data)
    // ========================================================================
    // This simulates inserting a newly scanned album's worth of tracks.

    group.throughput(Throughput::Elements(100));
    group.bench_function("create_100_tracks", |b| {
        b.iter(|| {
            let mut tracks = Vec::with_capacity(100);
            for i in 0..100 {
                tracks.push(Track {
                    id: black_box(i as i64),
                    title: black_box(format!("Track {}", i)),
                    artist_id: black_box(Some(1)),
                    album_id: black_box(Some(1)),
                    path: black_box(format!("/music/Artist/Album/{:02} - Track {}.mp3", i, i)),
                    duration: black_box(Some(180)),
                    track_number: black_box(Some(i as i64)),
                    quality_score: black_box(None),
                    quality_flags: black_box(None),
                    quality_checked_at: black_box(None),
                    acoustid_confidence: black_box(None),
                    musicbrainz_recording_id: black_box(None),
                });
            }
            black_box(tracks)
        });
    });

    // ========================================================================
    // Clone track for queue operations
    // ========================================================================
    // Tracks get cloned when added to the playback queue.
    // With 10k+ library tracks, we want clone to be cheap.

    let sample_track = Track {
        id: 1,
        title: "Test Track".to_string(),
        artist_id: Some(1),
        album_id: Some(1),
        path: "/music/Artist/Album/Track.mp3".to_string(),
        duration: Some(180),
        track_number: Some(1i64),
        quality_score: None,
        quality_flags: None,
        quality_checked_at: None,
        acoustid_confidence: None,
        musicbrainz_recording_id: None,
    };

    group.bench_function("clone_track", |b| {
        b.iter(|| {
            let _cloned = black_box(sample_track.clone());
        });
    });

    group.finish();
}

// ============================================================================
// String Operations (TAG DATA HANDLING)
// ============================================================================
//
// Audio metadata is string-heavy. We measure common string operations
// to ensure we're not doing unexpected work when parsing/storing tags.

fn string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    group.bench_function("clone_short_string", |b| {
        let s = black_box("Queen".to_string());
        b.iter(|| s.clone());
    });

    group.bench_function("clone_long_path", |b| {
        let s = black_box(
            "/music/Very/Long/Path/To/Audio/File/With/Many/Directories/Track Title.mp3".to_string(),
        );
        b.iter(|| s.clone());
    });

    group.bench_function("format_string_with_numbers", |b| {
        b.iter(|| {
            let _s = format!(
                "Track {} - {} ({})",
                black_box(42),
                black_box("Song Title"),
                black_box(180)
            );
        });
    });

    group.finish();
}

// ============================================================================
// Register all benchmark groups
// ============================================================================

criterion_group!(
    benches,
    data_structure_creation,
    batch_operations,
    string_operations
);
criterion_main!(benches);
