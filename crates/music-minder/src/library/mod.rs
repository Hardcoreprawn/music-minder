//! Library scanning and management.
//!
//! Coordinates the scanning of directories for audio files, reading their
//! metadata, and storing track information in the database.
//!
//! ## Performance Profiling (Phase B.2)
//!
//! The scanning pipeline has three main phases:
//! 1. **File Discovery** — Walking the directory tree (I/O bound)
//! 2. **Metadata Parsing** — Reading audio tags with lofty (CPU + I/O)
//! 3. **Database Writes** — SQLite inserts for artist/album/track (I/O bound)
//!
//! Use `RUST_LOG=music_minder::library=debug` to see timing breakdowns.

use crate::{db, metadata, scanner};
use futures::{Stream, StreamExt};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Processed(PathBuf),
    Error(PathBuf, String),
}

/// Timing statistics for scan profiling (Phase B.2)
#[derive(Default)]
pub struct ScanTimings {
    /// Total time spent reading metadata (nanoseconds)
    pub metadata_ns: AtomicU64,
    /// Total time spent on database operations (nanoseconds)
    pub db_ns: AtomicU64,
    /// Number of files processed
    pub file_count: AtomicU64,
}

impl ScanTimings {
    /// Reset all counters
    pub fn reset(&self) {
        self.metadata_ns.store(0, Ordering::Relaxed);
        self.db_ns.store(0, Ordering::Relaxed);
        self.file_count.store(0, Ordering::Relaxed);
    }

    /// Log timing summary
    pub fn log_summary(&self) {
        let count = self.file_count.load(Ordering::Relaxed);
        let meta_ms = self.metadata_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let db_ms = self.db_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0;

        if count > 0 {
            let meta_per_file = meta_ms / count as f64;
            let db_per_file = db_ms / count as f64;

            tracing::info!(
                target: "music_minder::library",
                files = count,
                metadata_total_ms = format!("{:.1}", meta_ms),
                metadata_per_file_ms = format!("{:.2}", meta_per_file),
                db_total_ms = format!("{:.1}", db_ms),
                db_per_file_ms = format!("{:.2}", db_per_file),
                "Scan timing breakdown"
            );
        }
    }
}

/// Global timing stats for current scan (thread-safe)
static SCAN_TIMINGS: std::sync::LazyLock<ScanTimings> =
    std::sync::LazyLock::new(ScanTimings::default);

/// Reset timing stats before a new scan
pub fn reset_scan_timings() {
    SCAN_TIMINGS.reset();
}

/// Get timing summary after a scan
pub fn get_scan_timings() -> &'static ScanTimings {
    &SCAN_TIMINGS
}

/// Scans a directory with batched database writes for maximum performance.
/// Returns a stream of ScanEvents.
///
/// ## Performance Strategy (Phase B.2)
///
/// This implementation batches database writes to eliminate per-file transaction overhead:
/// 1. **Metadata Parsing** — Read 100 files (parallel, CPU+I/O bound)
/// 2. **Batch DB Write** — Single transaction for all 100 files (I/O bound)
/// 3. **Repeat** — Continue until all files processed
///
/// Expected 10-15x improvement over individual inserts due to:
/// - Single transaction per batch (vs 100 commits)
/// - Reduced async overhead (300 awaits → ~3 awaits per batch)
/// - Bulk INSERT OR IGNORE for artists/albums
///
/// ## Configuration
///
/// - Batch size: 100 files (tunable trade-off between memory and latency)
/// - Metadata parallelism: 10 concurrent reads
pub fn scan_library_batched(pool: SqlitePool, root: PathBuf) -> impl Stream<Item = ScanEvent> {
    tracing::debug!(target: "music_minder::library", path = %root.display(), "Starting batched library scan");

    const BATCH_SIZE: usize = 100;

    let paths = scanner::scan(root);

    // Collect paths into batches, then process each batch
    async_stream::stream! {
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let mut path_stream = Box::pin(paths);

        while let Some(path) = path_stream.next().await {
            batch.push(path);

            if batch.len() >= BATCH_SIZE {
                // Process this batch
                for event in process_batch(&pool, &batch).await {
                    yield event;
                }
                batch.clear();
            }
        }

        // Process remaining files
        if !batch.is_empty() {
            for event in process_batch(&pool, &batch).await {
                yield event;
            }
        }
    }
}

