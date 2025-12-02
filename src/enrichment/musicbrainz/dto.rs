//! MusicBrainz API Data Transfer Objects
//!
//! These types match EXACTLY what the MusicBrainz API returns.
//! DO NOT add fields that aren't in the API response.
//! DO NOT use these types outside the musicbrainz module - convert to domain types.
//!
//! API Reference: https://musicbrainz.org/doc/MusicBrainz_API
//!
//! We primarily use the /recording endpoint to look up recordings by MBID
//! (obtained from AcoustID) and get full metadata.

use serde::{Deserialize, Serialize};

/// Recording lookup response (single recording with includes)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RecordingResponse {
    /// MusicBrainz recording ID
    pub id: String,
    /// Track title
    pub title: String,
    /// Duration in milliseconds
    pub length: Option<u64>,
    /// Disambiguation comment
    pub disambiguation: Option<String>,
    /// Artist credits
    #[serde(default)]
    pub artist_credit: Vec<ArtistCredit>,
    /// Releases this recording appears on
    #[serde(default)]
    pub releases: Vec<Release>,
}

/// Artist credit (can be multiple for collaborations)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtistCredit {
    /// The artist
    pub artist: Artist,
    /// How this artist is credited (may differ from official name)
    pub name: Option<String>,
    /// Join phrase (e.g., " & ", " feat. ")
    pub joinphrase: Option<String>,
}

/// Artist info
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Artist {
    /// MusicBrainz artist ID
    pub id: String,
    /// Official artist name
    pub name: String,
    /// Sort name (e.g., "Beatles, The")
    pub sort_name: Option<String>,
    /// Artist type (Person, Group, etc.)
    #[serde(rename = "type")]
    pub artist_type: Option<String>,
}

/// Release (album/single/EP)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Release {
    /// MusicBrainz release ID
    pub id: String,
    /// Release title
    pub title: String,
    /// Release status (Official, Bootleg, etc.)
    pub status: Option<String>,
    /// Release date (YYYY, YYYY-MM, or YYYY-MM-DD)
    pub date: Option<String>,
    /// Country code
    pub country: Option<String>,
    /// Release group (groups same album across editions)
    pub release_group: Option<ReleaseGroup>,
    /// Media (discs) in this release
    #[serde(default)]
    pub media: Vec<Medium>,
}

/// Release group (e.g., "Abbey Road" across all editions)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseGroup {
    /// MusicBrainz release group ID
    pub id: String,
    /// Title
    pub title: String,
    /// Primary type (Album, Single, EP, etc.)
    pub primary_type: Option<String>,
    /// First release date
    pub first_release_date: Option<String>,
}

/// Medium (disc) within a release
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Medium {
    /// Position in release (disc number)
    pub position: Option<u32>,
    /// Format (CD, Vinyl, Digital, etc.)
    pub format: Option<String>,
    /// Number of tracks
    pub track_count: Option<u32>,
    /// Tracks on this medium
    #[serde(default)]
    pub tracks: Vec<Track>,
}

/// Track on a medium
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    /// Track position on medium
    pub position: Option<u32>,
    /// Track number (may include disc prefix like "1-5")
    pub number: Option<String>,
    /// Track title (may differ from recording title)
    pub title: Option<String>,
    /// Track length in milliseconds
    pub length: Option<u64>,
}

/// Error response from MusicBrainz API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub error: String,
    pub help: Option<String>,
}

// ============================================================================
// CONTRACT TESTS
// These verify our DTOs match what the real API returns.
// If these fail, the API has changed and we need to update our DTOs.
// ============================================================================

#[cfg(test)]
mod contract_tests {
    use super::*;

    /// Test parsing a minimal recording response
    #[test]
    fn test_parse_minimal_recording() {
        let json = r#"{
            "id": "abc123",
            "title": "Test Song"
        }"#;

        let recording: RecordingResponse =
            serde_json::from_str(json).expect("Should parse minimal recording");

