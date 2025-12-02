//! Enrichment service - orchestrates track identification and metadata lookup
//!
//! This is the high-level API for enriching tracks:
//! 1. Generate audio fingerprint (via fpcalc)
//! 2. Look up fingerprint on AcoustID (returns MusicBrainz IDs)
//! 3. Fetch detailed metadata from MusicBrainz
//! 4. Optionally fetch cover art

use std::path::Path;
use std::time::Duration;

use crate::enrichment::{
    acoustid::AcoustIdClient,
    coverart::{CoverArt, CoverArtClient, CoverSize},
    domain::{EnrichmentError, TrackIdentification},
    fingerprint,
    musicbrainz::MusicBrainzClient,
};

/// Configuration for the enrichment service
pub struct EnrichmentConfig {
    /// AcoustID API key (get one at https://acoustid.org/new-application)
    pub acoustid_api_key: String,
    /// Minimum confidence score to accept (0.0 to 1.0)
    pub min_confidence: f32,
    /// Whether to fetch additional metadata from MusicBrainz
    pub use_musicbrainz: bool,
    /// Preferred cover art size
    pub cover_size: CoverSize,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            acoustid_api_key: String::new(),
            min_confidence: 0.8,
            use_musicbrainz: true,
            cover_size: CoverSize::Medium,
        }
    }
}

/// Service for enriching track metadata from external sources
pub struct EnrichmentService {
    config: EnrichmentConfig,
    acoustid: AcoustIdClient,
    musicbrainz: MusicBrainzClient,
    coverart: CoverArtClient,
}

impl EnrichmentService {
    /// Create a new enrichment service with the given config
    pub fn new(config: EnrichmentConfig) -> Self {
        Self {
            acoustid: AcoustIdClient::new(&config.acoustid_api_key),
            musicbrainz: MusicBrainzClient::new(),
            coverart: CoverArtClient::new(),
            config,
        }
    }

    /// Check if fingerprinting is available (fpcalc installed)
    pub fn is_fingerprinting_available(&self) -> bool {
        fingerprint::is_fpcalc_available()
    }

    /// Get fpcalc version for diagnostics
    pub fn fingerprint_version(&self) -> Option<String> {
        fingerprint::get_fpcalc_version()
    }

