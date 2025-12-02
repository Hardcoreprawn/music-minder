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
    /// Album title
    pub album: Option<String>,
    /// Track number on album
    pub track_number: Option<u32>,
    /// Total tracks on album
    pub total_tracks: Option<u32>,
    /// Release year
    pub year: Option<i32>,
    /// Track duration
    pub duration: Option<Duration>,
    /// MusicBrainz artist ID
    pub artist_id: Option<String>,
    /// MusicBrainz release (album) ID  
    pub release_id: Option<String>,
    /// Release type (Album, Single, EP, etc.)
    pub release_type: Option<String>,
    /// Secondary release types (Compilation, Live, Soundtrack, etc.)
    pub secondary_types: Vec<String>,
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
#[derive(Debug, thiserror::Error)]
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

impl IdentifiedTrack {
    /// Merge another identification into this one, preferring non-None values
    pub fn merge(&mut self, other: &IdentifiedTrack) {
        if self.title.is_none() { self.title = other.title.clone(); }
        if self.artist.is_none() { self.artist = other.artist.clone(); }
        if self.album.is_none() { self.album = other.album.clone(); }
        if self.track_number.is_none() { self.track_number = other.track_number; }
        if self.total_tracks.is_none() { self.total_tracks = other.total_tracks; }
        if self.year.is_none() { self.year = other.year; }
        if self.duration.is_none() { self.duration = other.duration; }
        if self.recording_id.is_none() { self.recording_id = other.recording_id.clone(); }
        if self.artist_id.is_none() { self.artist_id = other.artist_id.clone(); }
        if self.release_id.is_none() { self.release_id = other.release_id.clone(); }
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
}
