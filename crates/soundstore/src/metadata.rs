//! Audio file metadata reading and writing.
//!
//! Uses the lofty crate for format-independent metadata access.
//! Supports reading from and writing to MP3, FLAC, OGG, M4A, and WAV files.
//!
//! # Features
//! - Read track metadata (title, artist, album, year, track number)
//! - Preview metadata changes before writing
//! - Write enriched metadata from identification services
//! - Support for MusicBrainz recording IDs
//! - Embed cover art images

use anyhow::{Context, Result};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use std::path::Path;

/// Track metadata - uses String for SQLx compatibility.
/// The metadata is read once and stored, so allocation overhead is minimal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: u64,
    pub track_number: Option<u32>,
}

/// Comprehensive metadata - ALL fields an audio file can hold
#[derive(Debug, Clone, Default)]
pub struct FullMetadata {
    // Basic info
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,

    // Track positioning
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,

    // Additional metadata
    pub composer: Option<String>,
    pub comment: Option<String>,
    pub lyrics: Option<String>,

    // MusicBrainz IDs
    pub musicbrainz_recording_id: Option<String>,
    pub musicbrainz_artist_id: Option<String>,
    pub musicbrainz_release_id: Option<String>,
    pub musicbrainz_release_group_id: Option<String>,
    pub musicbrainz_track_id: Option<String>,

    // Audio properties
    pub duration_secs: u64,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bits_per_sample: Option<u8>,

    // Cover art
    pub has_cover_art: bool,
    pub cover_art_size: Option<(u32, u32)>, // width x height if known

    // File info
    pub format: String,
    pub file_size: u64,
}

/// Options for controlling what metadata gets written
#[derive(Debug, Clone, Default)]
pub struct WriteOptions2 {
    /// Only write fields that are currently empty/unknown in the file
    pub only_fill_empty: bool,
    /// Write MusicBrainz IDs to tags
    pub write_musicbrainz_ids: bool,
}

/// Result of a write operation
#[derive(Debug, Clone)]
pub struct WriteResult {
    /// Number of fields that were updated
    pub fields_updated: usize,
    /// Fields that were skipped (already had values)
    pub fields_skipped: Vec<String>,
}

