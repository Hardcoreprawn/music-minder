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

use anyhow::{Context, Result, bail};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, ItemValue, Tag, TagExt, TagItem};
use std::fs;
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
    eprintln!("[DEBUG WRITE] Tag type: {:?}", tag_type);

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

    // Write album artist (use track.album_artist, or fall back to track.artist for consistency)
    if let Some(ref album_artist) = track.album_artist {
        let existing = tag
            .get(&ItemKey::AlbumArtist)
            .and_then(|i| i.value().text());
        if !options.only_fill_empty || existing.is_none() {
            tag.insert_text(ItemKey::AlbumArtist, album_artist.clone());
            fields_updated += 1;
        } else {
            fields_skipped.push("album_artist".to_string());
        }
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

    // Write disc number
    if let Some(disc_num) = track.disc_number {
        let existing = tag.disk();
        if !options.only_fill_empty || existing.is_none() {
            tag.set_disk(disc_num);
            fields_updated += 1;
        } else {
            fields_skipped.push("disc_number".to_string());
        }
    }

    // Write total discs
    if let Some(total_discs) = track.total_discs {
        let existing = tag.disk_total();
        if !options.only_fill_empty || existing.is_none() {
            tag.set_disk_total(total_discs);
            fields_updated += 1;
        } else {
            fields_skipped.push("total_discs".to_string());
        }
    }

    // Write genre (use first genre as primary)
    if !track.genres.is_empty() {
        let existing = tag.genre();
        if !options.only_fill_empty || existing.is_none() {
            // Join multiple genres with semicolon (common convention)
            let genre_str = track.genres.join("; ");
            tag.set_genre(genre_str);
            fields_updated += 1;
        } else {
            fields_skipped.push("genre".to_string());
        }
    }

    // Write MusicBrainz IDs if enabled
    if options.write_musicbrainz_ids {
        eprintln!("[DEBUG WRITE] Writing MusicBrainz IDs...");
        eprintln!("[DEBUG WRITE] recording_id = {:?}", track.recording_id);
        eprintln!("[DEBUG WRITE] artist_id = {:?}", track.artist_id);
        eprintln!("[DEBUG WRITE] release_id = {:?}", track.release_id);
        eprintln!(
            "[DEBUG WRITE] release_group_id = {:?}",
            track.release_group_id
        );

        // Helper to insert MusicBrainz ID - use insert_unchecked for ID3v2 since
        // the standard insert_text may not have valid mappings for these ItemKeys
        let insert_mb_id = |tag: &mut Tag, key: ItemKey, value: String| {
            // For ID3v2, MusicBrainz IDs are stored as TXXX frames
            // insert_unchecked bypasses the mapping check
            let item = TagItem::new(key, ItemValue::Text(value));
            tag.insert_unchecked(item);
        };

        if let Some(ref recording_id) = track.recording_id {
            eprintln!(
                "[DEBUG WRITE] Inserting MusicBrainzRecordingId: {}",
                recording_id
            );
            insert_mb_id(tag, ItemKey::MusicBrainzRecordingId, recording_id.clone());
            // Verify it was actually inserted
            let check = tag
                .get(&ItemKey::MusicBrainzRecordingId)
                .and_then(|i| i.value().text());
            eprintln!(
                "[DEBUG WRITE] After insert_unchecked, tag has recording_id: {:?}",
                check
            );
            fields_updated += 1;
        }
        if let Some(ref artist_id) = track.artist_id {
            insert_mb_id(tag, ItemKey::MusicBrainzArtistId, artist_id.clone());
            fields_updated += 1;
        }
        if let Some(ref release_id) = track.release_id {
            insert_mb_id(tag, ItemKey::MusicBrainzReleaseId, release_id.clone());
            fields_updated += 1;
        }
        if let Some(ref release_group_id) = track.release_group_id {
            insert_mb_id(
                tag,
                ItemKey::MusicBrainzReleaseGroupId,
                release_group_id.clone(),
            );
            fields_updated += 1;
        }
    }

    // ATOMIC WRITE: Write to temp file, verify, then replace original
    // This prevents corruption if the app crashes or power is lost mid-write
    let temp_path = path.with_extension("tmp");
    let backup_path = path.with_extension("bak");

    // Step 1: Write to temp file
    tagged_file
        .save_to_path(&temp_path, WriteOptions::default())
        .context("Failed to write tags to temp file")?;

    // Step 2: Verify the temp file is valid audio
    if let Err(e) = Probe::open(&temp_path).and_then(|p| p.read()) {
        // Clean up temp file and fail
        let _ = fs::remove_file(&temp_path);
        bail!(
            "Written file failed validation: {}. Original file unchanged.",
            e
        );
    }

    // Step 3: Rename original to backup
    if path.exists() {
        fs::rename(path, &backup_path).context("Failed to create backup of original file")?;
    }

    // Step 4: Rename temp to original
    if let Err(e) = fs::rename(&temp_path, path) {
        // Try to restore backup
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        return Err(e).context("Failed to replace original with updated file");
    }

    // Step 5: Remove backup (success!)
    let _ = fs::remove_file(&backup_path);

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

/// Write cover art to an audio file's tags
///
/// Embeds the provided image data as the front cover.
/// If `only_if_missing` is true, won't overwrite existing cover art.
pub fn write_cover_art(
    path: &Path,
    image_data: &[u8],
    mime_type: &str,
    only_if_missing: bool,
) -> Result<bool> {
    // Read the existing file
    let mut tagged_file = Probe::open(path)
        .context("Failed to open file for cover art writing")?
        .read()
        .context("Failed to read file for cover art writing")?;

    // Get the primary tag type for this format
    let tag_type = tagged_file.primary_tag_type();

    // Get or create the tag
    let tag = if let Some(tag) = tagged_file.tag_mut(tag_type) {
        tag
    } else {
        tagged_file.insert_tag(Tag::new(tag_type));
        tagged_file.tag_mut(tag_type).expect("Just inserted tag")
    };

    // Check if cover art already exists
    if only_if_missing {
        let has_front_cover = tag
            .pictures()
            .iter()
            .any(|p| matches!(p.pic_type(), PictureType::CoverFront | PictureType::Other));
        if has_front_cover {
            return Ok(false);
        }
    }

    // Remove existing front covers to avoid duplicates
    tag.remove_picture_type(PictureType::CoverFront);

    // Determine MIME type
    let mime = match mime_type {
        "image/png" => MimeType::Png,
        "image/gif" => MimeType::Gif,
        "image/bmp" => MimeType::Bmp,
        "image/tiff" => MimeType::Tiff,
        _ => MimeType::Jpeg, // Default to JPEG
    };

    // Create the picture
    let picture = Picture::new_unchecked(
        PictureType::CoverFront,
        Some(mime),
        None, // No description
        image_data.to_vec(),
    );

    // Add the picture
    tag.push_picture(picture);

    // Save the file
    tag.save_to_path(path, WriteOptions::default())
        .context("Failed to write cover art to file")?;

    Ok(true)
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
