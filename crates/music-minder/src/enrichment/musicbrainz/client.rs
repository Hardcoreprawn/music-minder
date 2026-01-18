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
    ///
    /// ## Robustness
    ///
    /// - Retries up to 3 times on transient network errors with exponential backoff
    /// - 30-second timeout per request to prevent hanging
    /// - Does NOT retry on HTTP 404 (not found) or 429 (rate limit)
    async fn send_recording_request(
        &self,
        recording_id: &str,
    ) -> Result<dto::RecordingResponse, EnrichmentError> {
        let url = format!(
            "{}/recording/{}?fmt=json&inc=artists+releases+media+tags",
            self.base_url, recording_id
        );

        // Retry up to 3 times on transient network errors
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_error = None;

        while attempts < max_attempts {
            attempts += 1;

            // Exponential backoff: 0ms, 500ms, 1000ms
            if attempts > 1 {
                let delay_ms = (attempts - 1) * 500;
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms as u64)).await;
                tracing::debug!(
                    "Retrying MusicBrainz request (attempt {}/{})",
                    attempts,
                    max_attempts
                );
            }

            // Send request with 30-second timeout
            let request_future = self.http_client.get(&url).send();
            let timeout_duration = std::time::Duration::from_secs(30);

            let response = match tokio::time::timeout(timeout_duration, request_future).await {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => {
                    // Check if this is a transient error worth retrying
                    if Self::is_transient_error(&e) {
                        last_error = Some(EnrichmentError::Network(format!(
                            "Transient network error: {}",
                            e
                        )));
                        continue;
                    }
                    // Non-transient error - fail immediately
                    return Err(EnrichmentError::Network(e.to_string()));
                }
                Err(_) => {
                    last_error = Some(EnrichmentError::Network(format!(
                        "Request timeout after {}s (attempt {}/{})",
                        timeout_duration.as_secs(),
                        attempts,
                        max_attempts
                    )));
                    continue;
                }
            };

            let status = response.status();

            // Check for non-retryable errors first
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(EnrichmentError::NoMatches);
            }

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(EnrichmentError::RateLimited);
            }

            if status.is_success() {
                // Success - parse and return
                return response
                    .json::<dto::RecordingResponse>()
                    .await
                    .map_err(|e| EnrichmentError::Parse(e.to_string()));
            }

            // Try to parse error response for diagnostics
            let error_detail = if let Ok(error) = response.json::<dto::ApiError>().await {
                error.error
            } else {
                status.canonical_reason().unwrap_or("Unknown").to_string()
            };

            last_error = Some(EnrichmentError::ApiError(format!(
                "HTTP {}: {}",
                status, error_detail
            )));

            // Retry server errors (5xx), fail immediately on client errors (4xx)
            if status.is_client_error() {
                return Err(last_error.unwrap());
            }
            // Continue to retry on server errors (5xx)
        }

        // All retries exhausted
        Err(last_error
            .unwrap_or_else(|| EnrichmentError::Network("Max retries exceeded".to_string())))
    }

    /// Check if an error is transient and worth retrying
    fn is_transient_error(e: &reqwest::Error) -> bool {
        // Retry on connection errors, timeouts, etc.
        e.is_connect() || e.is_timeout() || e.is_request()
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