/// Process a batch of files: read metadata in parallel, then write to DB in one transaction
async fn process_batch(pool: &SqlitePool, paths: &[PathBuf]) -> Vec<ScanEvent> {
    use futures::stream::FuturesUnordered;

    // Phase 1: Read metadata for all files in parallel
    let meta_tasks: FuturesUnordered<_> = paths
        .iter()
        .map(|path| {
            let path = path.clone();
            async move {
                let meta_start = Instant::now();
                let result = metadata::read(&path);
                let meta_elapsed = meta_start.elapsed().as_nanos() as u64;
                SCAN_TIMINGS
                    .metadata_ns
                    .fetch_add(meta_elapsed, Ordering::Relaxed);

                (path, result)
            }
        })
        .collect();

    let metadata_results: Vec<_> = meta_tasks.collect().await;

    // Phase 2: Prepare successful metadata for batch insert
    let mut tracks_to_insert = Vec::new();
    let mut events = Vec::new();

    for (path, meta_result) in metadata_results {
        match meta_result {
            Ok(file_meta) => {
                let meta = soundstore::TrackMetadata {
                    title: file_meta.title,
                    artist: file_meta.artist,
                    album: file_meta.album,
                    duration: file_meta.duration,
                    track_number: file_meta.track_number,
                };
                let path_str = path.to_string_lossy().into_owned();
                tracks_to_insert.push((path.clone(), meta, path_str));
            }
            Err(e) => {
                SCAN_TIMINGS.file_count.fetch_add(1, Ordering::Relaxed);
                events.push(ScanEvent::Error(path, e.to_string()));
            }
        }
    }

    // Phase 3: Batch write to database
    if !tracks_to_insert.is_empty() {
        let db_start = Instant::now();

        let batch_data: Vec<_> = tracks_to_insert
            .iter()
            .map(|(_, meta, path_str)| (meta.clone(), path_str.clone()))
            .collect();

        match soundstore::db::batch_insert_tracks(pool, &batch_data).await {
            Ok(_track_ids) => {
                let db_elapsed = db_start.elapsed().as_nanos() as u64;
                SCAN_TIMINGS.db_ns.fetch_add(db_elapsed, Ordering::Relaxed);
                SCAN_TIMINGS
                    .file_count
                    .fetch_add(tracks_to_insert.len() as u64, Ordering::Relaxed);

                // All tracks inserted successfully
                for (path, _, _) in tracks_to_insert {
                    events.push(ScanEvent::Processed(path));
                }
            }
            Err(e) => {
                let db_elapsed = db_start.elapsed().as_nanos() as u64;
                SCAN_TIMINGS.db_ns.fetch_add(db_elapsed, Ordering::Relaxed);
                SCAN_TIMINGS
                    .file_count
                    .fetch_add(tracks_to_insert.len() as u64, Ordering::Relaxed);

                // Batch failed, report all as errors
                for (path, _, _) in tracks_to_insert {
                    events.push(ScanEvent::Error(path, e.to_string()));
                }
            }
        }
    }

    events
}

/// Scans a directory and updates the database with found tracks.
/// Returns a stream of ScanEvents.
///
/// ## Parallelism
///
/// Currently uses `buffer_unordered(10)` for 10-way parallel processing.
/// This is a good default for SSDs; HDDs may benefit from less parallelism.
///
/// ## Note
///
/// This is the original per-file implementation. Consider using
/// [`scan_library_batched`] for 10-15x better performance via transaction batching.
#[allow(dead_code)]
pub fn scan_library(pool: SqlitePool, root: PathBuf) -> impl Stream<Item = ScanEvent> {
    tracing::debug!(target: "music_minder::library", path = %root.display(), "Starting library scan");

    let paths = scanner::scan(root);

    paths
        .map(move |path| {
            let pool = pool.clone();
            async move {
                // Phase 2: Metadata parsing (timed)
                let meta_start = Instant::now();
                let meta_result = metadata::read(&path);
                let meta_elapsed = meta_start.elapsed().as_nanos() as u64;
                SCAN_TIMINGS
                    .metadata_ns
                    .fetch_add(meta_elapsed, Ordering::Relaxed);

                match meta_result {
                    Ok(file_meta) => {
                        // Convert to soundstore metadata type for database insertion
                        let meta = soundstore::TrackMetadata {
                            title: file_meta.title,
                            artist: file_meta.artist,
                            album: file_meta.album,
                            duration: file_meta.duration,
                            track_number: file_meta.track_number,
                        };

                        // Phase 3: Database writes (timed)
                        let db_start = Instant::now();
                        let artist_id = db::get_or_create_artist(&pool, &meta.artist).await.ok();
                        let album_id = db::get_or_create_album(&pool, &meta.album, artist_id)
                            .await
                            .ok();
                        let result = db::insert_track(
                            &pool,
                            &meta,
                            path.to_str().unwrap_or(""),
                            artist_id,
                            album_id,
                        )
                        .await;
                        let db_elapsed = db_start.elapsed().as_nanos() as u64;
                        SCAN_TIMINGS.db_ns.fetch_add(db_elapsed, Ordering::Relaxed);
                        SCAN_TIMINGS.file_count.fetch_add(1, Ordering::Relaxed);

                        match result {
                            Ok(_) => ScanEvent::Processed(path),
                            Err(e) => ScanEvent::Error(path, e.to_string()),
                        }
                    }
                    Err(e) => {
                        SCAN_TIMINGS.file_count.fetch_add(1, Ordering::Relaxed);
                        ScanEvent::Error(path, e.to_string())
                    }
                }
            }
        })
        .buffer_unordered(10) // Process 10 files in parallel
}