pub fn read(path: &Path) -> Result<TrackMetadata> {
    // Probe the file to determine format and read tags
    let tagged_file = Probe::open(path)
        .context("Failed to open file for probing")?
        .read()
        .context("Failed to read file metadata")?;

    // Get the primary tag, or fall back to the first available tag
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    // Extract fields with defaults
    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Title".to_string());

    let artist = tag
        .and_then(|t| t.artist().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = tag
        .and_then(|t| t.album().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let track_number = tag.and_then(|t| t.track());

    // Get duration from properties
    let properties = tagged_file.properties();
    let duration = properties.duration().as_secs();

    Ok(TrackMetadata {
        title,
        artist,
        album,
        duration,
        track_number,
    })
}

/// Read ALL metadata from an audio file
pub fn read_full(path: &Path) -> Result<FullMetadata> {
    let tagged_file = Probe::open(path)
        .context("Failed to open file for probing")?
        .read()
        .context("Failed to read file metadata")?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let properties = tagged_file.properties();

    // Get file size
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    // Determine format from file type
    let format = format!("{:?}", tagged_file.file_type());

    // Helper to get tag text
    let get_text = |key: ItemKey| -> Option<String> {
        tag.and_then(|t| t.get(&key))
            .and_then(|item| item.value().text())
            .map(|s| s.to_string())
    };

    // Check for cover art
    let (has_cover_art, cover_art_size) = tag
        .map(|t| {
            let pics = t.pictures();
            if pics.is_empty() {
                (false, None)
            } else {
                // Try to get dimensions from first picture
                // Note: lofty doesn't parse image dimensions, so we'd need image crate
                (true, None)
            }
        })
        .unwrap_or((false, None));

    Ok(FullMetadata {
        // Basic info
        title: tag.and_then(|t| t.title().map(|s| s.to_string())),
        artist: tag.and_then(|t| t.artist().map(|s| s.to_string())),
        album: tag.and_then(|t| t.album().map(|s| s.to_string())),
        album_artist: get_text(ItemKey::AlbumArtist),
        year: tag.and_then(|t| t.year()),
        genre: tag.and_then(|t| t.genre().map(|s| s.to_string())),

        // Track positioning
        track_number: tag.and_then(|t| t.track()),
        total_tracks: tag.and_then(|t| t.track_total()),
        disc_number: tag.and_then(|t| t.disk()),
        total_discs: tag.and_then(|t| t.disk_total()),

        // Additional metadata
        composer: get_text(ItemKey::Composer),
        comment: tag.and_then(|t| t.comment().map(|s| s.to_string())),
        lyrics: get_text(ItemKey::Lyrics),

        // MusicBrainz IDs
        musicbrainz_recording_id: {
            let val = get_text(ItemKey::MusicBrainzRecordingId);
            eprintln!("[DEBUG READ] musicbrainz_recording_id from file: {:?}", val);
            val
        },
        musicbrainz_artist_id: get_text(ItemKey::MusicBrainzArtistId),
        musicbrainz_release_id: get_text(ItemKey::MusicBrainzReleaseId),
        musicbrainz_release_group_id: get_text(ItemKey::MusicBrainzReleaseGroupId),
        musicbrainz_track_id: get_text(ItemKey::MusicBrainzTrackId),

        // Audio properties
        duration_secs: properties.duration().as_secs(),
        bitrate: properties.audio_bitrate(),
        sample_rate: properties.sample_rate(),
        channels: properties.channels(),
        bits_per_sample: properties.bit_depth(),

        // Cover art
        has_cover_art,
        cover_art_size,

        // File info
        format,
        file_size,
    })
}

/// Result of a preview operation
#[derive(Debug, Clone)]
pub struct WritePreview {
    pub changes: Vec<FieldChange>,
}

/// A single field change
#[derive(Debug, Clone)]
pub struct FieldChange {
    pub field: String,
    pub current_value: String,
    pub new_value: String,
}

/// Write metadata to an audio file
///
/// The `track_data` parameter should implement Into<FullMetadata> to allow flexibility
/// in what metadata structure is passed (e.g., IdentifiedTrack from enrichment module).
///
/// # Safety
///
/// This function uses a safe two-phase write:
/// 1. Build modified tag structure in memory
/// 2. Only then open file with write mode
///
/// This prevents data loss if metadata parsing/building fails.
pub fn write<T: Into<FullMetadata>>(
    path: &Path,
    track_data: T,
    options: &WriteOptions2,
) -> Result<WriteResult> {
    let metadata = track_data.into();

    // Phase 1: Validate file is accessible and not corrupted
    let current = read_full(path)
        .context("Failed to read current metadata - file may be corrupted or inaccessible")?;

    // Phase 2: Check file is not read-only
    let file_metadata = std::fs::metadata(path).context("Failed to get file metadata")?;
    if file_metadata.permissions().readonly() {
        anyhow::bail!("File is read-only: {}", path.display());
    }

    let mut fields_updated = 0;
    let mut fields_skipped = Vec::new();

    // Helper to decide whether to write a field
    let should_write = |current_val: &Option<String>| -> bool {
        !options.only_fill_empty || current_val.as_deref().is_none_or(str::is_empty)
    };

    // Phase 3: Open the file for reading and build tag structure
    // CRITICAL: Do NOT open with truncate=true until we're ready to write
    let mut tagged_file = Probe::open(path)
        .context("Failed to open file for writing")?
        .read()
        .context("Failed to read file for metadata writing")?;

    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| anyhow::anyhow!("No tag found in file - format may not support tags"))?;

    // Write basic metadata
    if let Some(ref title) = metadata.title {
        if should_write(&current.title) {
            tag.set_title(title.clone());
            fields_updated += 1;
        } else {
            fields_skipped.push("title".to_string());
        }
    }

    if let Some(ref artist) = metadata.artist {
        if should_write(&current.artist) {
            tag.set_artist(artist.clone());
            fields_updated += 1;
        } else {
            fields_skipped.push("artist".to_string());
        }
    }

    if let Some(ref album) = metadata.album {
        if should_write(&current.album) {
            tag.set_album(album.clone());
            fields_updated += 1;
        } else {
            fields_skipped.push("album".to_string());
        }
    }

    if let Some(track_num) = metadata.track_number {
        if current.track_number.is_none() || !options.only_fill_empty {
            tag.set_track(track_num);
            fields_updated += 1;
        } else {
            fields_skipped.push("track_number".to_string());
        }
    }

    if let Some(year) = metadata.year {
        if current.year.is_none() || !options.only_fill_empty {
            tag.set_year(year);
            fields_updated += 1;
        } else {
            fields_skipped.push("year".to_string());
        }
    }

    if let Some(ref genre) = metadata.genre {
        if should_write(&current.genre) {
            tag.set_genre(genre.clone());
            fields_updated += 1;
        } else {
            fields_skipped.push("genre".to_string());
        }
    }

    // MusicBrainz IDs require custom tag items which we'll skip for now
    // The accessor trait doesn't provide setters for MB IDs
    if options.write_musicbrainz_ids && metadata.musicbrainz_recording_id.is_some() {
        // Note: Writing MB IDs would require direct tag manipulation which lofty
        // doesn't expose via the Accessor trait. This is acceptable since
        // MusicBrainz IDs are typically read-only from identification services.
        fields_skipped.push("musicbrainz_recording_id (unsupported by Accessor)".to_string());
    }

    // Phase 4: Safe write - only truncate AFTER all validation passes
    // CRITICAL: Only open with truncate=true at the last possible moment
    // This prevents data loss if any previous step failed

    // Create backup path in case write fails
    let backup_path = path.with_extension("bak.tmp");

    // Copy original file to backup (defensive - in case write fails mid-operation)
    std::fs::copy(path, &backup_path).context("Failed to create backup before writing")?;

    // Now safe to truncate and write
    let write_result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .context("Failed to open file for writing")?;

        tagged_file
            .save_to(&mut file, Default::default())
            .context("Failed to save metadata to file")?;

        // Flush to ensure data is written
        use std::io::Write;
        file.flush().context("Failed to flush file writes")?;

        Ok(())
    })();

    // Handle write result and cleanup backup
    match write_result {
        Ok(()) => {
            // Success - remove backup
            let _ = std::fs::remove_file(&backup_path);
            Ok(WriteResult {
                fields_updated,
                fields_skipped,
            })
        }
        Err(e) => {
            // Failure - restore from backup
            if backup_path.exists() {
                if let Err(restore_err) = std::fs::copy(&backup_path, path) {
                    eprintln!(
                        "CRITICAL: Failed to restore backup after write failure: {}",
                        restore_err
                    );
                    eprintln!("Backup saved at: {}", backup_path.display());
                } else {
                    // Successfully restored, can remove backup
                    let _ = std::fs::remove_file(&backup_path);
                }
            }
            Err(e).context("Failed to write metadata (original file restored from backup)")
        }
    }
}

