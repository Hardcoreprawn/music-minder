//! Async streams for background operations (scanning, preview generation).

use super::messages::Message;
use crate::{db, library, metadata, organizer, scanner};
use futures::StreamExt;
use rayon::prelude::*;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

/// Create a stream that scans a library directory and emits scan events
pub fn scan_stream(pool: SqlitePool, path: PathBuf) -> impl futures::Stream<Item = Message> {
    library::scan_library(pool, path)
        .map(Message::ScanEventReceived)
        .chain(futures::stream::once(async { Message::ScanFinished }))
}

/// Create a stream that generates organize previews in batches
/// Uses rayon for parallel file existence checks within each batch
pub fn preview_stream(
    pool: SqlitePool,
    pattern: String,
    destination: PathBuf,
) -> impl futures::Stream<Item = Message> {
    futures::stream::unfold(
        PreviewStreamState::Init {
            pool,
            pattern,
            destination,
        },
        |state| async move {
            match state {
                PreviewStreamState::Init {
                    pool,
                    pattern,
                    destination,
                } => {
                    // Load tracks from DB
                    let tracks = match db::get_all_tracks_with_metadata(&pool).await {
                        Ok(t) => t,
                        Err(_) => {
                            return Some((
                                Message::OrganizePreviewComplete,
                                PreviewStreamState::Done,
                            ));
                        }
                    };
                    eprintln!("[Preview] Loaded {} tracks from DB", tracks.len());

                    // Use Arc to avoid cloning the full track list on each iteration
                    let tracks = Arc::new(tracks);

                    // Pre-allocate preview vector capacity hint
                    // Send empty batch to show we started
                    Some((
                        Message::OrganizePreviewBatch(Vec::with_capacity(0)),
                        PreviewStreamState::Processing {
                            tracks,
                            index: 0,
                            pattern,
                            destination,
                            batch_size: 500, // Larger batches for rayon efficiency
                        },
                    ))
                }
                PreviewStreamState::Processing {
                    tracks,
                    index,
                    pattern,
                    destination,
                    batch_size,
                } => {
                    if index >= tracks.len() {
                        eprintln!("[Preview] Complete - processed {} tracks", tracks.len());
                        return Some((Message::OrganizePreviewComplete, PreviewStreamState::Done));
                    }

                    // Process a batch of tracks with parallel file exists checks via rayon
                    let end = (index + batch_size).min(tracks.len());
                    let batch_tracks: Vec<_> = tracks[index..end].to_vec();
                    let pattern_clone = pattern.clone();
                    let dest_clone = destination.clone();

                    // Do file checks in blocking task using rayon for parallelism
                    let batch_previews = tokio::task::spawn_blocking(move || {
                        // Parallel iteration with rayon - checks file existence concurrently
                        batch_tracks
                            .par_iter()
                            .filter_map(|track| {
                                let source = PathBuf::from(&track.path);
                                // File existence check - now runs in parallel
                                if !source.exists() {
                                    return None;
                                }
                                let meta = metadata::TrackMetadata {
                                    title: track.title.clone(),
                                    artist: track.artist_name.clone(),
                                    album: track.album_name.clone(),
                                    duration: track.duration.unwrap_or(0) as u64,
                                    track_number: track.track_number.map(|n| n as u32),
                                };
                                Some(organizer::preview_organize(
                                    &source,
                                    &meta,
                                    &pattern_clone,
                                    &dest_clone,
                                    track.id,
                                ))
                            })
                            .collect::<Vec<_>>()
                    })
                    .await
                    .unwrap_or_default();

                    Some((
                        Message::OrganizePreviewBatch(batch_previews),
                        PreviewStreamState::Processing {
                            tracks,
                            index: end,
                            pattern,
                            destination,
                            batch_size,
                        },
                    ))
                }
                PreviewStreamState::Done => None,
            }
        },
    )
}

/// Internal state machine for preview streaming
enum PreviewStreamState {
    Init {
        pool: SqlitePool,
        pattern: String,
        destination: PathBuf,
    },
    Processing {
        tracks: Arc<Vec<db::TrackWithMetadata>>,
        index: usize,
        pattern: String,
        destination: PathBuf,
        batch_size: usize,
    },
    Done,
}

/// Create a stream that watches a directory for file changes.
///
/// Emits `WatcherEvent` messages whenever audio files are created,
/// modified, or removed in the watched directories.
///
/// Uses `tokio::sync::mpsc` with async `.recv().await` to avoid blocking
/// Iced's cooperative async scheduler. This allows other subscriptions
/// (like `PlayerTick`) to continue firing normally.
pub fn watcher_stream(watch_paths: Vec<PathBuf>) -> impl futures::Stream<Item = Message> {
    futures::stream::unfold(
        WatcherStreamState::Init { watch_paths },
        |state| async move {
            match state {
                WatcherStreamState::Init { watch_paths } => {
                    // Create the file watcher with async channel
                    match scanner::FileWatcher::new_async(watch_paths.clone()) {
                        Ok((watcher, rx)) => {
                            tracing::info!(target: "ui::watcher", paths = ?watch_paths, "File watcher started (async)");
                            Some((
                                Message::WatcherStarted,
                                WatcherStreamState::Running {
                                    _watcher: watcher,
                                    rx,
                                },
                            ))
                        }
                        Err(e) => {
                            tracing::error!(target: "ui::watcher", error = %e, "Failed to start file watcher");
                            Some((
                                Message::WatcherEvent(scanner::WatchEvent::Error(e.to_string())),
                                WatcherStreamState::Done,
                            ))
                        }
                    }
                }
                WatcherStreamState::Running { _watcher, mut rx } => {
                    // Non-blocking async receive - yields to other tasks while waiting
                    match rx.recv().await {
                        Some(event) => Some((
                            Message::WatcherEvent(event),
                            WatcherStreamState::Running { _watcher, rx },
                        )),
                        None => {
                            // Channel closed (watcher dropped)
                            tracing::warn!(target: "ui::watcher", "File watcher channel closed");
                            Some((Message::WatcherStopped, WatcherStreamState::Done))
                        }
                    }
                }
                WatcherStreamState::Done => None,
            }
        },
    )
}

/// Internal state machine for watcher streaming
enum WatcherStreamState {
    Init {
        watch_paths: Vec<PathBuf>,
    },
    Running {
        _watcher: scanner::FileWatcher,
        rx: tokio::sync::mpsc::Receiver<scanner::WatchEvent>,
    },
    Done,
}
