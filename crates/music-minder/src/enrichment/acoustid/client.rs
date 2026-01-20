//! AcoustID HTTP client
//!
//! Handles communication with the AcoustID web service.
//! See: https://acoustid.org/webservice
//!
//! ## API Quirks & Best Practices
//!
//! ### URL Encoding Issue with Meta Parameter
//! The AcoustID API uses `+` as a separator in the `meta` parameter (e.g., `recordings+releasegroups`).
//! Standard URL encoding converts `+` to `%2B`, but the API does NOT recognize `%2B` as a separator.
//! When `%2B` is sent, the API returns results WITHOUT the requested metadata fields.
//!
//! **Solution**: Build the URL manually, preserving the literal `+` character.
//! Do NOT use reqwest's `.query()` method for the meta parameter.
//!
//! ### Response Compression
//! The API supports gzip-compressed responses. reqwest automatically handles decompression
//! when the `gzip` feature is enabled (via rustls-tls feature).
//!
//! ### Request Method
//! - GET: Works correctly with metadata. Fingerprints (~3400 chars) fit in URLs.
//! - POST: Recommended by API docs for large fingerprints, but empirically doesn't
//!   return metadata even with correct Content-Type and body encoding.
//!
//! We use GET since it works reliably and fingerprint sizes are manageable.

use super::{adapter, dto};
use crate::enrichment::domain::{AudioFingerprint, EnrichmentError, TrackIdentification};

/// AcoustID API client
pub struct AcoustIdClient {
    api_key: String,
    http_client: reqwest::Client,
    base_url: String,
}

impl AcoustIdClient {
    /// Create a new client with the given API key
    ///
    /// The client is configured to:
    /// - Accept gzip-compressed responses (reduces bandwidth)
    /// - Send User-Agent header identifying the application
    pub fn new(api_key: impl Into<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .gzip(true) // Accept gzip-compressed responses
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key: api_key.into(),
            http_client,
            base_url: "https://api.acoustid.org/v2/lookup".to_string(),
        }
    }

    /// Create a client for testing with custom base URL
    #[cfg(test)]
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            http_client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Look up a fingerprint and return track identifications
    pub async fn lookup(
        &self,
        fingerprint: &AudioFingerprint,
    ) -> Result<Vec<TrackIdentification>, EnrichmentError> {
        let response = self.send_lookup_request(fingerprint).await?;
        adapter::to_identifications(response)
    }

    /// Send the HTTP request and parse the response
    ///
    /// ## Implementation Notes
    ///
    /// We build the URL manually to preserve the literal `+` in `recordings+releasegroups`.
    /// Standard URL encoding would convert this to `%2B`, which the API doesn't recognize
    /// as a field separator, causing it to return results without metadata.
    ///
    /// The `compress` meta flag is included to request compressed response data from
    /// MusicBrainz (distinct from HTTP gzip compression). This reduces the amount of
    /// data transferred and processed.
    ///
    /// ## Robustness
    ///
    /// - Retries up to 3 times on transient network errors with exponential backoff
    /// - 30-second timeout per request to prevent hanging
    /// - Does NOT retry on HTTP 429 (rate limit) - those should bubble up immediately
    async fn send_lookup_request(
        &self,
        fingerprint: &AudioFingerprint,
    ) -> Result<dto::LookupResponse, EnrichmentError> {
        // CRITICAL: The + character must NOT be URL-encoded (%2B) or the API won't
        // return metadata. We manually build the URL to preserve literal + characters.
        let url = format!(
            "{}?client={}&duration={}&fingerprint={}&meta=recordings+releasegroups+compress",
            self.base_url,
            urlencoding::encode(&self.api_key),
            fingerprint.duration_secs,
            urlencoding::encode(&fingerprint.fingerprint)
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
                    "Retrying AcoustID request (attempt {}/{})",
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

            // Check HTTP status
            let status = response.status();
            if status.is_success() {
                // Success - parse and return
                return response
                    .json::<dto::LookupResponse>()
                    .await
                    .map_err(|e| EnrichmentError::Parse(e.to_string()));
            }

            // Check for rate limiting - fail immediately without retry
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(EnrichmentError::RateLimited);
            }

            // Other HTTP errors - try to get body for diagnostics
            let body = response.text().await.unwrap_or_default();
            last_error = Some(EnrichmentError::Network(format!(
                "HTTP {}: {} - {}",
                status,
                status.canonical_reason().unwrap_or("Unknown"),
                body.chars().take(200).collect::<String>()
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

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Real integration tests would use wiremock or similar
    // to mock the HTTP server. These are unit tests for the client structure.

    #[test]
    fn test_client_creation() {
        let client = AcoustIdClient::new("test-key");
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, "https://api.acoustid.org/v2/lookup");
    }

    #[test]
    fn test_client_with_custom_url() {
        let client = AcoustIdClient::with_base_url("key", "http://localhost:8080");
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