        assert_eq!(recording.id, "abc123");
        assert_eq!(recording.title, "Test Song");
        assert!(recording.length.is_none());
        assert!(recording.artist_credit.is_empty());
        assert!(recording.releases.is_empty());
    }

    /// Test parsing recording with artist credits
    #[test]
    fn test_parse_recording_with_artists() {
        let json = r#"{
            "id": "rec-123",
            "title": "Bohemian Rhapsody",
            "length": 354000,
            "artist-credit": [{
                "artist": {
                    "id": "art-123",
                    "name": "Queen",
                    "sort-name": "Queen",
                    "type": "Group"
                },
                "name": "Queen",
                "joinphrase": ""
            }]
        }"#;

        let recording: RecordingResponse =
            serde_json::from_str(json).expect("Should parse recording with artists");

        assert_eq!(recording.title, "Bohemian Rhapsody");
        assert_eq!(recording.length, Some(354000));
        assert_eq!(recording.artist_credit.len(), 1);

        let credit = &recording.artist_credit[0];
        assert_eq!(credit.artist.name, "Queen");
        assert_eq!(credit.artist.artist_type, Some("Group".to_string()));
    }

    /// Test parsing recording with release info
    #[test]
    fn test_parse_recording_with_releases() {
        let json = r#"{
            "id": "rec-123",
            "title": "Test Song",
            "releases": [{
                "id": "rel-123",
                "title": "Test Album",
                "status": "Official",
                "date": "1975-10-31",
                "country": "GB",
                "release-group": {
                    "id": "rg-123",
                    "title": "Test Album",
                    "primary-type": "Album",
                    "first-release-date": "1975-10-31"
                },
                "media": [{
                    "position": 1,
                    "format": "CD",
                    "track-count": 12,
                    "tracks": [{
                        "position": 5,
                        "number": "5",
                        "title": "Test Song",
                        "length": 180000
                    }]
                }]
            }]
        }"#;

        let recording: RecordingResponse =
            serde_json::from_str(json).expect("Should parse recording with releases");

        assert_eq!(recording.releases.len(), 1);
        let release = &recording.releases[0];
        assert_eq!(release.title, "Test Album");
        assert_eq!(release.date, Some("1975-10-31".to_string()));
        assert_eq!(release.status, Some("Official".to_string()));

        let rg = release.release_group.as_ref().unwrap();
        assert_eq!(rg.primary_type, Some("Album".to_string()));

        let medium = &release.media[0];
        assert_eq!(medium.track_count, Some(12));
        assert_eq!(medium.tracks[0].position, Some(5));
    }

    /// Test parsing collaboration (multiple artist credits)
    #[test]
    fn test_parse_collaboration() {
        let json = r#"{
            "id": "rec-collab",
            "title": "Under Pressure",
            "artist-credit": [
                {
                    "artist": {"id": "queen-id", "name": "Queen"},
                    "joinphrase": " & "
                },
                {
                    "artist": {"id": "bowie-id", "name": "David Bowie"},
                    "joinphrase": ""
                }
            ]
        }"#;

        let recording: RecordingResponse =
            serde_json::from_str(json).expect("Should parse collaboration");

        assert_eq!(recording.artist_credit.len(), 2);
        assert_eq!(recording.artist_credit[0].artist.name, "Queen");
        assert_eq!(recording.artist_credit[0].joinphrase, Some(" & ".to_string()));
        assert_eq!(recording.artist_credit[1].artist.name, "David Bowie");
    }

    /// Test parsing error response
    #[test]
    fn test_parse_error_response() {
        let json = r#"{
            "error": "Not Found",
            "help": "For usage, please see: https://musicbrainz.org/doc/MusicBrainz_API"
        }"#;

        let error: ApiError = serde_json::from_str(json).expect("Should parse error");
        assert_eq!(error.error, "Not Found");
        assert!(error.help.is_some());
    }
}
