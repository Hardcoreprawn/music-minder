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

use anyhow::{Context, Result};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag, TagExt};
use std::path::Path;

use crate::enrichment::domain::IdentifiedTrack;

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

/// Write enrichment data to an audio file's tags
///
/// This updates the file's embedded metadata tags with the identified track info.
/// Supports MP3 (ID3v2), FLAC, M4A/AAC, OGG Vorbis, and other formats via lofty.
pub fn write(path: &Path, track: &IdentifiedTrack, options: &WriteOptions2) -> Result<WriteResult> {
    // Read the existing file
    let mut tagged_file = Probe::open(path)
        .context("Failed to open file for writing")?
        .read()
        .context("Failed to read file for tag writing")?;

    // Get the primary tag type for this format, or create one
    let tag_type = tagged_file.primary_tag_type();

    // Get or create the tag
    let tag = if let Some(tag) = tagged_file.tag_mut(tag_type) {
        tag
    } else {
        // Insert a new tag of the appropriate type
        tagged_file.insert_tag(Tag::new(tag_type));
        tagged_file.tag_mut(tag_type).expect("Just inserted tag")
    };

    let mut fields_updated = 0;
    let mut fields_skipped = Vec::new();

    // Helper to check if we should write a field
    let should_write =
        |existing: Option<&str>, field_name: &str, skipped: &mut Vec<String>| -> bool {
            if options.only_fill_empty {
                let dominated = existing
                    .map(|s| {
                        !s.is_empty()
                            && s != "Unknown Title"
                            && s != "Unknown Artist"
                            && s != "Unknown Album"
                    })
                    .unwrap_or(false);
                if dominated {
                    skipped.push(field_name.to_string());
                    return false;
                }
            }
            true
        };

    // Write title
    if let Some(ref title) = track.title
        && should_write(tag.title().as_deref(), "title", &mut fields_skipped)
    {
        tag.set_title(title.clone());
        fields_updated += 1;
    }

    // Write artist
    if let Some(ref artist) = track.artist
        && should_write(tag.artist().as_deref(), "artist", &mut fields_skipped)
    {
        tag.set_artist(artist.clone());
        fields_updated += 1;
    }

    // Write album
    if let Some(ref album) = track.album
        && should_write(tag.album().as_deref(), "album", &mut fields_skipped)
    {
        tag.set_album(album.clone());
        fields_updated += 1;
    }

    // Write track number
    if let Some(track_num) = track.track_number {
        let existing = tag.track();
        if !options.only_fill_empty || existing.is_none() {
            tag.set_track(track_num);
            fields_updated += 1;
        } else {
            fields_skipped.push("track_number".to_string());
        }
    }

    // Write total tracks
    if let Some(total) = track.total_tracks {
        let existing = tag.track_total();
        if !options.only_fill_empty || existing.is_none() {
            tag.set_track_total(total);
            fields_updated += 1;
        } else {
            fields_skipped.push("total_tracks".to_string());
        }
    }

    // Write year
    if let Some(year) = track.year {
        let existing = tag.year();
        if !options.only_fill_empty || existing.is_none() {
            tag.set_year(year as u32);
            fields_updated += 1;
        } else {
            fields_skipped.push("year".to_string());
        }
    }

    // Write MusicBrainz IDs if enabled
    if options.write_musicbrainz_ids {
        if let Some(ref recording_id) = track.recording_id {
            tag.insert_text(ItemKey::MusicBrainzRecordingId, recording_id.clone());
            fields_updated += 1;
        }
        if let Some(ref artist_id) = track.artist_id {
            tag.insert_text(ItemKey::MusicBrainzArtistId, artist_id.clone());
            fields_updated += 1;
        }
        if let Some(ref release_id) = track.release_id {
            tag.insert_text(ItemKey::MusicBrainzReleaseId, release_id.clone());
            fields_updated += 1;
        }
    }

    // Save the file
    tag.save_to_path(path, WriteOptions::default())
        .context("Failed to write tags to file")?;

    Ok(WriteResult {
        fields_updated,
        fields_skipped,
    })
}

/// Preview what changes would be made without actually writing
pub fn preview_write(
    path: &Path,
    track: &IdentifiedTrack,
    options: &WriteOptions2,
) -> Result<WritePreview> {
    let current = read(path)?;

    let mut changes = Vec::new();

    // Helper to add a change
    let mut add_change = |field: &str, current_val: &str, new_val: Option<&str>| {
        if let Some(new) = new_val {
            let is_unknown = current_val.starts_with("Unknown") || current_val.is_empty();
            if !options.only_fill_empty || is_unknown {
                changes.push(FieldChange {
                    field: field.to_string(),
                    current_value: current_val.to_string(),
                    new_value: new.to_string(),
                });
            }
        }
    };

    add_change("title", &current.title, track.title.as_deref());
    add_change("artist", &current.artist, track.artist.as_deref());
    add_change("album", &current.album, track.album.as_deref());

    if let Some(track_num) = track.track_number {
        let current_str = current
            .track_number
            .map(|n| n.to_string())
            .unwrap_or_default();
        if !options.only_fill_empty || current.track_number.is_none() {
            changes.push(FieldChange {
                field: "track_number".to_string(),
                current_value: current_str,
                new_value: track_num.to_string(),
            });
        }
    }

    if let Some(year) = track.year {
        changes.push(FieldChange {
            field: "year".to_string(),
            current_value: String::new(),
            new_value: year.to_string(),
        });
    }

    Ok(WritePreview { changes })
}

/// A preview of changes that would be made
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

        let track = IdentifiedTrack::default();
        let options = WriteOptions2::default();

        let result = preview_write(file.path(), &track, &options);
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
