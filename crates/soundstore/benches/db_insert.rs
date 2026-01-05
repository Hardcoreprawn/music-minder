/// Benchmarks for database models and operations.
/// Measures basic data structure performance.
use criterion::{Criterion, criterion_group, criterion_main};
use soundstore::{Album, Artist, Track};

fn db_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("database");

    group.bench_function("artist_creation", |b| {
        b.iter(|| {
            let _artist = Artist {
                id: 1,
                name: "Test Artist".to_string(),
            };
        });
    });

    group.bench_function("album_creation", |b| {
        b.iter(|| {
            let _album = Album {
                id: 1,
                title: "Test Album".to_string(),
                artist_id: Some(1),
                year: Some(2023),
            };
        });
    });

    group.bench_function("track_creation", |b| {
        b.iter(|| {
            let _track = Track {
                id: 1,
                title: "Test Track".to_string(),
                artist_id: Some(1),
                album_id: Some(1),
                path: "/test/track.mp3".to_string(),
                duration: Some(180),
                track_number: Some(1),
                quality_score: None,
                quality_flags: None,
                quality_checked_at: None,
                acoustid_confidence: None,
                musicbrainz_recording_id: None,
            };
        });
    });

    group.finish();
}

criterion_group!(benches, db_benchmarks);
criterion_main!(benches);
