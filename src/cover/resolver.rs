//! Cover art resolver - unified interface for fetching cover art.
//!
//! Resolves cover art from multiple sources with proper priority:
//! 1. Embedded in file tags (most accurate, immediate)
//! 2. Sidecar files in same directory (immediate)
//! 3. Disk cache (if previously fetched)
//! 4. Remote fetch from Cover Art Archive (background, async)
//!
//! # Design
//!
//! The resolver is designed to never block. Local sources (embedded, sidecar, cache)
//! are checked synchronously but fast. Remote fetching is always async and can be
//! triggered in the background.

use std::path::{Path, PathBuf};

use crate::enrichment::coverart::{CoverArtClient, CoverSize};

use super::cache::CoverCache;
use super::embedded::extract_embedded_cover;
use super::sidecar::find_sidecar_cover;
use super::CoverArt;

/// Where the cover art came from
#[derive(Debug, Clone, PartialEq)]
pub enum CoverSource {
    /// Embedded in the audio file's tags
    Embedded,
    /// From a sidecar file (folder.jpg, cover.png, etc.)
    Sidecar(PathBuf),
    /// From the disk cache
    Cached(PathBuf),
    /// Fetched from Cover Art Archive
    Remote,
}

/// Result of a cover art resolution
#[derive(Debug, Clone)]
pub struct CoverArtResult {
    /// The cover art, if found
    pub cover: Option<CoverArt>,
    /// Whether a remote fetch is in progress
    pub fetch_pending: bool,
}

/// Cover art resolver with caching and background fetching.
pub struct CoverResolver {
    cache: CoverCache,
    client: CoverArtClient,
}

impl CoverResolver {
    /// Create a new resolver with default cache location.
    pub fn new() -> Self {
        Self {
            cache: CoverCache::default_location(),
            client: CoverArtClient::new(),
        }
    }
    
    /// Create a resolver with a custom cache directory.
    pub fn with_cache_dir(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache: CoverCache::new(cache_dir),
            client: CoverArtClient::new(),
        }
    }
    
    /// Resolve cover art for an audio file (fast, local sources only).
    ///
    /// This checks embedded tags and sidecar files synchronously.
    /// It's designed to be fast enough to call from the UI thread.
    ///
    /// Returns immediately with local cover art if available,
    /// or None if no local source is found.
    pub fn resolve_local(&self, audio_path: &Path) -> Option<CoverArt> {
        // Priority 1: Embedded in tags (most accurate)
        if let Some(cover) = extract_embedded_cover(audio_path) {
            return Some(cover);
        }
        
        // Priority 2: Sidecar file
        if let Some(cover) = find_sidecar_cover(audio_path) {
            return Some(cover);
        }
        
        None
    }
    
    /// Resolve cover art from cache by release ID.
    pub fn resolve_cached(&self, release_id: &str) -> Option<CoverArt> {
        self.cache.get(release_id)
    }
    
    /// Resolve cover art with all sources including remote.
    ///
    /// This is an async operation that may fetch from the network.
    /// Use `resolve_local` for immediate, non-blocking resolution.
    pub async fn resolve(
        &self,
        audio_path: &Path,
        release_id: Option<&str>,
    ) -> CoverArtResult {
        // Try local sources first
        if let Some(cover) = self.resolve_local(audio_path) {
            return CoverArtResult {
                cover: Some(cover),
                fetch_pending: false,
            };
        }
        
        // Try cache
        if let Some(id) = release_id {
            if let Some(cover) = self.cache.get(id) {
                return CoverArtResult {
                    cover: Some(cover),
                    fetch_pending: false,
                };
            }
        }
        
        // Try remote fetch
        if let Some(id) = release_id {
            if let Ok(remote_cover) = self.fetch_remote(id).await {
                // Cache it
                let _ = self.cache.put(id, &remote_cover);
                return CoverArtResult {
                    cover: Some(remote_cover),
                    fetch_pending: false,
                };
            }
        }
        
        CoverArtResult {
            cover: None,
            fetch_pending: false,
        }
    }
    
    /// Fetch cover art from Cover Art Archive.
    ///
    /// This is a network operation and should be called from a background task.
    pub async fn fetch_remote(&self, release_id: &str) -> Result<CoverArt, String> {
        let result = self.client
            .get_front_cover(release_id, CoverSize::Medium)
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(CoverArt {
            data: result.data,
            mime_type: result.mime_type,
            source: CoverSource::Remote,
            album: None,
            artist: None,
        })
    }
    
    /// Pre-fetch cover art for a release in the background.
    ///
    /// This is fire-and-forget - it caches the result but doesn't block.
    pub fn prefetch_background(&self, release_id: String) -> tokio::task::JoinHandle<()> {
        let cache = CoverCache::default_location();
        let client = CoverArtClient::new();
        
        tokio::spawn(async move {
            // Skip if already cached
            if cache.contains(&release_id) {
                return;
            }
            
            // Try to fetch
            if let Ok(result) = client.get_front_cover(&release_id, CoverSize::Medium).await {
                let cover = CoverArt {
                    data: result.data,
                    mime_type: result.mime_type,
                    source: CoverSource::Remote,
                    album: None,
                    artist: None,
                };
                let _ = cache.put(&release_id, &cover);
            }
        })
    }
    
    /// Get cache statistics.
    pub fn cache_size_bytes(&self) -> u64 {
        self.cache.size_bytes()
    }
    
    /// Clear the cover art cache.
    pub fn clear_cache(&self) -> Result<(), std::io::Error> {
        self.cache.clear()
    }
}

impl Default for CoverResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_local_sidecar() {
        let temp = TempDir::new().unwrap();
        
        // Create fake audio file
        let audio_path = temp.path().join("track.mp3");
        std::fs::write(&audio_path, b"fake audio").unwrap();
        
        // Create cover.jpg
        let cover_path = temp.path().join("cover.jpg");
        std::fs::write(&cover_path, b"fake jpeg").unwrap();
        
        let resolver = CoverResolver::with_cache_dir(temp.path().join("cache"));
        let result = resolver.resolve_local(&audio_path);
        
        assert!(result.is_some());
        let cover = result.unwrap();
        assert!(matches!(cover.source, CoverSource::Sidecar(_)));
    }

    #[test]
    fn test_resolve_local_no_cover() {
        let temp = TempDir::new().unwrap();
        
        let audio_path = temp.path().join("track.mp3");
        std::fs::write(&audio_path, b"fake audio").unwrap();
        
        let resolver = CoverResolver::with_cache_dir(temp.path().join("cache"));
        let result = resolver.resolve_local(&audio_path);
        
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_cached() {
        let temp = TempDir::new().unwrap();
        let resolver = CoverResolver::with_cache_dir(temp.path());
        
        // Manually add to cache
        let cover = CoverArt {
            data: b"cached image".to_vec(),
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Remote,
            album: None,
            artist: None,
        };
        resolver.cache.put("test-release", &cover).unwrap();
        
        let result = resolver.resolve_cached("test-release");
        assert!(result.is_some());
    }
}
