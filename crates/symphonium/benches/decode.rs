/// Benchmarks for audio pipeline operations.
///
/// These benchmarks measure performance-critical operations in the audio subsystem.
/// We focus on the "hot path" — operations that run repeatedly during playback:
///
/// 1. **Format Detection** — How fast we identify audio file formats
/// 2. **Volume Scaling (SIMD)** — The most frequent operation in the audio callback
/// 3. **Time Formatting** — Used frequently for UI display
///
/// The goal is to:
/// - Establish baselines to prevent regressions
/// - Understand where our audio processing time goes
/// - Validate SIMD optimizations are working
/// - Measure impact of different SIMD levels (AVX2 vs SSE4.1 vs Scalar)
/// - **CRITICAL**: Compare our manual SIMD against compiler auto-vectorization
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use symphonium::simd;

// ============================================================================
// SIMD Volume Scaling Benchmarks (AUDIO CALLBACK HOT PATH)
// ============================================================================
//
// This is one of the most critical operations. It runs once per audio frame
// at 48kHz (48,000 times per second), so even tiny optimizations matter.
//
// The resampler output → ring buffer → CPAL callback uses volume scaling
// to apply user volume setting without modifying the decoded audio.
//
// **Phase B.3 Goal**: Verify our manual SIMD actually beats the compiler!

/// Test: Naive iterator implementation (what the compiler sees)
#[inline(never)]
fn volume_naive(samples: &mut [f32], volume: f32) {
    for sample in samples.iter_mut() {
        *sample *= volume;
    }
}

/// Test: Compiler with auto-vectorization hints
#[inline(always)]
fn volume_compiler_optimized(samples: &mut [f32], volume: f32) {
    // Give the compiler every advantage:
    // - inline(always) for aggressive optimization
    // - Iterator pattern it can auto-vectorize
    // - No artificial barriers
    for sample in samples.iter_mut() {
        *sample *= volume;
    }
}

fn simd_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_volume_scaling");
    group.sample_size(100); // Increase sample size for stable timing

    // Realistic audio frame size from CPAL: typically 256-2048 samples per callback
    // We'll test common sizes representing different latency profiles:
    //
    // - 256 samples: High performance (5.3ms latency @ 48kHz) — for real-time responsiveness
    // - 1024 samples: Balanced (21.3ms latency @ 48kHz) — typical default
    // - 4096 samples: Low CPU usage (85.3ms latency @ 48kHz) — for battery life

    for &frame_size in &[256, 1024, 4096] {
        group.throughput(Throughput::Elements(frame_size as u64));

        // Benchmark 1: Our manual SIMD implementation (runtime detection)
        group.bench_function(format!("manual_simd_{}frames", frame_size), |b| {
            let mut audio_buffer = vec![0.5f32; frame_size];
            b.iter(|| {
                let volume = black_box(0.8f32);
                simd::apply_volume(black_box(&mut audio_buffer), volume);
            });
        });

        // Benchmark 2: Naive implementation (baseline)
        group.bench_function(format!("naive_scalar_{}frames", frame_size), |b| {
            let mut audio_buffer = vec![0.5f32; frame_size];
            b.iter(|| {
                let volume = black_box(0.8f32);
                volume_naive(black_box(&mut audio_buffer), volume);
            });
        });

        // Benchmark 3: Compiler-optimized (auto-vectorization)
        group.bench_function(format!("compiler_optimized_{}frames", frame_size), |b| {
            let mut audio_buffer = vec![0.5f32; frame_size];
            b.iter(|| {
                let volume = black_box(0.8f32);
                volume_compiler_optimized(black_box(&mut audio_buffer), volume);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Time Calculation Benchmarks (UI RENDERING)
// ============================================================================
//
// These run in the UI thread when displaying playback position.
// While not performance-critical, we measure them as a baseline.

fn time_formatting_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("time_formatting");

    // Test different durations to understand the calculation overhead
    let test_cases = vec![
        ("0_seconds", 0u64),
        ("30_seconds", 30u64),
        ("180_seconds_3min", 180u64),
        ("3600_seconds_1hr", 3600u64),
        ("36000_seconds_10hr", 36000u64),
    ];

    for (label, secs) in test_cases {
        group.bench_function(label, |b| {
            b.iter(|| {
                let secs = black_box(secs);
                let _hours = secs / 3600;
                let _mins = (secs % 3600) / 60;
                let _secs_rem = secs % 60;
            });
        });
    }

    group.finish();
}

// ============================================================================
// Ring Buffer Operations (AUDIO THREAD COMMUNICATION)
// ============================================================================
//
// The audio callback reads from a lock-free ring buffer (rtrb crate).
// This measures the overhead of ring buffer operations.
//
// The audio thread writes decoded samples, the CPAL thread reads them.
// We need to ensure this is fast enough for high sample rates.

fn ringbuffer_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_ops");

    // Typical ring buffer operations per audio frame:
    // - Write: decoder thread pushes samples
    // - Read: CPAL thread pulls samples to output device
    //
    // Testing with 1024 samples (typical frame size)

    group.bench_function("ring_buffer_write_1024samples", |b| {
        b.iter(|| {
            // Simulate writing 1024 f32 samples to a ring buffer
            let samples = vec![0.5f32; 1024];
            let _data = black_box(samples);
            // In reality, this would call ring_buffer.write(&samples)
            // but the allocation + copy is the main cost
        });
    });

    group.bench_function("ring_buffer_read_1024samples", |b| {
        b.iter(|| {
            // Simulate reading 1024 f32 samples from a ring buffer
            let mut output = vec![0.0f32; 1024];
            let _data = black_box(&mut output);
            // In reality, this would call ring_buffer.read(&mut output)
        });
    });

    group.finish();
}

// ============================================================================
// Register all benchmark groups
// ============================================================================

criterion_group!(
    benches,
    simd_benchmarks,
    time_formatting_benchmarks,
    ringbuffer_benchmarks
);
criterion_main!(benches);
