/// Benchmarks for metadata operations.
/// Measures basic metadata structure creation and manipulation.
use criterion::{Criterion, criterion_group, criterion_main};
use discographer::TrackMetadata;

fn metadata_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata");

    group.bench_function("track_metadata_creation", |b| {
        b.iter(|| {
            let _metadata = TrackMetadata {
                title: "Test Track".to_string(),
                artist: "Test Artist".to_string(),
                album: "Test Album".to_string(),
                duration: 180000,
                track_number: Some(1),
            };
        });
    });

    group.finish();
}

criterion_group!(benches, metadata_benchmarks);
criterion_main!(benches);
