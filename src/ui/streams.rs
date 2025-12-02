//! Async streams for background operations (scanning, preview generation).

use std::path::PathBuf;
use std::sync::Arc;
use sqlx::SqlitePool;
use futures::StreamExt;
use rayon::prelude::*;
use crate::{db, library, metadata, organizer};
use super::messages::Message;

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
        PreviewStreamState::Init { pool, pattern, destination },
        |state| async move {
            match state {
                PreviewStreamState::Init { pool, pattern, destination } => {
                    // Load tracks from DB
                    let tracks = match db::get_all_tracks_with_metadata(&pool).await {
                        Ok(t) => t,
                        Err(_) => return Some((Message::OrganizePreviewComplete, PreviewStreamState::Done)),
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
                        }
                    ))
                }
                PreviewStreamState::Processing { tracks, index, pattern, destination, batch_size } => {
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
                    }).await.unwrap_or_default();
                    
                    Some((
                        Message::OrganizePreviewBatch(batch_previews),
                        PreviewStreamState::Processing { 
                            tracks, 
                            index: end, 
                            pattern, 
                            destination,
                            batch_size,
                        }
                    ))
                }
                PreviewStreamState::Done => None,
            }
        }
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
