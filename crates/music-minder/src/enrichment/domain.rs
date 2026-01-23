//! Internal domain models for track identification and enrichment.
//!
//! These types are OUR types - they don't change when external APIs change.
//! All external API responses get converted into these types via adapters.

use std::time::Duration;

/// Result of attempting to identify a track via audio fingerprint
#[derive(Debug, Clone)]
pub struct TrackIdentification {
    /// Confidence score (0.0 to 1.0)
    pub score: f32,
    /// The identified track info
    pub track: IdentifiedTrack,
    /// Where this identification came from
    pub source: EnrichmentSource,
    /// What level of enrichment was achieved
    pub enrichment_level: EnrichmentLevel,
}

/// Enrichment level achieved for a track.
///
/// Tracks what metadata we successfully obtained:
/// - **Minimal**: File metadata only (no external enrichment)
/// - **Basic**: AcoustID fingerprint match (title, artist, album)
/// - **Enhanced**: Basic + MusicBrainz enrichment (genres, release types, IDs)
/// - **Complete**: Enhanced + cover art downloaded
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnrichmentLevel {
    Minimal,
    Basic,
    Enhanced,
    Complete,
}

impl EnrichmentLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnrichmentLevel::Minimal => "minimal",
            EnrichmentLevel::Basic => "basic",
            EnrichmentLevel::Enhanced => "enhanced",
            EnrichmentLevel::Complete => "complete",
        }
    }
}

/// Track metadata obtained from external services
#[derive(Debug, Clone, Default)]
pub struct IdentifiedTrack {
    /// MusicBrainz recording ID (if available)
    pub recording_id: Option<String>,
    /// Track title
    pub title: Option<String>,
    /// Artist name
    pub artist: Option<String>,
    /// Album artist (may differ from track artist on compilations)
    pub album_artist: Option<String>,
    /// Album title
    pub album: Option<String>,
    /// Track number on album
    pub track_number: Option<u32>,
    /// Total tracks on album
    pub total_tracks: Option<u32>,
    /// Disc number on multi-disc release
    pub disc_number: Option<u32>,
    /// Total discs in release
    pub total_discs: Option<u32>,
    /// Release year
    pub year: Option<i32>,
    /// Track duration
    pub duration: Option<Duration>,
    /// MusicBrainz artist ID
    pub artist_id: Option<String>,
    /// MusicBrainz release (album) ID  
    pub release_id: Option<String>,
    /// MusicBrainz release group ID
    pub release_group_id: Option<String>,
    /// Release type (Album, Single, EP, etc.)
    pub release_type: Option<String>,
    /// Secondary release types (Compilation, Live, Soundtrack, etc.)
    pub secondary_types: Vec<String>,
    /// Genres/tags from MusicBrainz
    pub genres: Vec<String>,
}

/// Source of enrichment data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentSource {
    AcoustId,
    MusicBrainz,
    Manual,
}

/// Audio fingerprint for a track
#[derive(Debug, Clone)]
pub struct AudioFingerprint {
    /// The fingerprint string (Chromaprint format)
    pub fingerprint: String,
    /// Duration of the audio in seconds (required by AcoustID)
    pub duration_secs: u32,
}

/// Errors that can occur during enrichment
#[derive(Debug, Clone, thiserror::Error)]
pub enum EnrichmentError {
    #[error("Failed to generate fingerprint: {0}")]
    FingerprintError(String),

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Failed to parse response: {0}")]
    Parse(String),

    #[error("No matches found for fingerprint")]
    NoMatches,

    #[error("Rate limited - try again later")]
    RateLimited,

    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    #[error("API contract violation: expected {expected}, got {actual}")]
    ContractViolation { expected: String, actual: String },
}

/// Category of error for UI display and retry logic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Can retry automatically (network, timeout, rate limit)
    Recoverable,
    /// User can fix (missing API key, fpcalc not installed, file locked)
    Fixable,
    /// Cannot be fixed (unsupported format, file too short, no matches)
    Permanent,
}

impl ErrorCategory {
    /// Get a human-readable label for this category
    pub fn label(&self) -> &'static str {
        match self {
            ErrorCategory::Recoverable => "Network/API Errors",
            ErrorCategory::Fixable => "Fixable Issues",
            ErrorCategory::Permanent => "Unsupported",
        }
    }
}

