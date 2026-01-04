//! Cover art disk cache.
//!
//! Caches fetched cover art to avoid repeated network requests.
//! Uses the album's MusicBrainz release ID as the cache key.

use std::fs;
use std::path::PathBuf;

use super::{CoverArt, CoverSource};

/// Cover art disk cache.
pub struct CoverCache {
    cache_dir: PathBuf,
}

impl CoverCache {
    /// Create a new cache in the specified directory.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let cache_dir = cache_dir.into();
        // Ensure cache directory exists
        let _ = fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    /// Create a cache in the default location (user cache directory).
    pub fn default_location() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("music-minder")
            .join("covers");
        Self::new(cache_dir)
    }

    /// Get cached cover art for a release ID.
    pub fn get(&self, release_id: &str) -> Option<CoverArt> {
        let path = self.cache_path(release_id);
        if !path.exists() {
            return None;
        }

        let data = fs::read(&path).ok()?;

        // Determine MIME type from file extension
        let mime_type = match path.extension().and_then(|s| s.to_str()) {
            Some("png") => "image/png",
            _ => "image/jpeg",
        };

        Some(CoverArt {
            data,
            mime_type: mime_type.to_string(),
            source: CoverSource::Cached(path.clone()),
            album: None,
            artist: None,
        })
    }

    /// Store cover art in the cache.
    pub fn put(&self, release_id: &str, cover: &CoverArt) -> Result<PathBuf, std::io::Error> {
        // Determine extension from MIME type
        let ext = if cover.mime_type.contains("png") {
            "png"
        } else {
            "jpg"
        };
        let path = self.cache_dir.join(format!("{}.{}", release_id, ext));

        fs::write(&path, &cover.data)?;
        Ok(path)
    }

    /// Check if a release is cached.
    pub fn contains(&self, release_id: &str) -> bool {
        self.cache_path(release_id).exists()
    }

    /// Get the cache path for a release ID.
    fn cache_path(&self, release_id: &str) -> PathBuf {
        // Check for both jpg and png
        let jpg_path = self.cache_dir.join(format!("{}.jpg", release_id));
        if jpg_path.exists() {
            return jpg_path;
        }

        let png_path = self.cache_dir.join(format!("{}.png", release_id));
        if png_path.exists() {
            return png_path;
        }

        // Default to jpg for new entries
        jpg_path
    }

    /// Clear all cached covers.
    pub fn clear(&self) -> Result<(), std::io::Error> {
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    fs::remove_file(entry.path())?;
                }
            }
        }
        Ok(())
    }

    /// Get the total size of the cache in bytes.
    pub fn size_bytes(&self) -> u64 {
        if !self.cache_dir.exists() {
            return 0;
        }

        fs::read_dir(&self.cache_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_put_and_get() {
        let temp = TempDir::new().unwrap();
        let cache = CoverCache::new(temp.path());

        let cover = CoverArt {
            data: b"fake jpeg data".to_vec(),
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Remote,
            album: None,
            artist: None,
        };

        let result = cache.put("release-123", &cover);
        assert!(result.is_ok());

        let cached = cache.get("release-123");
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.data, b"fake jpeg data");
        assert_eq!(cached.mime_type, "image/jpeg");
    }

    #[test]
    fn test_cache_miss() {
        let temp = TempDir::new().unwrap();
        let cache = CoverCache::new(temp.path());

        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_contains() {
        let temp = TempDir::new().unwrap();
        let cache = CoverCache::new(temp.path());

        assert!(!cache.contains("release-456"));

        let cover = CoverArt {
            data: vec![1, 2, 3],
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Remote,
            album: None,
            artist: None,
        };

        cache.put("release-456", &cover).unwrap();
        assert!(cache.contains("release-456"));
    }

    #[test]
    fn test_cache_clear() {
        let temp = TempDir::new().unwrap();
        let cache = CoverCache::new(temp.path());

        let cover = CoverArt {
            data: vec![1, 2, 3],
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Remote,
            album: None,
            artist: None,
        };

        cache.put("r1", &cover).unwrap();
        cache.put("r2", &cover).unwrap();

        assert!(cache.contains("r1"));
        assert!(cache.contains("r2"));

        cache.clear().unwrap();

        assert!(!cache.contains("r1"));
        assert!(!cache.contains("r2"));
    }

    #[test]
    fn test_cache_size() {
        let temp = TempDir::new().unwrap();
        let cache = CoverCache::new(temp.path());

        assert_eq!(cache.size_bytes(), 0);

        let cover = CoverArt {
            data: vec![0; 1000],
            mime_type: "image/jpeg".to_string(),
            source: CoverSource::Remote,
            album: None,
            artist: None,
        };

        cache.put("test", &cover).unwrap();
        assert_eq!(cache.size_bytes(), 1000);
    }
}
