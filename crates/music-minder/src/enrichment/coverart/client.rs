//! Cover Art Archive HTTP client
//!
//! Fetches album artwork from the Cover Art Archive.
//! No API key required, but please respect their rate limits.
//!
//! API: https://coverartarchive.org

use super::dto;
use crate::enrichment::domain::EnrichmentError;

/// Desired cover art size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoverSize {
    /// 250px thumbnail
    Small,
    /// 500px thumbnail (default)
    #[default]
    Medium,
    /// 1200px thumbnail
    Large,
    /// Original full-size image
    Original,
}

/// Downloaded cover art
#[derive(Debug, Clone)]
pub struct CoverArt {
    /// Image data (JPEG or PNG)
    pub data: Vec<u8>,
    /// MIME type (image/jpeg or image/png)
    pub mime_type: String,
    /// Source URL
    pub url: String,
}

/// Cover Art Archive client
pub struct CoverArtClient {
    http_client: reqwest::Client,
    base_url: String,
}

impl CoverArtClient {
    /// Create a new client
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: "https://coverartarchive.org".to_string(),
        }
    }

    /// Create a client for testing with custom base URL
    #[cfg(test)]
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Get the front cover for a MusicBrainz release
    pub async fn get_front_cover(
        &self,
        release_id: &str,
        size: CoverSize,
    ) -> Result<CoverArt, EnrichmentError> {
        // Use the convenient redirect endpoint
        let size_suffix = match size {
            CoverSize::Small => "-250",
            CoverSize::Medium => "-500",
            CoverSize::Large => "-1200",
            CoverSize::Original => "",
        };

        let url = format!(
            "{}/release/{}/front{}",
            self.base_url, release_id, size_suffix
        );

        self.download_image(&url).await
    }

    /// List all cover art for a release
    pub async fn list_cover_art(
        &self,
        release_id: &str,
    ) -> Result<dto::CoverArtResponse, EnrichmentError> {
        let url = format!("{}/release/{}", self.base_url, release_id);

        let response = self
            .http_client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| EnrichmentError::Network(e.to_string()))?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(EnrichmentError::NoMatches);
        }

        if !status.is_success() {
            return Err(EnrichmentError::Network(format!(
                "HTTP {}: {}",
                status,
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        response
            .json::<dto::CoverArtResponse>()
            .await
            .map_err(|e| EnrichmentError::Parse(e.to_string()))
    }

    /// Download an image from a URL
    async fn download_image(&self, url: &str) -> Result<CoverArt, EnrichmentError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| EnrichmentError::Network(e.to_string()))?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(EnrichmentError::NoMatches);
        }

        if !status.is_success() {
            return Err(EnrichmentError::Network(format!(
                "HTTP {}: {}",
                status,
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        // Get content type
        let mime_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let data = response
            .bytes()
            .await
            .map_err(|e| EnrichmentError::Network(e.to_string()))?
            .to_vec();

        Ok(CoverArt {
            data,
            mime_type,
            url: url.to_string(),
        })
    }
}

impl Default for CoverArtClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CoverArtClient::new();
        assert_eq!(client.base_url, "https://coverartarchive.org");
    }

    #[test]
    fn test_cover_size_default() {
        let size = CoverSize::default();
        assert_eq!(size, CoverSize::Medium);
    }
}
