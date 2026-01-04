//! File organization and movement utilities.
//!
//! Provides functionality to organize music files into a structured directory
//! hierarchy based on metadata patterns like `{Artist}/{Album}/{TrackNum} - {Title}.{ext}`.
//!
//! # Features
//! - Pattern-based file organization
//! - Preview mode to see changes before applying
//! - Undo support with logged move operations
//! - Automatic cleanup of empty directories

use crate::metadata::TrackMetadata;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A record of a file move operation, used for undo functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub track_id: i64,
}

/// The undo log containing the last organize operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UndoLog {
    pub moves: Vec<MoveRecord>,
    pub timestamp: Option<String>,
}

impl UndoLog {
    const LOG_PATH: &'static str = "music_minder_undo.json";

    /// Load the undo log from disk
    pub fn load() -> Option<Self> {
        fs::read_to_string(Self::LOG_PATH)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    /// Save the undo log to disk
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(Self::LOG_PATH, json)?;
        Ok(())
    }

    /// Clear the undo log
    pub fn clear() -> Result<()> {
        if Path::new(Self::LOG_PATH).exists() {
            fs::remove_file(Self::LOG_PATH)?;
        }
        Ok(())
    }

    /// Check if there's an undo operation available
    pub fn has_undo() -> bool {
        Path::new(Self::LOG_PATH).exists()
    }
}

/// Preview result for dry-run
#[derive(Debug, Clone)]
pub struct OrganizePreview {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub track_id: i64,
}

/// Generates a preview of what organize would do (dry-run)
pub fn preview_organize(
    source_path: &Path,
    metadata: &TrackMetadata,
    pattern: &str,
    destination_root: &Path,
    track_id: i64,
) -> OrganizePreview {
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");

    let track_num = metadata
        .track_number
        .map(|n| format!("{:02}", n))
        .unwrap_or_else(|| "00".to_string());

    let path_str = pattern
        .replace("{Artist}", &sanitize_filename(&metadata.artist))
        .replace("{Album}", &sanitize_filename(&metadata.album))
        .replace("{Title}", &sanitize_filename(&metadata.title))
        .replace("{TrackNum}", &track_num)
        .replace("{ext}", ext);

    let dest_path = destination_root.join(&path_str);

    OrganizePreview {
        source: source_path.to_path_buf(),
        destination: dest_path,
        track_id,
    }
}

/// Organizes a track file by moving it to a new location based on a pattern.
/// Pattern variables: {Artist}, {Album}, {Title}, {TrackNum}, {ext}
/// Example: "{Artist}/{Album}/{TrackNum} - {Title}.{ext}"
pub fn organize_track(
    source_path: &Path,
    metadata: &TrackMetadata,
    pattern: &str,
    destination_root: &Path,
) -> Result<PathBuf> {
    // Get file extension
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");

    // Format track number with zero padding
    let track_num = metadata
        .track_number
        .map(|n| format!("{:02}", n))
        .unwrap_or_else(|| "00".to_string());

    // Substitute pattern variables
    let path_str = pattern
        .replace("{Artist}", &sanitize_filename(&metadata.artist))
        .replace("{Album}", &sanitize_filename(&metadata.album))
        .replace("{Title}", &sanitize_filename(&metadata.title))
        .replace("{TrackNum}", &track_num)
        .replace("{ext}", ext);

    // Build destination path
    let dest_path = destination_root.join(&path_str);

    // Create parent directories
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    // Move or copy the file
    if let Err(_e) = fs::rename(source_path, &dest_path) {
        // If rename fails (cross-device), try copy + delete

        fs::copy(source_path, &dest_path)
            .with_context(|| format!("Failed to copy file to: {:?}", dest_path))?;
        fs::remove_file(source_path)
            .with_context(|| format!("Failed to remove source file: {:?}", source_path))?;
    }

    Ok(dest_path)
}

/// Sanitizes a filename by removing/replacing invalid characters
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Moves a single file back to its original location (for undo)
pub fn undo_move(record: &MoveRecord) -> Result<()> {
    // Create parent directories for the original location
    if let Some(parent) = record.source.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    // Move the file back
    if let Err(_e) = fs::rename(&record.destination, &record.source) {
        // If rename fails (cross-device), try copy + delete
        fs::copy(&record.destination, &record.source)
            .with_context(|| format!("Failed to copy file to: {:?}", record.source))?;
        fs::remove_file(&record.destination)
            .with_context(|| format!("Failed to remove file: {:?}", record.destination))?;
    }

    // Try to clean up empty directories
    if let Some(parent) = record.destination.parent() {
        let _ = remove_empty_dirs(parent);
    }

    Ok(())
}

