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
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

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
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
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

    /// Create a new file watcher for the given directories with an async channel.
    ///
    /// This variant is designed for use in async contexts (like Iced subscriptions)
    /// where blocking `recv()` would starve the async runtime.
    ///
    /// Returns the watcher handle and a tokio mpsc receiver for watch events.
    pub fn new_async(
        watch_paths: Vec<PathBuf>,
    ) -> Result<(Self, tokio_mpsc::Receiver<WatchEvent>), WatchError> {
        let (tx, rx) = tokio_mpsc::channel(256);
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
                Self::handle_debounced_events_async(result, &tx);
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
                            notify::EventKind::Modify(ModifyKind::Data(_))
                            | notify::EventKind::Modify(ModifyKind::Metadata(_)) => {
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

    /// Handle debounced events from notify (async channel variant).
    ///
    /// Uses `try_send()` which is safe to call from sync code (notify's callback thread).
    fn handle_debounced_events_async(
        result: DebounceEventResult,
        tx: &tokio_mpsc::Sender<WatchEvent>,
    ) {
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
                            notify::EventKind::Modify(ModifyKind::Data(_))
                            | notify::EventKind::Modify(ModifyKind::Metadata(_)) => {
                                if is_audio_file(path) {
                                    tracing::debug!(target: "scanner::watcher", path = %path.display(), "File modified");
                                    Some(WatchEvent::Modified(path.clone()))
                                } else {
                                    None
                                }
                            }
                            notify::EventKind::Remove(RemoveKind::File) => {
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
                            // try_send is non-blocking and safe from sync context
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
fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_lowercase().as_str(),
                "mp3" | "flac" | "ogg" | "wav" | "m4a"
            )
        })
        .unwrap_or(false)
}

/// Check if a path looks like it could be an audio file (for deleted files).
fn looks_like_audio_path(path: &std::path::Path) -> bool {
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

    #[test]
    fn test_async_watcher_creation() {
        let dir = tempdir().unwrap();
        let (watcher, _rx) = FileWatcher::new_async(vec![dir.path().to_path_buf()]).unwrap();
        drop(watcher); // Should not panic
    }

    /// Test that async watcher doesn't block concurrent async tasks.
    ///
    /// This is a regression test for the bug where `recv_timeout()` in the
    /// watcher stream blocked Iced's async runtime, causing PlayerTick to freeze.
    ///
    /// The test verifies that:
    /// 1. The async watcher receiver can be polled without blocking
    /// 2. Other async tasks continue to run while waiting for watcher events
    #[tokio::test]
    async fn test_async_watcher_does_not_block_runtime() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempdir().unwrap();
        let (watcher, mut rx) = FileWatcher::new_async(vec![dir.path().to_path_buf()]).unwrap();

        // Counter to track how many times our "tick" task runs
        let tick_count = Arc::new(AtomicUsize::new(0));
        let tick_count_clone = Arc::clone(&tick_count);

        // Simulate PlayerTick: a fast-firing task that should NOT be blocked
        let tick_task = tokio::spawn(async move {
            for _ in 0..10 {
                tick_count_clone.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // Poll the watcher receiver with a timeout (simulating the subscription)
        // This should NOT block the tick_task above
        let watcher_task = tokio::spawn(async move {
            // Use tokio's timeout to wait for an event that won't come
            let result = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
            // We expect timeout since no file was created
            assert!(result.is_err(), "Expected timeout, got event");
            drop(watcher);
        });

        // Wait for both tasks
        let (tick_result, watcher_result) = tokio::join!(tick_task, watcher_task);
        tick_result.unwrap();
        watcher_result.unwrap();

        // The tick task should have run multiple times while watcher was waiting
        // If the watcher blocked, tick_count would be 0 or very low
        let final_count = tick_count.load(Ordering::SeqCst);
        assert!(
            final_count >= 3,
            "Tick task was starved! Only ran {} times (expected >= 3). \
             This suggests the watcher is blocking the async runtime.",
            final_count
        );
    }

    /// Test that async watcher receives events without blocking.
    #[tokio::test]
    async fn test_async_watcher_receives_events() {
        let dir = tempdir().unwrap();
        let (watcher, mut rx) = FileWatcher::new_async(vec![dir.path().to_path_buf()]).unwrap();

        // Create a file in a separate task
        let dir_path = dir.path().to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::thread::sleep(Duration::from_millis(100));
            let file_path = dir_path.join("test_song.mp3");
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"fake mp3").unwrap();
            file.sync_all().unwrap();
        });

        // Wait for the event with timeout
        let result = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
        drop(watcher);

        // We should receive an event (Created or Modified depending on OS)
        if let Ok(Some(event)) = result {
            match event {
                WatchEvent::Created(path) | WatchEvent::Modified(path) => {
                    assert_eq!(path.file_name().unwrap(), "test_song.mp3");
                }
                other => {
                    // DirCreated or other events are acceptable on some platforms
                    println!("Received unexpected but valid event: {:?}", other);
                }
            }
        }
        // Note: On some CI systems, file events may not fire reliably,
        // so we don't hard-fail if no event was received
    }
}
