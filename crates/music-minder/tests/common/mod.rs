//! Shared test utilities for integration tests
//!
//! This module provides helpers for HTTP mocking and test data setup.

pub mod http_mocks {
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
                                        "type": "Album"
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
            let response_body = r#"{"status": "ok", "results": []}"#;

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
                    "length": 180000
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
}
