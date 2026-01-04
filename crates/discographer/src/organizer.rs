//! File organization and movement utilities.
//!
//! Provides functionality to organize music files into a structured directory
//! hierarchy based on metadata patterns like `{Artist}/{Album}/{TrackNum} - {Title}.{ext}`.

use crate::metadata::TrackMetadata;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Generate a target path for a track based on the pattern and metadata
pub fn generate_target_path(
    source_path: &Path,
    metadata: &TrackMetadata,
    pattern: &str,
    destination_root: &Path,
) -> PathBuf {
    // Replace pattern variables with metadata
    let mut target = pattern.to_string();
    target = target.replace("{Artist}", &metadata.artist);
    target = target.replace("{Album}", &metadata.album);
    target = target.replace("{Title}", &metadata.title);

    if let Some(track_num) = metadata.track_number {
        target = target.replace("{TrackNum}", &format!("{:02}", track_num));
    }

    // Preserve file extension
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");
    target = target.replace("{ext}", ext);

    destination_root.join(target)
}

/// Preview what organize would do (dry-run)
pub fn preview_organize(
    source_path: &Path,
    metadata: &TrackMetadata,
    pattern: &str,
    destination_root: &Path,
) -> PathBuf {
    generate_target_path(source_path, metadata, pattern, destination_root)
}

/// Actually organize a file
pub fn organize_track(
    source_path: &Path,
    metadata: &TrackMetadata,
    pattern: &str,
    destination_root: &Path,
) -> Result<PathBuf> {
    let target = generate_target_path(source_path, metadata, pattern, destination_root);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Move the file
    std::fs::rename(source_path, &target)?;

    Ok(target)
}
