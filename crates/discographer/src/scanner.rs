//! Directory scanning for audio files.
//!
//! Provides sync/async file discovery within a directory tree.
//! Filters for common audio formats: MP3, FLAC, OGG, M4A, WAV.

use std::path::Path;

/// Supported audio file extensions (lowercase).
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "wav", "m4a"];

/// Check if a path has a supported audio file extension.
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
