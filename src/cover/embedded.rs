//! Extract cover art embedded in audio file tags.
//!
//! Uses lofty to read picture data from:
//! - ID3v2 tags (MP3)
//! - Vorbis comments (FLAC, OGG)
//! - MP4 atoms (M4A/AAC)

use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::path::Path;

use super::{CoverArt, CoverSource};

/// Extract the front cover from embedded tags.
///
/// This is a fast, synchronous operation that only reads the tag data.
/// Returns None if no cover art is embedded or the file can't be read.
pub fn extract_embedded_cover(path: &Path) -> Option<CoverArt> {
    // Open and probe the file
    let tagged_file = Probe::open(path).ok()?.read().ok()?;

    // Get the primary tag
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())?;

    // Get album/artist for consistency checking
    let album = tag.album().map(|s| s.to_string());
    let artist = tag.artist().map(|s| s.to_string());

    // Find the front cover picture
    let pictures = tag.pictures();

    // Prefer front cover, fall back to first picture
    let picture = pictures
        .iter()
        .find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront)
        .or_else(|| pictures.first())?;

    let mime_type = match picture.mime_type() {
        Some(lofty::picture::MimeType::Jpeg) => "image/jpeg",
        Some(lofty::picture::MimeType::Png) => "image/png",
        Some(lofty::picture::MimeType::Gif) => "image/gif",
        Some(lofty::picture::MimeType::Bmp) => "image/bmp",
        Some(lofty::picture::MimeType::Tiff) => "image/tiff",
        _ => "image/jpeg", // Default assumption
    };

    Some(CoverArt {
        data: picture.data().to_vec(),
        mime_type: mime_type.to_string(),
        source: CoverSource::Embedded,
        album,
        artist,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_from_nonexistent_file() {
        let result = extract_embedded_cover(Path::new("nonexistent.mp3"));
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_from_non_audio_file() {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(file, "Not an audio file").expect("Failed to write");

        let result = extract_embedded_cover(file.path());
        assert!(result.is_none());
    }
}