/// Recursively removes empty directories up the tree
fn remove_empty_dirs(path: &Path) -> Result<()> {
    if path.is_dir() && fs::read_dir(path)?.next().is_none() {
        fs::remove_dir(path)?;
        if let Some(parent) = path.parent() {
            let _ = remove_empty_dirs(parent);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("AC/DC"), "AC_DC");
        assert_eq!(sanitize_filename("Track: Title"), "Track_ Title");
        assert_eq!(sanitize_filename("Valid Name"), "Valid Name");
        assert_eq!(sanitize_filename("Artist?"), "Artist_");
        assert_eq!(sanitize_filename("a<b>c"), "a_b_c");
        assert_eq!(sanitize_filename("pipe|test"), "pipe_test");
    }

    #[test]
    fn test_preview_organize_generates_correct_path() {
        let metadata = TrackMetadata {
            title: "Song Title".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration: 180,
            track_number: Some(5),
        };

        let pattern = "{Artist}/{Album}/{TrackNum} - {Title}.{ext}";
        let source = Path::new("/tmp/song.mp3");
        let dest_root = Path::new("/music");

        let preview = preview_organize(source, &metadata, pattern, dest_root, 42);

        assert_eq!(preview.source, source);
        assert_eq!(preview.track_id, 42);
        assert_eq!(
            preview.destination,
            PathBuf::from("/music/Test Artist/Test Album/05 - Song Title.mp3")
        );
    }

    #[test]
    fn test_preview_organize_handles_missing_track_number() {
        let metadata = TrackMetadata {
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration: 180,
            track_number: None,
        };

        let preview = preview_organize(
            Path::new("/test.flac"),
            &metadata,
            "{Artist}/{Album}/{TrackNum} - {Title}.{ext}",
            Path::new("/out"),
            1,
        );

        assert_eq!(
            preview.destination,
            PathBuf::from("/out/Artist/Album/00 - Song.flac")
        );
    }

    #[test]
    fn test_preview_organize_sanitizes_special_chars() {
        let metadata = TrackMetadata {
            title: "What?".to_string(),
            artist: "AC/DC".to_string(),
            album: "Back: In Black".to_string(),
            duration: 180,
            track_number: Some(1),
        };

        let preview = preview_organize(
            Path::new("/test.mp3"),
            &metadata,
            "{Artist}/{Album}/{Title}.{ext}",
            Path::new("/out"),
            1,
        );

        assert_eq!(
            preview.destination,
            PathBuf::from("/out/AC_DC/Back_ In Black/What_.mp3")
        );
    }

    #[test]
    fn test_organize_track_moves_file() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let dest_dir = temp.path().join("dest");
        std::fs::create_dir_all(&source_dir).unwrap();

        // Create a source file
        let source_file = source_dir.join("test.mp3");
        std::fs::write(&source_file, b"fake mp3 content").unwrap();

        let metadata = TrackMetadata {
            title: "Test".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration: 100,
            track_number: Some(1),
        };

        let result = organize_track(
            &source_file,
            &metadata,
            "{Artist}/{Album}/{TrackNum} - {Title}.{ext}",
            &dest_dir,
        );

        assert!(result.is_ok());
        let new_path = result.unwrap();
        assert!(new_path.exists());
        assert!(!source_file.exists()); // Source should be moved
        assert_eq!(
            std::fs::read_to_string(&new_path).unwrap(),
            "fake mp3 content"
        );
    }

    #[test]
    fn test_undo_log_save_and_load() {
        let temp = tempdir().unwrap();
        let _log_path = temp.path().join("undo.json");

        // Override the log path for testing isn't easily possible with const,
        // but we can test the serialization
        let log = UndoLog {
            moves: vec![MoveRecord {
                source: PathBuf::from("/original/path.mp3"),
                destination: PathBuf::from("/new/path.mp3"),
                track_id: 42,
            }],
            timestamp: Some("2025-01-01T00:00:00Z".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&log).unwrap();
        let loaded: UndoLog = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.moves.len(), 1);
        assert_eq!(loaded.moves[0].track_id, 42);
        assert_eq!(loaded.moves[0].source, PathBuf::from("/original/path.mp3"));
    }

    #[test]
    fn test_undo_move_restores_file() {
        let temp = tempdir().unwrap();
        let original_dir = temp.path().join("original");
        let moved_dir = temp.path().join("moved");
        std::fs::create_dir_all(&original_dir).unwrap();
        std::fs::create_dir_all(&moved_dir).unwrap();

        // Create a "moved" file
        let moved_file = moved_dir.join("test.mp3");
        std::fs::write(&moved_file, b"content").unwrap();

        let record = MoveRecord {
            source: original_dir.join("test.mp3"),
            destination: moved_file.clone(),
            track_id: 1,
        };

        let result = undo_move(&record);
        assert!(result.is_ok());
        assert!(record.source.exists());
        assert!(!moved_file.exists());
    }
}

