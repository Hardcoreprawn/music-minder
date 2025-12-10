//! File watcher event handlers.
//!
//! Handles events from the background file watcher that monitors the
//! music library directories for changes. When files are added, modified,
//! or removed, we update the database incrementally.

use iced::Task;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::scanner::WatchEvent;

use super::super::messages::Message;
use super::super::state::LoadedState;
use super::load_tracks_task;

/// Handle file watcher messages.
pub fn handle_watcher(s: &mut LoadedState, message: Message) -> Task<Message> {
    match message {
        Message::WatcherStarted => {
            info!(target: "ui::watcher", "Background file watcher started");
            s.watcher_state.active = true;
            s.watcher_state.last_error = None;
            Task::none()
        }

        Message::WatcherStopped => {
            warn!(target: "ui::watcher", "Background file watcher stopped");
            s.watcher_state.active = false;
            Task::none()
        }

        Message::WatcherEvent(event) => {
            match event {
                WatchEvent::Created(path) => {
                    debug!(target: "ui::watcher", path = %path.display(), "File created");
                    s.watcher_state.pending_changes += 1;
                    handle_file_created(s, path)
                }
                WatchEvent::Modified(path) => {
                    debug!(target: "ui::watcher", path = %path.display(), "File modified");
                    s.watcher_state.pending_changes += 1;
                    handle_file_modified(s, path)
                }
                WatchEvent::Removed(path) => {
                    debug!(target: "ui::watcher", path = %path.display(), "File removed");
                    s.watcher_state.pending_changes += 1;
                    handle_file_removed(s, path)
                }
                WatchEvent::DirCreated(path) => {
                    debug!(target: "ui::watcher", path = %path.display(), "Directory created");
                    // New directory - might contain files, but file events will come separately
                    Task::none()
                }
                WatchEvent::Error(e) => {
                    warn!(target: "ui::watcher", error = %e, "Watcher error");
                    s.watcher_state.last_error = Some(e);
                    Task::none()
                }
            }
        }

        Message::LibraryFileChanged(_path) => {
            // A file change was processed - decrement pending count
            if s.watcher_state.pending_changes > 0 {
                s.watcher_state.pending_changes -= 1;
            }
            
            // If no more pending changes, trigger a lightweight refresh
            if s.watcher_state.pending_changes == 0 && !s.is_scanning {
                info!(target: "ui::watcher", "All pending changes processed, refreshing track list");
                return load_tracks_task(s.pool.clone());
            }
            Task::none()
        }

        _ => Task::none(),
    }
}

/// Handle a new file being created in the library.
fn handle_file_created(s: &mut LoadedState, path: PathBuf) -> Task<Message> {
    let pool = s.pool.clone();
    
    Task::perform(
        async move {
            // Read metadata from the new file
            let meta = match crate::metadata::read(&path) {
                Ok(m) => m,
                Err(e) => {
                    warn!(target: "ui::watcher", path = %path.display(), error = %e, "Failed to read metadata");
                    return path;
                }
            };

            // Get file mtime
            let mtime = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // Insert or update artist
            let artist_id = if !meta.artist.is_empty() {
                crate::db::get_or_create_artist(&pool, &meta.artist).await.ok()
            } else {
                None
            };

            // Insert or update album
            let album_id = if !meta.album.is_empty() {
                crate::db::get_or_create_album(&pool, &meta.album, None).await.ok()
            } else {
                None
            };

            // Insert track with mtime
            let path_str = path.to_string_lossy().to_string();
            if let Err(e) = crate::db::insert_track_with_mtime(&pool, &meta, &path_str, artist_id, album_id, mtime).await {
                warn!(target: "ui::watcher", path = %path.display(), error = %e, "Failed to insert track");
            } else {
                info!(target: "ui::watcher", path = %path.display(), title = %meta.title, "Track added to library");
            }

            path
        },
        Message::LibraryFileChanged,
    )
}

/// Handle a file being modified in the library.
fn handle_file_modified(s: &mut LoadedState, path: PathBuf) -> Task<Message> {
    let pool = s.pool.clone();
    
    Task::perform(
        async move {
            // Check if we know this file
            let path_str = path.to_string_lossy().to_string();
            let existing = crate::db::get_track_by_path(&pool, &path_str).await.ok().flatten();

            // Get current mtime
            let mtime = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            if let Some(track_info) = existing {
                // File exists in DB - check if mtime changed
                if track_info.mtime == Some(mtime) {
                    // mtime unchanged, skip
                    return path;
                }

                // Re-read metadata and update
                let meta = match crate::metadata::read(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(target: "ui::watcher", path = %path.display(), error = %e, "Failed to read metadata");
                        return path;
                    }
                };

                // Get or create artist/album
                let artist_id = if !meta.artist.is_empty() {
                    crate::db::get_or_create_artist(&pool, &meta.artist).await.ok()
                } else {
                    None
                };
                let album_id = if !meta.album.is_empty() {
                    crate::db::get_or_create_album(&pool, &meta.album, None).await.ok()
                } else {
                    None
                };

                // Update track
                if let Err(e) = crate::db::insert_track_with_mtime(&pool, &meta, &path_str, artist_id, album_id, mtime).await {
                    warn!(target: "ui::watcher", path = %path.display(), error = %e, "Failed to update track");
                } else {
                    debug!(target: "ui::watcher", path = %path.display(), "Track updated");
                }
            } else {
                // New file (not in DB) - treat as create
                debug!(target: "ui::watcher", path = %path.display(), "Modified file not in DB, treating as new");
            }

            path
        },
        Message::LibraryFileChanged,
    )
}

/// Handle a file being removed from the library.
fn handle_file_removed(s: &mut LoadedState, path: PathBuf) -> Task<Message> {
    let pool = s.pool.clone();
    
    Task::perform(
        async move {
            let path_str = path.to_string_lossy().to_string();
            
            match crate::db::delete_track_by_path(&pool, &path_str).await {
                Ok(true) => {
                    info!(target: "ui::watcher", path = %path.display(), "Track removed from library");
                }
                Ok(false) => {
                    debug!(target: "ui::watcher", path = %path.display(), "Track was not in database");
                }
                Err(e) => {
                    warn!(target: "ui::watcher", path = %path.display(), error = %e, "Failed to remove track");
                }
            }

            path
        },
        Message::LibraryFileChanged,
    )
}
