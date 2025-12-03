//! Trait definitions for external API clients.
//!
//! These traits enable dependency injection and mocking for tests.
//! Production code uses the real client implementations, while tests
//! can substitute mock implementations.
//!
//! # Example
//!
//! ```ignore
//! use music_minder::enrichment::traits::AcoustIdApi;
//!
//! // In production code:
//! fn process<T: AcoustIdApi>(client: &T, fp: &AudioFingerprint) {
//!     let results = client.lookup(fp).await?;
//! }
//!
//! // In tests:
//! struct MockAcoustId { ... }
//! impl AcoustIdApi for MockAcoustId { ... }
//! ```

use async_trait::async_trait;

use super::coverart::{CoverArt, CoverSize};
use super::domain::{AudioFingerprint, EnrichmentError, TrackIdentification};

/// Trait for AcoustID fingerprint lookup.
///
/// Implement this trait to create mock implementations for testing.
#[async_trait]
pub trait AcoustIdApi: Send + Sync {
    /// Look up a fingerprint and return possible track identifications.
    async fn lookup(
        &self,
        fingerprint: &AudioFingerprint,
    ) -> Result<Vec<TrackIdentification>, EnrichmentError>;
}

/// Trait for MusicBrainz metadata lookup.
///
/// Implement this trait to create mock implementations for testing.
#[async_trait]
pub trait MusicBrainzApi: Send + Sync {
    /// Look up a recording by its MusicBrainz ID.
    async fn lookup_recording(
        &self,
        recording_id: &str,
    ) -> Result<TrackIdentification, EnrichmentError>;
}

/// Trait for Cover Art Archive lookup.
///
/// Implement this trait to create mock implementations for testing.
#[async_trait]
pub trait CoverArtApi: Send + Sync {
    /// Get the front cover for a release.
    async fn get_front_cover(
        &self,
        release_id: &str,
        size: CoverSize,
    ) -> Result<CoverArt, EnrichmentError>;
}

// Implement traits for real clients

#[async_trait]
impl AcoustIdApi for super::acoustid::AcoustIdClient {
    async fn lookup(
        &self,
        fingerprint: &AudioFingerprint,
    ) -> Result<Vec<TrackIdentification>, EnrichmentError> {
        self.lookup(fingerprint).await
    }
}

#[async_trait]
impl MusicBrainzApi for super::musicbrainz::MusicBrainzClient {
    async fn lookup_recording(
        &self,
        recording_id: &str,
    ) -> Result<TrackIdentification, EnrichmentError> {
        self.lookup_recording(recording_id).await
    }
}

#[async_trait]
impl CoverArtApi for super::coverart::CoverArtClient {
    async fn get_front_cover(
        &self,
        release_id: &str,
        size: CoverSize,
    ) -> Result<CoverArt, EnrichmentError> {
        self.get_front_cover(release_id, size).await
    }
}

/// Mock AcoustID client for testing.
///
/// Returns configurable responses for testing different scenarios.
#[cfg(test)]
pub mod mocks {
    use super::*;
    use crate::enrichment::domain::IdentifiedTrack;

    /// Mock AcoustID client that returns predefined results.
    pub struct MockAcoustId {
        /// Results to return from lookup
        pub results: Vec<TrackIdentification>,
        /// Error to return (takes precedence over results)
        pub error: Option<EnrichmentError>,
    }

    impl MockAcoustId {
        /// Create a mock that returns no matches.
        pub fn no_matches() -> Self {
            Self {
                results: vec![],
                error: None,
            }
        }

        /// Create a mock that returns a single match.
        pub fn single_match(title: &str, artist: &str, score: f32) -> Self {
            Self {
                results: vec![TrackIdentification {
                    score,
                    track: IdentifiedTrack {
                        title: Some(title.to_string()),
                        artist: Some(artist.to_string()),
                        album: None,
                        year: None,
                        track_number: None,
                        recording_id: Some("mock-recording-id".to_string()),
                        release_id: None,
                        release_type: None,
                        secondary_types: vec![],
                        ..Default::default()
                    },
                    source: crate::enrichment::domain::EnrichmentSource::AcoustId,
                }],
                error: None,
            }
        }

        /// Create a mock that returns an error.
        pub fn with_error(error: EnrichmentError) -> Self {
            Self {
                results: vec![],
                error: Some(error),
            }
        }
    }