/// Property-based tests using proptest
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Generate valid filename characters (excluding path separators and invalid chars)
    fn valid_filename_char() -> impl Strategy<Value = char> {
        prop::char::range('!', '~').prop_filter("no invalid chars", |c| {
            !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
    }

    /// Generate a valid filename string
    fn valid_filename() -> impl Strategy<Value = String> {
        prop::collection::vec(valid_filename_char(), 1..50)
            .prop_map(|chars| chars.into_iter().collect())
    }

    /// Generate an arbitrary string that might contain invalid characters
    fn arbitrary_filename() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9 /:*?\"<>|_-]{1,50}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    proptest! {
        /// Sanitized filenames should never contain path separators
        #[test]
        fn sanitize_removes_path_separators(input in arbitrary_filename()) {
            let sanitized = sanitize_filename(&input);
            prop_assert!(!sanitized.contains('/'), "Found / in: {}", sanitized);
            prop_assert!(!sanitized.contains('\\'), "Found \\ in: {}", sanitized);
        }

        /// Sanitized filenames should never contain Windows-invalid characters
        #[test]
        fn sanitize_removes_invalid_chars(input in arbitrary_filename()) {
            let sanitized = sanitize_filename(&input);
            for c in [':', '*', '?', '"', '<', '>', '|'] {
                prop_assert!(!sanitized.contains(c), "Found {} in: {}", c, sanitized);
            }
        }

        /// Sanitized filename length should be same as input length
        #[test]
        fn sanitize_preserves_length(input in arbitrary_filename()) {
            let sanitized = sanitize_filename(&input);
            prop_assert_eq!(input.chars().count(), sanitized.chars().count());
        }

        /// Valid filenames should pass through unchanged
        #[test]
        fn sanitize_preserves_valid_names(input in valid_filename()) {
            let sanitized = sanitize_filename(&input);
            prop_assert_eq!(input, sanitized);
        }

        /// Preview organize should always produce a path under destination root
        #[test]
        fn preview_stays_under_dest_root(
            artist in valid_filename(),
            album in valid_filename(),
            title in valid_filename(),
            track_num in proptest::option::of(1u32..100),
        ) {
            let metadata = TrackMetadata {
                title,
                artist,
                album,
                duration: 180,
                track_number: track_num,
            };

            let source = PathBuf::from("/source/test.mp3");
            let dest_root = PathBuf::from("/music/library");

            let preview = preview_organize(
                &source,
                &metadata,
                "{Artist}/{Album}/{TrackNum} - {Title}.{ext}",
                &dest_root,
                1,
            );

            prop_assert!(
                preview.destination.starts_with(&dest_root),
                "Destination {:?} should start with {:?}",
                preview.destination,
                dest_root
            );
        }

        /// Preview organize should preserve the file extension
        #[test]
        fn preview_preserves_extension(
            ext in prop::sample::select(vec!["mp3", "flac", "ogg", "wav", "m4a"]),
            title in valid_filename(),
        ) {
            let metadata = TrackMetadata {
                title,
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                duration: 180,
                track_number: Some(1),
            };

            let source = PathBuf::from(format!("/source/test.{}", ext));
            let dest_root = PathBuf::from("/music");

            let preview = preview_organize(
                &source,
                &metadata,
                "{Artist}/{Album}/{Title}.{ext}",
                &dest_root,
                1,
            );

            let result_ext = preview.destination.extension().and_then(|e| e.to_str());
            prop_assert_eq!(Some(ext), result_ext);
        }

        /// Track number formatting should always be zero-padded to 2 digits
        #[test]
        fn track_number_is_zero_padded(track_num in 1u32..100) {
            let metadata = TrackMetadata {
                title: "Song".to_string(),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                duration: 180,
                track_number: Some(track_num),
            };

            let preview = preview_organize(
                Path::new("/test.mp3"),
                &metadata,
                "{TrackNum}.{ext}",
                Path::new("/out"),
                1,
            );

            let filename = preview.destination.file_name().unwrap().to_str().unwrap();
            let expected = format!("{:02}.mp3", track_num);
            prop_assert_eq!(filename, expected.as_str());
        }
    }
}
