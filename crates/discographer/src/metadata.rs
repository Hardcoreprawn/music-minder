//! Audio file metadata reading.
//!
//! Uses the lofty crate for format-independent metadata access.
//! Supports reading from MP3, FLAC, OGG, M4A, and WAV files.

use anyhow::{Context, Result};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use std::path::Path;

/// Track metadata - uses String for SQLx compatibility.
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

/// Read basic track metadata from an audio file
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
