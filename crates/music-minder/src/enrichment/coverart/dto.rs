//! Cover Art Archive API Data Transfer Objects
//!
//! The Cover Art Archive (https://coverartarchive.org) provides album artwork
//! for MusicBrainz releases. It's a free service with no API key required.
//!
//! API Reference: https://wiki.musicbrainz.org/Cover_Art_Archive/API

use serde::{Deserialize, Serialize};

/// Cover art listing for a release
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoverArtResponse {
    /// Array of images for this release
    pub images: Vec<Image>,
    /// URL of the release on MusicBrainz
    pub release: String,
}

/// A single cover art image
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    /// Whether this is the front cover
    pub front: bool,
    /// Whether this is the back cover
    pub back: bool,
    /// Image types (Front, Back, Booklet, etc.)
    pub types: Vec<String>,
    /// URL to full-size image
    pub image: String,
    /// Thumbnail URLs
    pub thumbnails: Thumbnails,
    /// Whether this is approved
    pub approved: bool,
    /// Edit ID on MusicBrainz
    pub edit: Option<i64>,
    /// Image ID
    pub id: String,
    /// Comment about the image
    pub comment: Option<String>,
}

/// Available thumbnail sizes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thumbnails {
    /// 250px thumbnail
    #[serde(rename = "250")]
    pub small: Option<String>,
    /// 500px thumbnail
    #[serde(rename = "500")]
    pub large: Option<String>,
    /// 1200px thumbnail (if available)
    #[serde(rename = "1200")]
    pub xlarge: Option<String>,
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn test_parse_cover_art_response() {
        let json = r#"{
            "images": [{
                "front": true,
                "back": false,
                "types": ["Front"],
                "image": "http://coverartarchive.org/release/abc/123.jpg",
                "thumbnails": {
                    "250": "http://coverartarchive.org/release/abc/123-250.jpg",
                    "500": "http://coverartarchive.org/release/abc/123-500.jpg"
                },
                "approved": true,
                "id": "123",
                "comment": ""
            }],
            "release": "https://musicbrainz.org/release/abc"
        }"#;

        let response: CoverArtResponse =
            serde_json::from_str(json).expect("Should parse cover art response");

        assert_eq!(response.images.len(), 1);
        assert!(response.images[0].front);
        assert!(!response.images[0].back);
        assert_eq!(response.images[0].types, vec!["Front"]);
    }

    #[test]
    fn test_parse_minimal_response() {
        let json = r#"{
            "images": [],
            "release": "https://musicbrainz.org/release/xyz"
        }"#;

        let response: CoverArtResponse =
            serde_json::from_str(json).expect("Should parse empty response");

        assert!(response.images.is_empty());
    }

    #[test]
    fn test_parse_multiple_images() {
        let json = r#"{
            "images": [
                {
                    "front": true,
                    "back": false,
                    "types": ["Front"],
                    "image": "http://example.com/front.jpg",
                    "thumbnails": {"250": "http://example.com/front-250.jpg"},
                    "approved": true,
                    "id": "1"
                },
                {
                    "front": false,
                    "back": true,
                    "types": ["Back"],
                    "image": "http://example.com/back.jpg",
                    "thumbnails": {"250": "http://example.com/back-250.jpg"},
                    "approved": true,
                    "id": "2"
                }
            ],
            "release": "https://musicbrainz.org/release/abc"
        }"#;

        let response: CoverArtResponse =
            serde_json::from_str(json).expect("Should parse multiple images");

        assert_eq!(response.images.len(), 2);
        assert!(response.images[0].front);
        assert!(response.images[1].back);
    }
}