    /// Identify a track by its audio fingerprint
    ///
    /// Returns the best match with confidence >= min_confidence, or NoMatches error.
    /// Uses smart matching to prefer results that match existing file metadata/path.
    pub async fn identify_track(&self, path: &Path) -> Result<TrackIdentification, EnrichmentError> {
        // Step 1: Generate fingerprint
        let fp = fingerprint::generate_fingerprint(path)?;

        // Step 2: Look up on AcoustID
        let identifications = self.acoustid.lookup(&fp).await?;

        // Step 3: Read existing metadata from file for matching hints
        let existing_meta = crate::metadata::read(path).ok();

        // Step 4: Find best match using smart scoring (metadata + confidence)
        let best = identifications
            .into_iter()
            .filter(|id| id.score >= self.config.min_confidence)
            .max_by(|a, b| {
                let score_a = calculate_match_score(a, path, existing_meta.as_ref());
                let score_b = calculate_match_score(b, path, existing_meta.as_ref());
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(mut identification) = best else {
            return Err(EnrichmentError::NoMatches);
        };

        // Step 4: Optionally enrich with MusicBrainz
        if self.config.use_musicbrainz
            && let Some(ref recording_id) = identification.track.recording_id {
                // Add a small delay to respect MusicBrainz rate limits (1 req/sec)
                tokio::time::sleep(Duration::from_millis(1100)).await;

                match self.musicbrainz.lookup_recording(recording_id).await {
                    Ok(mb_result) => {
                        // Merge MusicBrainz data into our identification
                        identification.track.merge(&mb_result.track);
                    }
                    Err(e) => {
                        // Log but don't fail - AcoustID data is still useful
                        tracing::warn!("MusicBrainz lookup failed: {}", e);
                    }
                }
            }

        Ok(identification)
    }

    /// Fetch cover art for a release
    ///
    /// Requires a MusicBrainz release ID (from identify_track result).
    pub async fn get_cover_art(&self, release_id: &str) -> Result<CoverArt, EnrichmentError> {
        self.coverart
            .get_front_cover(release_id, self.config.cover_size)
            .await
    }

    /// Fetch cover art with custom size
    pub async fn get_cover_art_sized(
        &self,
        release_id: &str,
        size: CoverSize,
    ) -> Result<CoverArt, EnrichmentError> {
        self.coverart.get_front_cover(release_id, size).await
    }

    /// Identify multiple tracks, respecting rate limits
    ///
    /// Returns results in the same order as input paths.
    /// Failed identifications return None.
    pub async fn identify_tracks(
        &self,
        paths: &[&Path],
    ) -> Vec<Result<TrackIdentification, EnrichmentError>> {
        let mut results = Vec::with_capacity(paths.len());

        for (i, path) in paths.iter().enumerate() {
            let result = self.identify_track(path).await;
            results.push(result);

            // Progress logging
            if (i + 1) % 10 == 0 {
                tracing::info!("Identified {}/{} tracks", i + 1, paths.len());
            }

            // Small delay between tracks to avoid overwhelming APIs
            if i < paths.len() - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        results
    }
}

/// Quick helper to identify a single track without creating a service
pub async fn identify_track(
    path: &Path,
    acoustid_api_key: &str,
) -> Result<TrackIdentification, EnrichmentError> {
    let config = EnrichmentConfig {
        acoustid_api_key: acoustid_api_key.to_string(),
        ..Default::default()
    };
    let service = EnrichmentService::new(config);
    service.identify_track(path).await
}

/// Calculate a combined match score based on AcoustID confidence + metadata matching
///
/// This helps pick the "right" release when a track appears on multiple albums.
/// For example, if the file path contains "Greatest Hits", prefer that over "Karaoke".
fn calculate_match_score(
    identification: &TrackIdentification,
    file_path: &Path,
    existing_meta: Option<&crate::metadata::TrackMetadata>,
) -> f32 {
    let mut score = identification.score; // Start with AcoustID confidence (0.0-1.0)

    // Extract hints from file path
    let path_str = file_path.to_string_lossy().to_lowercase();
    
    // Boost score if album name matches path or existing metadata
    if let Some(ref album) = identification.track.album {
        let album_lower = album.to_lowercase();
        
        // Check if album appears in file path
        if path_str.contains(&album_lower) {
            score += 0.15; // Significant boost for path match
        }
        
        // Check if album matches existing embedded metadata
        if let Some(meta) = existing_meta {
            let existing_lower = meta.album.to_lowercase();
            if !meta.album.is_empty() && 
               (album_lower.contains(&existing_lower) || existing_lower.contains(&album_lower)) {
                score += 0.20; // Even bigger boost for embedded tag match
            }
        }
    }
    
    // Check if artist matches embedded metadata
    if let Some(ref artist) = identification.track.artist
        && let Some(meta) = existing_meta {
            let artist_lower = artist.to_lowercase();
            let existing_lower = meta.artist.to_lowercase();
            if !meta.artist.is_empty() &&
               (artist_lower.contains(&existing_lower) || existing_lower.contains(&artist_lower)) {
                score += 0.10; // Boost for artist match
            }
        }

    // Penalize undesirable release types based on secondary types
    for secondary_type in &identification.track.secondary_types {
        let type_lower = secondary_type.to_lowercase();
        match type_lower.as_str() {
            "karaoke" => score -= 0.25, // Heavily penalize karaoke versions
            "compilation" => {
                // Boost compilation if path indicates it (Greatest Hits, Best Of, etc.)
                if path_str.contains("greatest") || path_str.contains("hits") || 
                   path_str.contains("best") || path_str.contains("collection") {
                    score += 0.10; // Boost for compilation when path indicates it
                } else {
                    score -= 0.05; // Mild penalty otherwise
                }
            }
            "live" => {
                // Penalize live unless path indicates it's expected
                if !path_str.contains("live") && !path_str.contains("concert") {
                    score -= 0.10;
                }
            }
            "remix" => {
                if !path_str.contains("remix") {
                    score -= 0.15;
                }
            }
            _ => {}
        }
    }

    // Boost original studio albums (primary type = Album, no secondary types)
    if identification.track.release_type.as_deref() == Some("Album") && 
       identification.track.secondary_types.is_empty() {
        score += 0.05; // Small boost for original studio albums
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EnrichmentConfig::default();
        assert!(config.acoustid_api_key.is_empty());
        assert_eq!(config.min_confidence, 0.8);
        assert!(config.use_musicbrainz);
        assert_eq!(config.cover_size, CoverSize::Medium);
    }

    #[test]
    fn test_service_creation() {
        let config = EnrichmentConfig {
            acoustid_api_key: "test-key".to_string(),
            ..Default::default()
        };
        let service = EnrichmentService::new(config);
        
        // Just verify it doesn't panic
        let _ = service.is_fingerprinting_available();
    }
}
