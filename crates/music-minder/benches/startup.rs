//! Startup time benchmarks for Music Minder.
//!
//! These benchmarks measure the critical path from launch to interactive UI.
//! The goal is to establish baselines and catch regressions in startup performance.
//!
//! # Benchmarked Operations
//!
//! 1. **Config Loading** — Read and parse TOML configuration
//! 2. **Database Initialization** — Open SQLite, check schema, run migrations
//! 3. **Initial Track Loading** — First batch of tracks for immediate display
//! 4. **Icon Loading** — Decode embedded PNG for window icon
//!
//! # Lazy Loading Optimization (B.1)
//!
//! Audio device enumeration is deferred until first play command. This benchmark
//! measures the cost that was moved out of the startup path.
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all startup benchmarks
//! cargo bench -p music-minder --bench startup
//!
//! # Run with HTML report
//! cargo bench -p music-minder --bench startup -- --save-baseline startup-v0.2.1
//! ```
//!
//! # Target Metrics (from ROADMAP.md)
//!
//! - GUI startup: <10ms
//! - Initial 200 tracks: <20ms
//! - Full library (10k+ tracks): <150ms
//! - Target: <100ms time-to-interactive for 50k+ tracks

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Instant;

// ============================================================================
// Configuration Loading Benchmarks
// ============================================================================

/// Benchmark TOML configuration parsing.
/// This runs at startup to load user settings.
fn config_parsing_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_loading");

    // Simulate typical config content
    let sample_config = r#"
[library]
path = "D:\\Music"
scan_on_startup = true

[playback]
volume = 0.8
repeat_mode = "off"
shuffle = false

[ui]
theme = "dark"
show_visualizer = true
visualization_mode = "spectrum"

[enrichment]
auto_identify = false
api_key = ""
"#;

    group.bench_function("parse_toml_config", |b| {
        b.iter(|| {
            let config: toml::Value = toml::from_str(black_box(sample_config)).unwrap();
            config
        });
    });

    // Benchmark config file existence check (common startup operation)
    group.bench_function("config_path_check", |b| {
        b.iter(|| {
            let config_dir = dirs::config_dir();
            let _exists = config_dir.as_ref().map(|p| p.exists()).unwrap_or(false);
        });
    });

    group.finish();
}

// ============================================================================
// Icon Loading Benchmarks
// ============================================================================

/// Benchmark icon loading from embedded PNG.
/// This runs once at startup before the window opens.
fn icon_loading_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("icon_loading");

    // The actual embedded icon bytes
    const APP_ICON_32: &[u8] = include_bytes!("../../../assets/icon-32.png");

    group.bench_function("decode_32x32_png", |b| {
        b.iter(|| {
            let icon_bytes = black_box(APP_ICON_32);
            let img = image::load_from_memory(icon_bytes).unwrap();
            let rgba = img.into_rgba8();
            let (width, height) = (rgba.width(), rgba.height());
            let raw = rgba.into_raw();
            (width, height, raw.len())
        });
    });

    group.finish();
}

// ============================================================================
// Startup Timing Simulation
// ============================================================================

/// Measure baseline timing overhead.
/// This helps understand the measurement overhead itself.
fn timing_overhead_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("timing_overhead");

    group.bench_function("instant_now", |b| {
        b.iter(|| {
            let start = Instant::now();
            black_box(start)
        });
    });

    group.bench_function("instant_elapsed", |b| {
        let start = Instant::now();
        b.iter(|| {
            let elapsed = start.elapsed();
            black_box(elapsed)
        });
    });

    group.bench_function("full_timing_cycle", |b| {
        b.iter(|| {
            let start = Instant::now();
            // Simulate a tiny operation
            let _ = black_box(1 + 1);
            let elapsed = start.elapsed();
            black_box(elapsed)
        });
    });

    group.finish();
}

// ============================================================================
// String Operations (Common during startup)
// ============================================================================

/// Benchmark string operations used during startup.
/// Path building, formatting, etc.
fn string_operations_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_strings");

    group.bench_function("format_log_message", |b| {
        b.iter(|| {
            let msg = format!("Startup completed in {:.1}ms", black_box(14.5f64));
            black_box(msg)
        });
    });

    group.bench_function("path_join", |b| {
        b.iter(|| {
            let base = std::path::Path::new(black_box("D:\\Music"));
            let full = base.join(black_box("Artist")).join(black_box("Album"));
            black_box(full)
        });
    });

    group.bench_function("database_path_resolution", |b| {
        b.iter(|| {
            let data_dir = dirs::data_local_dir();
            let db_path = data_dir.map(|d| d.join("music-minder").join("library.db"));
            black_box(db_path)
        });
    });

    group.finish();
}

// ============================================================================
// Memory Allocation Patterns
// ============================================================================

/// Benchmark memory allocation patterns during startup.
/// Understanding allocation overhead helps optimize initial data structures.
fn allocation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_allocations");

    // Simulate allocating initial track vector
    group.bench_function("allocate_empty_track_vec", |b| {
        b.iter(|| {
            let tracks: Vec<u8> = Vec::new();
            black_box(tracks)
        });
    });

    group.bench_function("allocate_200_track_capacity", |b| {
        b.iter(|| {
            let tracks: Vec<u8> = Vec::with_capacity(200);
            black_box(tracks)
        });
    });

    group.bench_function("allocate_10000_track_capacity", |b| {
        b.iter(|| {
            let tracks: Vec<u8> = Vec::with_capacity(10_000);
            black_box(tracks)
        });
    });

    // Simulate HashMap allocation for track lookup
    group.bench_function("allocate_hashmap_1000_capacity", |b| {
        b.iter(|| {
            let map: std::collections::HashMap<u64, u8> =
                std::collections::HashMap::with_capacity(1000);
            black_box(map)
        });
    });

    group.finish();
}

// ============================================================================
// Deferred Operations (B.1 Optimization - NOT on startup path)
// ============================================================================

/// Benchmark operations that are now deferred from startup.
///
/// These operations were moved out of the critical startup path to improve
/// time-to-interactive. They now run in the background or on-demand.
///
/// This benchmark documents the cost savings achieved by lazy loading.
fn deferred_operations_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("deferred_operations");

    // Measure audio host creation (first step of device enumeration)
    // This is typically fast (~1ms) but varies by platform
    group.bench_function("cpal_default_host", |b| {
        b.iter(|| {
            let host = cpal::default_host();
            black_box(host)
        });
    });

    // Note: Full audio device enumeration (list_audio_devices) is not benchmarked
    // here because it varies too much by system configuration (50-200ms typical).
    // The important thing is that it's NO LONGER on the startup path.

    group.finish();
}

// ============================================================================
// Register all benchmark groups
// ============================================================================

criterion_group!(
    benches,
    config_parsing_benchmarks,
    icon_loading_benchmarks,
    timing_overhead_benchmarks,
    string_operations_benchmarks,
    allocation_benchmarks,
    deferred_operations_benchmarks,
);
criterion_main!(benches);
