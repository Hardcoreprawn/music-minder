/// Benchmarks for audio player components.
/// Measures basic operations in the audio pipeline.
use criterion::{Criterion, criterion_group, criterion_main};

fn player_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio");

    group.bench_function("time_calculation_0s", |b| {
        b.iter(|| {
            let secs: u64 = 0;
            let _mins = secs / 60;
            let _secs_rem = secs % 60;
        });
    });

    group.bench_function("time_calculation_180s", |b| {
        b.iter(|| {
            let secs: u64 = 180;
            let _mins = secs / 60;
            let _secs_rem = secs % 60;
        });
    });

    group.bench_function("time_calculation_3600s", |b| {
        b.iter(|| {
            let secs: u64 = 3600;
            let _hours = secs / 3600;
            let _mins = (secs % 3600) / 60;
            let _secs_rem = secs % 60;
        });
    });

    group.finish();
}

criterion_group!(benches, player_benchmarks);
criterion_main!(benches);