    #[async_trait]
    impl AcoustIdApi for MockAcoustId {
        async fn lookup(
            &self,
            _fingerprint: &AudioFingerprint,
        ) -> Result<Vec<TrackIdentification>, EnrichmentError> {
            if let Some(ref err) = self.error {
                return Err(err.clone());
            }
            Ok(self.results.clone())
        }
    }

    /// Mock MusicBrainz client that returns predefined results.
    pub struct MockMusicBrainz {
        /// Result to return from lookup
        pub result: Option<TrackIdentification>,
        /// Error to return (takes precedence over result)
        pub error: Option<EnrichmentError>,
    }

    impl MockMusicBrainz {
        /// Create a mock that returns a result with enriched metadata.
        pub fn with_metadata(album: &str, year: u32) -> Self {
            Self {
                result: Some(TrackIdentification {
                    score: 1.0,
                    track: IdentifiedTrack {
                        title: None,
                        artist: None,
                        album: Some(album.to_string()),
                        year: Some(year as i32),
                        track_number: Some(1),
                        recording_id: None,
                        release_id: Some("mock-release-id".to_string()),
                        release_type: Some("Album".to_string()),
                        secondary_types: vec![],
                        ..Default::default()
                    },
                    source: crate::enrichment::domain::EnrichmentSource::MusicBrainz,
                }),
                error: None,
            }
        }

        /// Create a mock that returns an error.
        pub fn with_error(error: EnrichmentError) -> Self {
            Self {
                result: None,
                error: Some(error),
            }
        }
    }

    #[async_trait]
    impl MusicBrainzApi for MockMusicBrainz {
        async fn lookup_recording(
            &self,
            _recording_id: &str,
        ) -> Result<TrackIdentification, EnrichmentError> {
            if let Some(ref err) = self.error {
                return Err(err.clone());
            }
            self.result.clone().ok_or(EnrichmentError::NoMatches)
        }
    }

    /// Mock Cover Art client.
    pub struct MockCoverArt {
        /// Error to return
        pub error: Option<EnrichmentError>,
    }

    impl MockCoverArt {
        /// Create a mock that returns a placeholder cover.
        pub fn with_placeholder() -> Self {
            Self { error: None }
        }

        /// Create a mock that returns an error.
        pub fn with_error(error: EnrichmentError) -> Self {
            Self { error: Some(error) }
        }
    }

    #[async_trait]
    impl CoverArtApi for MockCoverArt {
        async fn get_front_cover(
            &self,
            release_id: &str,
            _size: CoverSize,
        ) -> Result<CoverArt, EnrichmentError> {
            if let Some(ref err) = self.error {
                return Err(err.clone());
            }
            Ok(CoverArt {
                url: format!("https://coverart.example.com/{}", release_id),
                data: vec![0u8; 100], // Placeholder image data
                mime_type: "image/jpeg".to_string(),
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_acoustid_no_matches() {
            let mock = MockAcoustId::no_matches();
            let fp = AudioFingerprint {
                fingerprint: "test".to_string(),
                duration_secs: 180,
            };
            let results = mock.lookup(&fp).await.unwrap();
            assert!(results.is_empty());
        }

        #[tokio::test]
        async fn test_mock_acoustid_single_match() {
            let mock = MockAcoustId::single_match("Test Song", "Test Artist", 0.95);
            let fp = AudioFingerprint {
                fingerprint: "test".to_string(),
                duration_secs: 180,
            };
            let results = mock.lookup(&fp).await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].track.title.as_deref(), Some("Test Song"));
            assert_eq!(results[0].score, 0.95);
        }

        #[tokio::test]
        async fn test_mock_acoustid_error() {
            let mock = MockAcoustId::with_error(EnrichmentError::Network("timeout".to_string()));
            let fp = AudioFingerprint {
                fingerprint: "test".to_string(),
                duration_secs: 180,
            };
            let result = mock.lookup(&fp).await;
            assert!(matches!(result, Err(EnrichmentError::Network(_))));
        }

        #[tokio::test]
        async fn test_mock_musicbrainz_with_metadata() {
            let mock = MockMusicBrainz::with_metadata("Test Album", 2020);
            let result = mock.lookup_recording("some-id").await.unwrap();
            assert_eq!(result.track.album.as_deref(), Some("Test Album"));
            assert_eq!(result.track.year, Some(2020));
        }

        #[tokio::test]
        async fn test_mock_coverart() {
            let mock = MockCoverArt::with_placeholder();
            let result = mock
                .get_front_cover("release-123", CoverSize::Medium)
                .await
                .unwrap();
            assert!(result.url.contains("release-123"));
            assert!(!result.data.is_empty());
        }
    }
}