impl EnrichmentError {
    /// Get the error category for UI display and retry logic
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Recoverable - can retry
            EnrichmentError::Network(_) => ErrorCategory::Recoverable,
            EnrichmentError::RateLimited => ErrorCategory::Recoverable,
            EnrichmentError::ApiError(msg) if msg.contains("timeout") => ErrorCategory::Recoverable,
            EnrichmentError::ApiError(msg) if msg.contains("timed out") => {
                ErrorCategory::Recoverable
            }

            // Fixable - user action needed
            EnrichmentError::FingerprintError(msg) if msg.contains("not found") => {
                ErrorCategory::Fixable
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("locked") => {
                ErrorCategory::Fixable
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("in use") => {
                ErrorCategory::Fixable
            }

            // Permanent - cannot fix
            EnrichmentError::NoMatches => ErrorCategory::Permanent,
            EnrichmentError::FingerprintError(msg) if msg.contains("Unsupported") => {
                ErrorCategory::Permanent
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("too short") => {
                ErrorCategory::Permanent
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("empty") => {
                ErrorCategory::Permanent
            }

            // Default to recoverable for unknown API errors
            _ => ErrorCategory::Recoverable,
        }
    }

    /// Get user-friendly guidance for fixing this error
    pub fn guidance(&self) -> &'static str {
        match self {
            EnrichmentError::FingerprintError(msg) if msg.contains("not found") => {
                "Install Chromaprint (fpcalc) - see Settings for instructions"
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("locked") => {
                "File is in use. Close the program using it and retry."
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("in use") => {
                "File is in use. Close the program using it and retry."
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("Unsupported") => {
                "This file format is not supported by fpcalc. Try MP3, FLAC, OGG, or WAV."
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("too short") => {
                "Audio file is too short for fingerprinting (minimum ~10 seconds)."
            }
            EnrichmentError::FingerprintError(msg) if msg.contains("empty") => {
                "File is empty or corrupted."
            }
            EnrichmentError::Network(_) => "Check your internet connection and retry.",
            EnrichmentError::RateLimited => "API rate limit reached. Wait a minute and retry.",
            EnrichmentError::NoMatches => {
                "No fingerprint match found. Try manual tagging or a different source file."
            }
            EnrichmentError::ApiError(msg) if msg.contains("timeout") => {
                "Request timed out. Check your network or try again later."
            }
            _ => "An unexpected error occurred. Check logs for details.",
        }
    }

    /// Get a short category label for grouping
    pub fn category_label(&self) -> &'static str {
        match self.category() {
            ErrorCategory::Recoverable => "Network/API Errors",
            ErrorCategory::Fixable => "Fixable Issues",
            ErrorCategory::Permanent => "Unsupported",
        }
    }
}

impl IdentifiedTrack {
    /// Merge another identification into this one, preferring non-None values
    pub fn merge(&mut self, other: &IdentifiedTrack) {
        if self.title.is_none() {
            self.title = other.title.clone();
        }
        if self.artist.is_none() {
            self.artist = other.artist.clone();
        }
        if self.album_artist.is_none() {
            self.album_artist = other.album_artist.clone();
        }
        if self.album.is_none() {
            self.album = other.album.clone();
        }
        if self.track_number.is_none() {
            self.track_number = other.track_number;
        }
        if self.total_tracks.is_none() {
            self.total_tracks = other.total_tracks;
        }
        if self.disc_number.is_none() {
            self.disc_number = other.disc_number;
        }
        if self.total_discs.is_none() {
            self.total_discs = other.total_discs;
        }
        if self.year.is_none() {
            self.year = other.year;
        }
        if self.duration.is_none() {
            self.duration = other.duration;
        }
        if self.recording_id.is_none() {
            self.recording_id = other.recording_id.clone();
        }
        if self.artist_id.is_none() {
            self.artist_id = other.artist_id.clone();
        }
        if self.release_id.is_none() {
            self.release_id = other.release_id.clone();
        }
        if self.release_group_id.is_none() {
            self.release_group_id = other.release_group_id.clone();
        }
        if self.genres.is_empty() {
            self.genres = other.genres.clone();
        }
    }
}

/// Convert IdentifiedTrack to FullMetadata for writing to audio files
impl From<IdentifiedTrack> for soundstore::metadata::FullMetadata {
    fn from(track: IdentifiedTrack) -> Self {
        soundstore::metadata::FullMetadata {
            title: track.title,
            artist: track.artist,
            album: track.album,
            album_artist: track.album_artist,
            year: track.year.map(|y| y as u32),
            genre: None,
            track_number: track.track_number,
            total_tracks: track.total_tracks,
            disc_number: track.disc_number,
            total_discs: track.total_discs,
            composer: None,
            comment: None,
            lyrics: None,
            musicbrainz_recording_id: track.recording_id,
            musicbrainz_artist_id: track.artist_id,
            musicbrainz_release_id: track.release_id,
            musicbrainz_release_group_id: track.release_group_id,
            musicbrainz_track_id: None,
            duration_secs: track.duration.map(|d| d.as_secs()).unwrap_or(0),
            bitrate: None,
            sample_rate: None,
            channels: None,
            bits_per_sample: None,
            has_cover_art: false,
            cover_art_size: None,
            format: String::new(),
            file_size: 0,
        }
    }
}

impl From<&IdentifiedTrack> for soundstore::metadata::FullMetadata {
    fn from(track: &IdentifiedTrack) -> Self {
        track.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identified_track_merge() {
        let mut track = IdentifiedTrack {
            title: Some("Song".to_string()),
            artist: None,
            ..Default::default()
        };

        let other = IdentifiedTrack {
            title: Some("Other Title".to_string()), // Should NOT override
            artist: Some("Artist".to_string()),     // Should fill in
            album: Some("Album".to_string()),       // Should fill in
            ..Default::default()
        };

        track.merge(&other);

        assert_eq!(track.title, Some("Song".to_string())); // Kept original
        assert_eq!(track.artist, Some("Artist".to_string())); // Filled in
        assert_eq!(track.album, Some("Album".to_string())); // Filled in
    }

    #[test]
    fn test_error_category_recoverable() {
        let err = EnrichmentError::Network("Connection refused".to_string());
        assert_eq!(err.category(), ErrorCategory::Recoverable);
        assert!(err.guidance().contains("internet connection"));

        let err = EnrichmentError::RateLimited;
        assert_eq!(err.category(), ErrorCategory::Recoverable);
        assert!(err.guidance().contains("rate limit"));

        let err = EnrichmentError::ApiError("Request timeout".to_string());
        assert_eq!(err.category(), ErrorCategory::Recoverable);
    }

    #[test]
    fn test_error_category_fixable() {
        let err = EnrichmentError::FingerprintError("fpcalc not found".to_string());
        assert_eq!(err.category(), ErrorCategory::Fixable);
        assert!(err.guidance().contains("Install Chromaprint"));

        let err = EnrichmentError::FingerprintError("File is locked or in use".to_string());
        assert_eq!(err.category(), ErrorCategory::Fixable);
        assert!(err.guidance().contains("in use"));
    }

    #[test]
    fn test_error_category_permanent() {
        let err = EnrichmentError::NoMatches;
        assert_eq!(err.category(), ErrorCategory::Permanent);
        assert!(err.guidance().contains("No fingerprint match"));

        let err = EnrichmentError::FingerprintError("Unsupported audio format".to_string());
        assert_eq!(err.category(), ErrorCategory::Permanent);
        assert!(err.guidance().contains("not supported"));

        let err = EnrichmentError::FingerprintError("Audio file too short".to_string());
        assert_eq!(err.category(), ErrorCategory::Permanent);
        assert!(err.guidance().contains("too short"));
    }

    #[test]
    fn test_error_category_labels() {
        let err = EnrichmentError::Network("test".to_string());
        assert_eq!(err.category_label(), "Network/API Errors");

        let err = EnrichmentError::FingerprintError("locked".to_string());
        assert_eq!(err.category_label(), "Fixable Issues");

        let err = EnrichmentError::NoMatches;
        assert_eq!(err.category_label(), "Unsupported");
    }
}
