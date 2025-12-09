//! Update handlers for application messages.
//!
//! This module is split into submodules for maintainability:
//! - `db`: Database initialization
//! - `scan`: Library scanning
//! - `organize`: File organization and undo
//! - `enrichment`: Track identification and metadata writing
//! - `player`: Audio playback and media controls
//! - `diagnostics`: System diagnostics and cover art

mod db;
mod diagnostics;
mod enrichment;
mod organize;
mod player;
mod scan;

use iced::Task;
use std::path::PathBuf;

use crate::cover;

use super::messages::Message;

// Re-export all handler functions
pub use db::handle_db_init;
pub use diagnostics::handle_diagnostics;
pub use enrichment::handle_enrichment;
pub use organize::{handle_organize, handle_undo};
pub use player::handle_player;
pub use scan::handle_scan;

/// Helper to load tracks from database
pub(crate) fn load_tracks_task(pool: sqlx::SqlitePool) -> Task<Message> {
    Task::perform(
        async move {
            crate::db::get_all_tracks_with_metadata(&pool)
                .await
                .map_err(|e| e.to_string())
        },
        Message::TracksLoaded,
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
pub(crate) fn resolve_cover_art_task(audio_path: PathBuf, release_id: Option<String>) -> Task<Message> {
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
                && let Some(cover) = resolver.resolve_cached(id) {
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
