//! MusicBrainz HTTP client
//!
//! Handles communication with the MusicBrainz web service.
//! See: https://musicbrainz.org/doc/MusicBrainz_API
//!
//! IMPORTANT: MusicBrainz requires a User-Agent header and rate limits to 1 req/sec.

use super::{adapter, dto};
use crate::enrichment::domain::{EnrichmentError, TrackIdentification};

/// MusicBrainz API client
pub struct MusicBrainzClient {
    http_client: reqwest::Client,
    base_url: String,
}

/// User agent string - MusicBrainz requires this
const USER_AGENT: &str = concat!(
    "MusicMinder/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/music-minder)"
);

impl MusicBrainzClient {
    /// Create a new client
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            base_url: "https://musicbrainz.org/ws/2".to_string(),
        }
    }

    /// Create a client for testing with custom base URL
    #[cfg(test)]
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            base_url: base_url.into(),
        }
    }

    /// Look up a recording by MusicBrainz ID and return enriched track info
    pub async fn lookup_recording(
        &self,
        recording_id: &str,
    ) -> Result<TrackIdentification, EnrichmentError> {
        let response = self.send_recording_request(recording_id).await?;
        Ok(adapter::to_identification(response))
    }

    /// Send the HTTP request and parse the response
    async fn send_recording_request(
        &self,
        recording_id: &str,
    ) -> Result<dto::RecordingResponse, EnrichmentError> {
        let url = format!(
            "{}/recording/{}?fmt=json&inc=artists+releases+media",
            self.base_url, recording_id
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EnrichmentError::Network(e.to_string()))?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(EnrichmentError::NoMatches);
        }

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(EnrichmentError::RateLimited);
        }

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error) = response.json::<dto::ApiError>().await {
                return Err(EnrichmentError::ApiError(error.error));
            }
            return Err(EnrichmentError::Network(format!(
                "HTTP {}: {}",
                status,
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        response
            .json::<dto::RecordingResponse>()
            .await
            .map_err(|e| EnrichmentError::Parse(e.to_string()))
    }
}

impl Default for MusicBrainzClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MusicBrainzClient::new();
        assert_eq!(client.base_url, "https://musicbrainz.org/ws/2");
    }

    #[test]
    fn test_client_with_custom_url() {
        let client = MusicBrainzClient::with_base_url("http://localhost:8080");
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_user_agent_format() {
        assert!(USER_AGENT.starts_with("MusicMinder/"));
    }
}
