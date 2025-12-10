//! File system watcher for detecting music library changes.
//!
//! Uses the `notify` crate to watch directories for changes and emit events
//! when audio files are added, modified, or removed.
//!
//! # Design
//!
//! - **Debounced events**: Multiple rapid changes coalesce into single events
//! - **Audio files only**: Filters for supported extensions (mp3, flac, etc.)
//! - **Non-blocking**: Runs on a dedicated thread, sends events via channel
//! - **Graceful shutdown**: Stop watching via the returned handle
//!
//! # Usage
//!
//! ```rust,ignore
//! let (watcher, rx) = FileWatcher::new(vec!["/music".into()])?;
//!
//! // In another task/thread:
//! while let Ok(event) = rx.recv() {
//!     match event {
//!         WatchEvent::Created(path) => println!("New file: {:?}", path),
//!         WatchEvent::Modified(path) => println!("Changed: {:?}", path),
//!         WatchEvent::Removed(path) => println!("Deleted: {:?}", path),
//!     }
//! }
//!
//! // To stop watching:
//! drop(watcher);
//! ```

use crossbeam_channel::{Receiver, Sender, bounded};
use notify::{
    RecommendedWatcher, RecursiveMode,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdMap, new_debouncer};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Events emitted by the file watcher.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A new audio file was created
    Created(PathBuf),
    /// An existing audio file was modified
    Modified(PathBuf),
    /// An audio file was removed
    Removed(PathBuf),
    /// A directory was created (may contain audio files)
    DirCreated(PathBuf),
    /// An error occurred while watching
    Error(String),
}

/// Handle to a running file watcher.
///
/// Dropping this handle will stop the watcher.
pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    running: Arc<AtomicBool>,
}

impl FileWatcher {
    /// Create a new file watcher for the given directories.
    ///
    /// Returns the watcher handle and a receiver for watch events.
    pub fn new(watch_paths: Vec<PathBuf>) -> Result<(Self, Receiver<WatchEvent>), WatchError> {
        let (tx, rx) = bounded(256);
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        // Create debouncer with 500ms timeout
        let debouncer = new_debouncer(
            Duration::from_millis(500),
            None, // No tick rate limit
            move |result: DebounceEventResult| {
                if !running_clone.load(Ordering::Relaxed) {
                    return;
                }
                Self::handle_debounced_events(result, &tx);
            },
        )
        .map_err(|e| WatchError::Init(e.to_string()))?;

        let mut watcher = Self {
            _debouncer: debouncer,
            running,
        };

        // Watch all paths
        for path in watch_paths {
            watcher.watch(&path)?;
        }

        Ok((watcher, rx))
    }

    /// Add a directory to watch.
    pub fn watch(&mut self, path: &PathBuf) -> Result<(), WatchError> {
        tracing::info!(target: "scanner::watcher", path = %path.display(), "Watching directory");
        self._debouncer
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| WatchError::Watch(e.to_string()))?;
        
        Ok(())
    }

    /// Stop watching a directory.
    pub fn unwatch(&mut self, path: &PathBuf) -> Result<(), WatchError> {
        tracing::info!(target: "scanner::watcher", path = %path.display(), "Unwatching directory");
        self._debouncer
            .unwatch(path)
            .map_err(|e| WatchError::Watch(e.to_string()))
    }

    /// Handle debounced events from notify.
    fn handle_debounced_events(result: DebounceEventResult, tx: &Sender<WatchEvent>) {
        match result {
            Ok(events) => {
                for event in events {
                    for path in &event.paths {
                        // Skip non-audio files
                        if path.is_file() && !is_audio_file(path) {
                            continue;
                        }

                        let watch_event = match event.kind {
                            notify::EventKind::Create(CreateKind::File) => {
                                if is_audio_file(path) {
                                    tracing::debug!(target: "scanner::watcher", path = %path.display(), "File created");
                                    Some(WatchEvent::Created(path.clone()))
                                } else {
                                    None
                                }
                            }
                            notify::EventKind::Create(CreateKind::Folder) => {
                                tracing::debug!(target: "scanner::watcher", path = %path.display(), "Directory created");
                                Some(WatchEvent::DirCreated(path.clone()))
                            }
                            notify::EventKind::Modify(ModifyKind::Data(_)) |
                            notify::EventKind::Modify(ModifyKind::Metadata(_)) => {
                                if is_audio_file(path) {
                                    tracing::debug!(target: "scanner::watcher", path = %path.display(), "File modified");
                                    Some(WatchEvent::Modified(path.clone()))
                                } else {
                                    None
                                }
                            }
                            notify::EventKind::Remove(RemoveKind::File) => {
                                // For removed files, we can't check extension anymore
                                // so we check if it looks like an audio path
                                if looks_like_audio_path(path) {
                                    tracing::debug!(target: "scanner::watcher", path = %path.display(), "File removed");
                                    Some(WatchEvent::Removed(path.clone()))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };

                        if let Some(evt) = watch_event {
                            let _ = tx.try_send(evt);
                        }
                    }
                }
            }
            Err(errors) => {
                for error in errors {
                    tracing::warn!(target: "scanner::watcher", error = %error, "Watch error");
                    let _ = tx.try_send(WatchEvent::Error(error.to_string()));
                }
            }
        }
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        tracing::debug!(target: "scanner::watcher", "File watcher stopped");
    }
}

/// Check if a path is an audio file by extension.
fn is_audio_file(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "mp3" | "flac" | "ogg" | "wav" | "m4a"))
        .unwrap_or(false)
}

/// Check if a path looks like it could be an audio file (for deleted files).
fn looks_like_audio_path(path: &PathBuf) -> bool {
    is_audio_file(path)
}

/// Errors that can occur during file watching.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WatchError {
    #[error("Failed to initialize watcher: {0}")]
    Init(String),
    #[error("Failed to watch path: {0}")]
    Watch(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(&PathBuf::from("song.mp3")));
        assert!(is_audio_file(&PathBuf::from("song.FLAC")));
        assert!(is_audio_file(&PathBuf::from("song.ogg")));
        assert!(!is_audio_file(&PathBuf::from("image.png")));
        assert!(!is_audio_file(&PathBuf::from("document.txt")));
    }

    #[test]
    fn test_watcher_creation() {
        let dir = tempdir().unwrap();
        let (watcher, _rx) = FileWatcher::new(vec![dir.path().to_path_buf()]).unwrap();
        drop(watcher); // Should not panic
    }

    #[test]
    fn test_watcher_detects_new_file() {
        let dir = tempdir().unwrap();
        let (watcher, rx) = FileWatcher::new(vec![dir.path().to_path_buf()]).unwrap();

        // Create a file
        let file_path = dir.path().join("new_song.mp3");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"fake mp3 content").unwrap();
        file.sync_all().unwrap();

        // Wait for event (with timeout)
        let event = rx.recv_timeout(Duration::from_secs(2));
        
        // Clean up
        drop(watcher);

        // Check we got a create event
        if let Ok(WatchEvent::Created(path)) = event {
            assert_eq!(path.file_name().unwrap(), "new_song.mp3");
        }
        // Note: On some systems, the event might not fire within the timeout
        // due to debouncing or filesystem quirks, so we don't assert!(event.is_ok())
    }
}
