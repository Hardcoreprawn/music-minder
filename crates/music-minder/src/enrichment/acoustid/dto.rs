//! AcoustID API Data Transfer Objects
//!
//! These types match EXACTLY what the AcoustID API returns.
//! DO NOT add fields that aren't in the API response.
//! DO NOT use these types outside the acoustid module - convert to domain types.
//!
//! API Reference: https://acoustid.org/webservice#lookup
//!
//! Example response:
//! ```json
//! {
//!   "status": "ok",
//!   "results": [{
//!     "id": "abcd1234",
//!     "score": 0.95,
//!     "recordings": [{
//!       "id": "recording-mbid",
//!       "title": "Song Title",
//!       "duration": 180,
//!       "artists": [{"id": "artist-mbid", "name": "Artist Name"}],
//!       "releases": [{"id": "release-mbid", "title": "Album"}]
//!     }]
//!   }]
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Top-level AcoustID lookup response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LookupResponse {
    pub status: String,
    #[serde(default)]
    pub results: Vec<LookupResult>,
    /// Error info if status != "ok"
    pub error: Option<ApiError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub code: i32,
    pub message: String,
}

/// A single fingerprint match result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LookupResult {
    /// AcoustID identifier
    pub id: String,
    /// Match confidence (0.0 to 1.0)
    pub score: f32,
    /// Associated MusicBrainz recordings (if meta=recordings requested)
    #[serde(default)]
    pub recordings: Vec<Recording>,
}

/// MusicBrainz recording info returned by AcoustID
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Recording {
    /// MusicBrainz recording ID
    pub id: String,
    /// Track title
    pub title: Option<String>,
    /// Duration in seconds (API returns float, e.g. 353.0)
    pub duration: Option<f64>,
    /// Artists
    #[serde(default)]
    pub artists: Vec<Artist>,
    /// Releases (albums) this recording appears on
    #[serde(default)]
    pub releases: Vec<Release>,
    /// Release groups (album groupings) this recording appears on
    #[serde(default)]
    pub releasegroups: Vec<ReleaseGroup>,
}

/// Artist info from AcoustID
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artist {
    /// MusicBrainz artist ID
    pub id: String,
    /// Artist name
    pub name: String,
}

/// Release (album) info from AcoustID
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Release {
    /// MusicBrainz release ID
    pub id: String,
    /// Album title
    pub title: Option<String>,
    /// Release country
    pub country: Option<String>,
    /// Track info within this release
    pub mediums: Option<Vec<Medium>>,
}

/// Release group info from AcoustID (when meta=releasegroups requested)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReleaseGroup {
    /// MusicBrainz release group ID
    pub id: String,
    /// Album title
    pub title: Option<String>,
    /// Primary type (Album, Single, EP, etc.)
    #[serde(rename = "type")]
    pub release_type: Option<String>,
    /// Secondary types (Compilation, Live, Soundtrack, etc.)
    #[serde(default)]
    pub secondarytypes: Vec<String>,
    /// Artists
    #[serde(default)]
    pub artists: Vec<Artist>,
}

/// Medium (disc) within a release
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Medium {
    pub position: Option<u32>,
    pub track_count: Option<u32>,
    pub tracks: Option<Vec<Track>>,
}

/// Track position within a medium
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub position: Option<u32>,
}

// ============================================================================
// CONTRACT TESTS
// These verify our DTOs match what the real API returns.
// If these fail, the API has changed and we need to update our DTOs.
// ============================================================================

#[cfg(test)]
mod contract_tests {
    use super::*;

    /// Test we can parse a minimal successful response
    #[test]
    fn test_parse_minimal_success_response() {
        let json = r#"{
            "status": "ok",
            "results": []
        }"#;

        let response: LookupResponse =
            serde_json::from_str(json).expect("Should parse minimal response");

        assert_eq!(response.status, "ok");
        assert!(response.results.is_empty());
        assert!(response.error.is_none());
    }

    /// Test we can parse a response with results
    #[test]
    fn test_parse_response_with_results() {
        let json = r#"{
            "status": "ok",
            "results": [{
                "id": "abc123",
                "score": 0.95,
                "recordings": [{
                    "id": "rec-mbid-123",
                    "title": "Test Song",
                    "duration": 180.0,
                    "artists": [{"id": "art-mbid", "name": "Test Artist"}],
                    "releases": [{"id": "rel-mbid", "title": "Test Album"}]
                }]
            }]
        }"#;

        let response: LookupResponse =
            serde_json::from_str(json).expect("Should parse response with results");

        assert_eq!(response.status, "ok");
        assert_eq!(response.results.len(), 1);

        let result = &response.results[0];
        assert_eq!(result.id, "abc123");
        assert!((result.score - 0.95).abs() < 0.001);
        assert_eq!(result.recordings.len(), 1);

        let recording = &result.recordings[0];
        assert_eq!(recording.id, "rec-mbid-123");
        assert_eq!(recording.title, Some("Test Song".to_string()));
        assert_eq!(recording.duration, Some(180.0));
        assert_eq!(recording.artists.len(), 1);
        assert_eq!(recording.artists[0].name, "Test Artist");
    }

    /// Test we can parse an error response
    #[test]
    fn test_parse_error_response() {
        let json = r#"{
            "status": "error",
            "error": {
                "code": 4,
                "message": "rate limit exceeded"
            }
        }"#;

        let response: LookupResponse =
            serde_json::from_str(json).expect("Should parse error response");

        assert_eq!(response.status, "error");
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, 4);
        assert_eq!(error.message, "rate limit exceeded");
    }

    /// Test we handle missing optional fields gracefully
    #[test]
    fn test_parse_sparse_recording() {
        let json = r#"{
            "status": "ok",
            "results": [{
                "id": "abc",
                "score": 0.5,
                "recordings": [{
                    "id": "rec-123"
                }]
            }]
        }"#;

        let response: LookupResponse =
            serde_json::from_str(json).expect("Should parse sparse recording");

        let recording = &response.results[0].recordings[0];
        assert_eq!(recording.id, "rec-123");
        assert!(recording.title.is_none());
        assert!(recording.duration.is_none());
        assert!(recording.artists.is_empty());
        assert!(recording.releases.is_empty());
    }

    /// Test we can handle response with track position info
    #[test]
    fn test_parse_response_with_track_info() {
        let json = r#"{
            "status": "ok",
            "results": [{
                "id": "abc",
                "score": 0.9,
                "recordings": [{
                    "id": "rec-123",
                    "releases": [{
                        "id": "rel-123",
                        "title": "Album",
                        "mediums": [{
                            "position": 1,
                            "track_count": 12,
                            "tracks": [{"position": 5}]
                        }]
                    }]
                }]
            }]
        }"#;

        let response: LookupResponse = serde_json::from_str(json).expect("Should parse track info");

        let release = &response.results[0].recordings[0].releases[0];
        let medium = &release.mediums.as_ref().unwrap()[0];
        assert_eq!(medium.position, Some(1));
        assert_eq!(medium.track_count, Some(12));
        assert_eq!(medium.tracks.as_ref().unwrap()[0].position, Some(5));
    }
}
