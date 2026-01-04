//! Update handlers for application messages.
//!
//! This module is split into submodules for maintainability:
//! - `db`: Database initialization
//! - `scan`: Library scanning
//! - `organize`: File organization and undo
//! - `enrichment`: Track identification and metadata writing
//! - `player`: Audio playback and media controls
//! - `diagnostics`: System diagnostics and cover art
//! - `watcher`: Background file system watching
//! - `search`: Search and filter functionality
//! - `keyboard`: Keyboard shortcut handling

mod db;
mod diagnostics;
mod enrichment;
mod keyboard;
mod organize;
mod player;
mod scan;
mod search;
mod selection;
mod track_detail;
mod watcher;

use iced::Task;
use std::path::PathBuf;

use crate::cover;

use super::messages::Message;

// Re-export all handler functions
pub use db::handle_db_init;
pub use diagnostics::handle_diagnostics;
pub use enrichment::{handle_enrich_pane, handle_enrichment};
pub use keyboard::handle_keyboard;
pub use organize::{handle_organize, handle_undo};
pub use player::handle_player;
pub use scan::handle_scan;
pub use search::handle_search_filter;
pub use selection::handle_selection;
pub use track_detail::handle_track_detail;
pub use watcher::handle_watcher;

/// Initial batch size for progressive loading - show UI quickly
const INITIAL_BATCH_SIZE: i64 = 200;

/// Helper to load tracks from database (all at once - legacy)
pub(crate) fn load_tracks_task(pool: sqlx::SqlitePool) -> Task<Message> {
    Task::perform(
        async move {
            let load_start = std::time::Instant::now();
            tracing::debug!("Loading tracks from database...");
            let result = crate::db::get_all_tracks_with_metadata(&pool)
                .await
                .map_err(|e| e.to_string());
            tracing::info!(
                "Tracks loaded in {:.1}ms",
                load_start.elapsed().as_secs_f64() * 1000.0
            );
            result
        },
        Message::TracksLoaded,
    )
}

/// Load initial batch of tracks for fast UI display
pub(crate) fn load_tracks_initial_task(pool: sqlx::SqlitePool) -> Task<Message> {
    Task::perform(
        async move {
            let load_start = std::time::Instant::now();
            tracing::debug!("Loading initial {} tracks...", INITIAL_BATCH_SIZE);

            // Get total count first (very fast query)
            let total = crate::db::count_tracks(&pool)
                .await
                .map_err(|e| e.to_string())?;

            // Load first batch
            let tracks = crate::db::get_tracks_paginated(&pool, INITIAL_BATCH_SIZE, 0)
                .await
                .map_err(|e| e.to_string())?;

            tracing::info!(
                "Initial {} tracks loaded in {:.1}ms (total: {})",
                tracks.len(),
                load_start.elapsed().as_secs_f64() * 1000.0,
                total
            );

            Ok((tracks, total))
        },
        Message::TracksLoadedInitial,
    )
}

/// Load remaining tracks after initial batch
pub(crate) fn load_tracks_remaining_task(
    pool: sqlx::SqlitePool,
    offset: i64,
    total: i64,
) -> Task<Message> {
    Task::perform(
        async move {
            let remaining = total - offset;
            if remaining <= 0 {
                return Ok(vec![]);
            }

            let load_start = std::time::Instant::now();
            tracing::debug!(
                "Loading remaining {} tracks (offset {})...",
                remaining,
                offset
            );

            let tracks = crate::db::get_tracks_paginated(&pool, remaining, offset)
                .await
                .map_err(|e| e.to_string())?;

            tracing::info!(
                "Remaining {} tracks loaded in {:.1}ms",
                tracks.len(),
                load_start.elapsed().as_secs_f64() * 1000.0
            );

            Ok(tracks)
        },
        Message::TracksLoadedMore,
    )
}

/// Helper to pick a folder
pub(crate) fn pick_folder_task(on_pick: fn(Option<PathBuf>) -> Message) -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|h| h.path().to_path_buf())
        },
        on_pick,
    )
}

/// Helper to resolve cover art in the background.
///
/// This is non-blocking and will never interfere with audio playback.
/// It first tries local sources (embedded, sidecar) which are fast,
/// then falls back to cache or remote fetch if a release ID is available.
pub(crate) fn resolve_cover_art_task(
    audio_path: PathBuf,
    release_id: Option<String>,
) -> Task<Message> {
    let path_for_message = audio_path.clone();
    Task::perform(
        async move {
            let resolver = cover::CoverResolver::new();

            // Try local sources first (fast, synchronous internally)
            if let Some(cover) = resolver.resolve_local(&audio_path) {
                return Ok(cover.into());
            }

            // Try cached cover if we have a release ID
            if let Some(ref id) = release_id
                && let Some(cover) = resolver.resolve_cached(id)
            {
                return Ok(cover.into());
            }

            // Try remote fetch (slow, async)
            if let Some(ref id) = release_id {
                match resolver.fetch_remote(id).await {
                    Ok(cover) => return Ok(cover.into()),
                    Err(e) => return Err(e),
                }
            }

            Err("No cover art sources available".to_string())
        },
        move |result| Message::CoverArtResolved(path_for_message.clone(), result),
    )
}

#[cfg(test)]
mod tests {
    //! Tests to verify external crate API contracts.
    //! These tests ensure our code will continue to work after dependency updates.

    /// Verify rfd::AsyncFileDialog API contract.
    /// This is a compile-time check - if this compiles, the API we use exists.
    #[allow(dead_code)]
    async fn verify_rfd_api_contract() {
        // We use: AsyncFileDialog::new().pick_folder().await
        let dialog = rfd::AsyncFileDialog::new();

        // pick_folder() should return impl Future<Output = Option<FileHandle>>
        let result = dialog.pick_folder().await;

        // FileHandle should have .path() -> &Path
        if let Some(handle) = result {
            let _path: &std::path::Path = handle.path();
            let _pathbuf: std::path::PathBuf = handle.path().to_path_buf();
        }
    }

    #[test]
    fn rfd_api_types_exist() {
        // Verify types we depend on exist
        fn _check_types() {
            let _: fn() -> rfd::AsyncFileDialog = rfd::AsyncFileDialog::new;
        }
    }
}
