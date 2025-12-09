//! Detect sidecar cover art files in the same directory as audio files.
//!
//! Common sidecar filenames:
//! - cover.jpg, cover.png
//! - folder.jpg, folder.png  
//! - album.jpg, album.png
//! - front.jpg, front.png
//! - artwork.jpg, artwork.png

use std::path::Path;

use super::{CoverArt, CoverSource};

/// Common cover art filenames (lowercase for matching)
const COVER_FILENAMES: &[&str] = &[
    "cover",
    "folder", 
    "album",
    "front",
    "artwork",
    "albumart",
    "albumartsmall",
];

/// Supported image extensions
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];

/// Find a sidecar cover art file in the same directory as the audio file.
///
/// Returns None if no cover art is found.
pub fn find_sidecar_cover(audio_path: &Path) -> Option<CoverArt> {
    let parent = audio_path.parent()?;
    
    // Try each known cover filename
    for name in COVER_FILENAMES {
        for ext in IMAGE_EXTENSIONS {
            let cover_path = parent.join(format!("{}.{}", name, ext));
            if cover_path.exists() {
                return load_sidecar_cover(&cover_path);
            }
        }
    }
    
    // Also check for case variations on case-sensitive filesystems
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            
            let file_stem = path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase());
            
            let extension = path.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase());
            
            if let (Some(stem), Some(ext)) = (file_stem, extension)
                && COVER_FILENAMES.contains(&stem.as_str()) 
                    && IMAGE_EXTENSIONS.contains(&ext.as_str()) 
                {
                    return load_sidecar_cover(&path);
                }
        }
    }
    
    None
}

/// Load cover art from a sidecar file path
fn load_sidecar_cover(path: &Path) -> Option<CoverArt> {
    let data = std::fs::read(path).ok()?;
    
    let mime_type = match path.extension().and_then(|s| s.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    };
    
    Some(CoverArt {
        data,
        mime_type: mime_type.to_string(),
        source: CoverSource::Sidecar(path.to_path_buf()),
        // Sidecar files don't have album/artist metadata
        // They're assumed to match the album in that folder
        album: None,
        artist: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_cover_jpg() {
        let temp = TempDir::new().unwrap();
        
        // Create a fake audio file
        let audio_path = temp.path().join("track.mp3");
        std::fs::write(&audio_path, b"fake audio").unwrap();
        
        // Create a cover.jpg
        let cover_path = temp.path().join("cover.jpg");
        std::fs::write(&cover_path, b"fake jpeg data").unwrap();
        
        let result = find_sidecar_cover(&audio_path);
        assert!(result.is_some());
        
        let cover = result.unwrap();
        assert_eq!(cover.mime_type, "image/jpeg");
        assert!(matches!(cover.source, CoverSource::Sidecar(_)));
    }

    #[test]
    fn test_find_folder_png() {
        let temp = TempDir::new().unwrap();
        
        let audio_path = temp.path().join("track.flac");
        std::fs::write(&audio_path, b"fake audio").unwrap();
        
        let cover_path = temp.path().join("folder.png");
        std::fs::write(&cover_path, b"fake png data").unwrap();
        
        let result = find_sidecar_cover(&audio_path);
        assert!(result.is_some());
        assert_eq!(result.unwrap().mime_type, "image/png");
    }

    #[test]
    fn test_no_cover_found() {
        let temp = TempDir::new().unwrap();
        
        let audio_path = temp.path().join("track.mp3");
        std::fs::write(&audio_path, b"fake audio").unwrap();
        
        let result = find_sidecar_cover(&audio_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_case_insensitive_match() {
        let temp = TempDir::new().unwrap();
        
        let audio_path = temp.path().join("track.mp3");
        std::fs::write(&audio_path, b"fake audio").unwrap();
        
        // Create COVER.JPG (uppercase)
        let cover_path = temp.path().join("COVER.JPG");
        std::fs::write(&cover_path, b"fake jpeg").unwrap();
        
        let result = find_sidecar_cover(&audio_path);
        assert!(result.is_some());
    }
}