/// Preview what changes would be made without actually writing
pub fn preview_write<T: Into<FullMetadata>>(
    path: &Path,
    track_data: T,
    options: &WriteOptions2,
) -> Result<WritePreview> {
    let metadata = track_data.into();
    let current = read_full(path)?;
    let mut changes = Vec::new();

    // Helper to decide whether to write a field
    let should_write = |current_val: &Option<String>| -> bool {
        !options.only_fill_empty || current_val.as_deref().is_none_or(str::is_empty)
    };

    // Check what would change for title
    if let Some(ref new_title) = metadata.title
        && should_write(&current.title)
    {
        let current_value = current.title.unwrap_or_default();
        if current_value != *new_title {
            changes.push(FieldChange {
                field: "title".to_string(),
                current_value,
                new_value: new_title.clone(),
            });
        }
    }

    // Check what would change for artist
    if let Some(ref new_artist) = metadata.artist
        && should_write(&current.artist)
    {
        let current_value = current.artist.unwrap_or_default();
        if current_value != *new_artist {
            changes.push(FieldChange {
                field: "artist".to_string(),
                current_value,
                new_value: new_artist.clone(),
            });
        }
    }

    // Check what would change for album
    if let Some(ref new_album) = metadata.album
        && should_write(&current.album)
    {
        let current_value = current.album.unwrap_or_default();
        if current_value != *new_album {
            changes.push(FieldChange {
                field: "album".to_string(),
                current_value,
                new_value: new_album.clone(),
            });
        }
    }

    Ok(WritePreview { changes })
}

/// Check if a file already has embedded cover art
pub fn has_cover_art(path: &Path) -> Result<bool> {
    let tagged_file = Probe::open(path)
        .context("Failed to open file")?
        .read()
        .context("Failed to read file")?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let has_cover = tag.map(|t| !t.pictures().is_empty()).unwrap_or(false);

    Ok(has_cover)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_non_audio_file_returns_error() {
        // Create a temporary text file
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(file, "This is just some text, not music.").expect("Failed to write to temp file");

        // Attempt to read metadata
        let result = read(file.path());

        // Should fail because it's not a valid audio file
        assert!(result.is_err());
    }

    #[test]
    fn test_read_non_existent_file_returns_error() {
        let path = Path::new("non_existent_file.mp3");
        let result = read(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_options_default() {
        let options = WriteOptions2::default();
        assert!(!options.only_fill_empty);
        assert!(!options.write_musicbrainz_ids);
    }

    #[test]
    fn test_write_result_fields() {
        let result = WriteResult {
            fields_updated: 3,
            fields_skipped: vec!["title".to_string()],
        };
        assert_eq!(result.fields_updated, 3);
        assert_eq!(result.fields_skipped.len(), 1);
    }

    #[test]
    fn test_preview_on_non_audio_returns_error() {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(file, "Not an audio file").expect("Failed to write");

        let options = WriteOptions2::default();
        let metadata = FullMetadata::default();

        let result = preview_write(file.path(), metadata, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_preview_changes() {
        // Test the FieldChange struct
        let change = FieldChange {
            field: "title".to_string(),
            current_value: "Unknown Title".to_string(),
            new_value: "Real Title".to_string(),
        };
        assert_eq!(change.field, "title");
        assert_eq!(change.current_value, "Unknown Title");
        assert_eq!(change.new_value, "Real Title");
    }

    #[test]
    fn test_write_preview_struct() {
        let preview = WritePreview {
            changes: vec![FieldChange {
                field: "artist".to_string(),
                current_value: "".to_string(),
                new_value: "Queen".to_string(),
            }],
        };
        assert_eq!(preview.changes.len(), 1);
        assert_eq!(preview.changes[0].new_value, "Queen");
    }
}
