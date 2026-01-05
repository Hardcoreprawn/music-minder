//! Test utilities for mocking enrichment APIs
//!
//! Provides wiremock-based HTTP mocking for AcoustID, MusicBrainz, and Cover Art Archive APIs.
//! These utilities enable integration testing without hitting real endpoints or requiring API keys.

use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock AcoustID API server for testing
pub struct MockAcoustIdServer {
    pub server: MockServer,
}

impl MockAcoustIdServer {
    /// Start a new mock AcoustID server
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    /// Get the base URL for the mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Mock a successful AcoustID lookup with metadata
    pub async fn mock_lookup_success(&self) {
        let response_body = r#"{
            "status": "ok",
            "results": [
                {
                    "id": "e8afe38a-8044-40d8-a708-191165c86742",
                    "recordings": [
                        {
                            "id": "4f4868cb-dcc2-4998-bafe-a97bdda95e7f",
                            "score": 100,
                            "title": "Test Track",
                            "artists": [
                                {
                                    "id": "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d",
                                    "name": "Test Artist"
                                }
                            ],
                            "releasegroups": [
                                {
                                    "id": "550e8400-e29b-41d4-a716-446655440000",
                                    "title": "Test Album",
                                    "type": "Album",
                                    "releases": [
                                        {
                                            "id": "12345678-1234-1234-1234-123456789012",
                                            "title": "Test Album",
                                            "date": "2024-01-01",
                                            "country": "US",
                                            "track_count": 12
                                        }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&self.server)
            .await;
    }

    /// Mock an AcoustID lookup with no matches
    pub async fn mock_lookup_no_matches(&self) {
        let response_body = r#"{
            "status": "ok",
            "results": []
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&self.server)
            .await;
    }

    /// Mock an AcoustID server error
    pub async fn mock_lookup_error(&self, status: u16) {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status).set_body_string("Server error"))
            .mount(&self.server)
            .await;
    }
}

/// Mock MusicBrainz API server for testing
pub struct MockMusicBrainzServer {
    pub server: MockServer,
}

impl MockMusicBrainzServer {
    /// Start a new mock MusicBrainz server
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    /// Get the base URL for the mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Mock a successful recording lookup
    pub async fn mock_recording_lookup_success(&self) {
        let response_body = r#"{
            "recording": {
                "id": "4f4868cb-dcc2-4998-bafe-a97bdda95e7f",
                "title": "Test Track",
                "length": 180000,
                "artist-credit": [
                    {
                        "artist": {
                            "id": "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d",
                            "name": "Test Artist"
                        }
                    }
                ],
                "releases": [
                    {
                        "id": "12345678-1234-1234-1234-123456789012",
                        "title": "Test Album",
                        "date": "2024-01-01",
                        "release-group": {
                            "id": "550e8400-e29b-41d4-a716-446655440000",
                            "type": "Album"
                        },
                        "track-count": 12
                    }
                ]
            }
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&self.server)
            .await;
    }

    /// Mock a MusicBrainz 404 (not found)
    pub async fn mock_recording_not_found(&self) {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
            .mount(&self.server)
            .await;
    }
}

/// Mock Cover Art Archive for testing
pub struct MockCoverArtServer {
    pub server: MockServer,
}

impl MockCoverArtServer {
    /// Start a new mock Cover Art Archive server
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    /// Get the base URL for the mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Mock a successful cover art lookup (returns JSON metadata)
    pub async fn mock_coverart_success(&self) {
        let response_body = r#"{
            "images": [
                {
                    "types": ["Front"],
                    "front": true,
                    "back": false,
                    "edit": 123456789,
                    "id": "550e8400e29b41d4a716446655440000",
                    "approved": true,
                    "comment": "",
                    "mime": "image/jpeg",
                    "thumbnails": {
                        "small": "https://coverartarchive.org/release/550e8400-e29b-41d4-a716-446655440000/123456789-small.jpg",
                        "large": "https://coverartarchive.org/release/550e8400-e29b-41d4-a716-446655440000/123456789-large.jpg"
                    }
                }
            ],
            "release": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&self.server)
            .await;
    }

    /// Mock a cover art archive 404 (no artwork)
    pub async fn mock_coverart_not_found(&self) {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_acoustid_server_starts() {
        let mock = MockAcoustIdServer::start().await;
        let url = mock.base_url();
        assert!(url.contains("http://127.0.0.1"));
    }

    #[tokio::test]
    async fn test_mock_musicbrainz_server_starts() {
        let mock = MockMusicBrainzServer::start().await;
        let url = mock.base_url();
        assert!(url.contains("http://127.0.0.1"));
    }

    #[tokio::test]
    async fn test_mock_coverart_server_starts() {
        let mock = MockCoverArtServer::start().await;
        let url = mock.base_url();
        assert!(url.contains("http://127.0.0.1"));
    }
}
