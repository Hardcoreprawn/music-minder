//! Cover art resolution and caching.
//!
//! This module provides a unified interface for resolving album cover art from
//! multiple sources with proper priority ordering:
//!
//! 1. **Embedded tags** - Cover art embedded in the audio file (most accurate)
//! 2. **Sidecar files** - folder.jpg, cover.jpg, etc. in the same directory
//! 3. **Remote fetch** - Cover Art Archive (background, cached)
//!
//! # Design Principles
//!
//! - **Non-blocking**: All operations are async and never block audio playback
//! - **Graceful degradation**: Missing art is fine, just returns None
//! - **Consistency**: Cover art must match the album in tags
//! - **Caching**: Fetched art is cached to disk to avoid repeated network calls

mod embedded;
mod sidecar;
mod cache;
mod resolver;

pub use cache::CoverCache;
pub use resolver::{CoverResolver, CoverArtResult, CoverSource};

/// Cover art data ready for display
#[derive(Debug, Clone)]
pub struct CoverArt {
    /// Raw image data (JPEG or PNG)
    pub data: Vec<u8>,
    /// MIME type (image/jpeg, image/png)
    pub mime_type: String,
    /// Where this cover came from
    pub source: CoverSource,
    /// Album this cover is for (for consistency checking)
    pub album: Option<String>,
    /// Artist this cover is for (for consistency checking)  
    pub artist: Option<String>,
}

impl CoverArt {
    /// Check if this cover art matches the expected album/artist
    pub fn matches(&self, album: Option<&str>, artist: Option<&str>) -> bool {
        // If we don't have metadata to compare, assume it matches
        let album_matches = match (&self.album, album) {
            (Some(cover_album), Some(expected)) => {
                cover_album.eq_ignore_ascii_case(expected)
            }
            _ => true, // No metadata to compare
        };
        
        let artist_matches = match (&self.artist, artist) {
            (Some(cover_artist), Some(expected)) => {
                cover_artist.eq_ignore_ascii_case(expected)
            }
            _ => true,
        };
        
        album_matches && artist_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_art_matches() {
        let cover = CoverArt {
            data: vec![],
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Embedded,
            album: Some("Back in Black".to_string()),
            artist: Some("AC/DC".to_string()),
        };

        assert!(cover.matches(Some("Back in Black"), Some("AC/DC")));
        assert!(cover.matches(Some("back in black"), Some("ac/dc"))); // Case insensitive
        assert!(cover.matches(None, None)); // No metadata to compare
        assert!(!cover.matches(Some("Highway to Hell"), Some("AC/DC"))); // Wrong album
    }

    #[test]
    fn test_cover_art_matches_partial_metadata() {
        let cover = CoverArt {
            data: vec![],
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Embedded,
            album: None, // No album metadata on cover
            artist: Some("Queen".to_string()),
        };

        // Should match since we can't compare album
        assert!(cover.matches(Some("A Night at the Opera"), Some("Queen")));
    }
}
