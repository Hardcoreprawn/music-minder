//! Metadata quality assessment for music files.
//!
//! This module evaluates how "complete" and "trustworthy" a track's metadata is,
//! flagging files that could benefit from enrichment. It's designed to run in the
//! background, gradually nurturing your music library over time.
//!
//! # Quality Flags
//!
//! Each flag represents a potential improvement opportunity:
//! - `missing_artist` - No artist information
//! - `missing_album` - No album information  
//! - `missing_year` - No release year
//! - `title_is_filename` - Title appears to be derived from filename
//! - `generic_metadata` - Contains placeholder text like "Unknown Artist"
//! - `no_musicbrainz_id` - No MusicBrainz ID for verification
//! - `low_confidence` - Identification match was uncertain
//! - `better_match_available` - A higher-confidence match exists
//!
//! # Quality Score
//!
//! Tracks receive a score from 0-100:
//! - 90-100: Excellent - fully tagged with high confidence
//! - 70-89: Good - minor gaps but usable
//! - 50-69: Fair - significant metadata missing
//! - 0-49: Poor - needs attention

use bitflags::bitflags;

bitflags! {
    /// Flags indicating metadata quality issues.
    ///
    /// Multiple flags can be set simultaneously. Use `.is_empty()` to check
    /// if a track has no quality issues.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct QualityFlags: u32 {
        // === Missing Metadata ===
        /// No artist metadata
        const MISSING_ARTIST = 1 << 0;
        /// No album metadata
        const MISSING_ALBUM = 1 << 1;
        /// No release year
        const MISSING_YEAR = 1 << 2;
        /// No track number
        const MISSING_TRACK_NUM = 1 << 3;

        // === Suspicious Metadata ===
        /// Title appears to be derived from filename
        const TITLE_IS_FILENAME = 1 << 4;
        /// Contains generic placeholder text
        const GENERIC_METADATA = 1 << 5;

        // === Identification Status ===
        /// No MusicBrainz recording ID
        const NO_MUSICBRAINZ_ID = 1 << 6;
        /// Identification confidence was low (<0.7)
        const LOW_CONFIDENCE = 1 << 7;
        /// A better match might be available
        const BETTER_MATCH_AVAILABLE = 1 << 8;
        /// File has never been checked
        const NEVER_CHECKED = 1 << 9;
        /// File changed since last check
        const FILE_CHANGED = 1 << 10;

        // === Verification Status (fingerprint vs metadata) ===
        /// Title doesn't match fingerprint result
        const TITLE_MISMATCH = 1 << 11;
        /// Artist doesn't match fingerprint result
        const ARTIST_MISMATCH = 1 << 12;
        /// Album doesn't match fingerprint result (may be compilation)
        const ALBUM_MISMATCH = 1 << 13;
        /// Track might be mislabeled (significant mismatch)
        const POSSIBLY_MISLABELED = 1 << 14;
        /// Track verified against fingerprint database
        const VERIFIED = 1 << 15;
        /// Fingerprint found but no match in database
        const UNIDENTIFIED = 1 << 16;
        /// Multiple good matches exist (ambiguous)
        const AMBIGUOUS_MATCH = 1 << 17;
        /// Recording appears on multiple albums (compilation candidate)
        const MULTI_ALBUM = 1 << 18;

        // === Composite flags for common checks ===
        /// Any mismatch between metadata and fingerprint
        const ANY_MISMATCH = Self::TITLE_MISMATCH.bits()
            | Self::ARTIST_MISMATCH.bits()
            | Self::ALBUM_MISMATCH.bits();
        /// Any critical issue needing attention
        const NEEDS_REVIEW = Self::POSSIBLY_MISLABELED.bits()
            | Self::AMBIGUOUS_MATCH.bits()
            | Self::UNIDENTIFIED.bits();
    }
}

impl QualityFlags {
    /// Get human-readable descriptions of all set flags.
    pub fn descriptions(&self) -> Vec<&'static str> {
        let mut descs = Vec::new();

        // Missing metadata
        if self.contains(Self::MISSING_ARTIST) {
            descs.push("Missing artist");
        }
        if self.contains(Self::MISSING_ALBUM) {
            descs.push("Missing album");
        }
        if self.contains(Self::MISSING_YEAR) {
            descs.push("Missing year");
        }
        if self.contains(Self::MISSING_TRACK_NUM) {
            descs.push("Missing track number");
        }

        // Suspicious metadata
        if self.contains(Self::TITLE_IS_FILENAME) {
            descs.push("Title looks like filename");
        }
        if self.contains(Self::GENERIC_METADATA) {
            descs.push("Generic placeholder text");
        }

        // Identification status
        if self.contains(Self::NO_MUSICBRAINZ_ID) {
            descs.push("No MusicBrainz ID");
        }
        if self.contains(Self::LOW_CONFIDENCE) {
            descs.push("Low confidence match");
        }
        if self.contains(Self::BETTER_MATCH_AVAILABLE) {
            descs.push("Better match may exist");
        }
        if self.contains(Self::NEVER_CHECKED) {
            descs.push("Never checked");
        }
        if self.contains(Self::FILE_CHANGED) {
            descs.push("File changed");
        }

        // Verification status
        if self.contains(Self::TITLE_MISMATCH) {
            descs.push("Title doesn't match fingerprint");
        }
        if self.contains(Self::ARTIST_MISMATCH) {
            descs.push("Artist doesn't match fingerprint");
        }
        if self.contains(Self::ALBUM_MISMATCH) {
            descs.push("Album differs from fingerprint");
        }
        if self.contains(Self::POSSIBLY_MISLABELED) {
            descs.push("⚠ Possibly mislabeled");
        }
        if self.contains(Self::VERIFIED) {
            descs.push("✓ Verified");
        }
        if self.contains(Self::UNIDENTIFIED) {
            descs.push("Could not identify");
        }
        if self.contains(Self::AMBIGUOUS_MATCH) {
            descs.push("Multiple possible matches");
        }
        if self.contains(Self::MULTI_ALBUM) {
            descs.push("Appears on multiple albums");
        }

        descs
    }

    /// Get a short summary icon for display.
    #[allow(clippy::if_same_then_else)]
    pub fn summary_icon(&self) -> &'static str {
        if self.contains(Self::POSSIBLY_MISLABELED) {
            "⚠" // Mislabeled - needs attention
        } else if self.contains(Self::VERIFIED) && self.intersection(Self::ANY_MISMATCH).is_empty()
        {
            "✓" // Verified and clean
        } else if self.is_empty() {
            "✓" // All good
        } else if self.contains(Self::NEVER_CHECKED) || self.contains(Self::UNIDENTIFIED) {
            "?" // Unknown
        } else if self.intersects(Self::ANY_MISMATCH) {
            "≠" // Mismatch detected
        } else if self
            .intersects(Self::MISSING_ARTIST | Self::MISSING_ALBUM | Self::GENERIC_METADATA)
        {
            "○" // Missing data
        } else {
            "·" // Minor issues
        }
    }

    /// Convert to bits for database storage.
    pub fn to_bits_i64(&self) -> i64 {
        self.bits() as i64
    }

    /// Create from database bits.
    pub fn from_bits_i64(bits: i64) -> Self {
        Self::from_bits_truncate(bits as u32)
    }
}

/// Quality assessment for a track.
#[derive(Debug, Clone, Default)]
pub struct TrackQuality {
    /// Quality flags indicating issues
    pub flags: QualityFlags,
    /// Overall quality score (0-100)
    pub score: u8,
    /// Identification confidence from AcoustID (if checked)
    pub confidence: Option<f32>,
    /// MusicBrainz recording ID (if identified)
    pub musicbrainz_id: Option<String>,
}

impl TrackQuality {
    /// Check if this track needs attention.
    pub fn needs_attention(&self) -> bool {
        self.score < 70
            || self.flags.intersects(
                QualityFlags::MISSING_ARTIST
                    | QualityFlags::MISSING_ALBUM
                    | QualityFlags::GENERIC_METADATA
                    | QualityFlags::NEVER_CHECKED,
            )
    }

    /// Check if this track could be improved.
    pub fn could_improve(&self) -> bool {
        !self.flags.is_empty()
    }

    /// Get a quality tier for display.
    pub fn tier(&self) -> QualityTier {
        match self.score {
            90..=100 => QualityTier::Excellent,
            70..=89 => QualityTier::Good,
            50..=69 => QualityTier::Fair,
            _ => QualityTier::Poor,
        }
    }
}

/// Quality tier for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityTier {
    Excellent,
    Good,
    Fair,
    Poor,
}

impl QualityTier {
    /// Get display color (as theme color name).
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Excellent => "success",
            Self::Good => "text",
            Self::Fair => "warning",
            Self::Poor => "error",
        }
    }

    /// Get emoji for display.
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Excellent => "★",
            Self::Good => "●",
            Self::Fair => "◐",
            Self::Poor => "○",
        }
    }
}

/// Patterns that indicate generic/placeholder metadata.
const GENERIC_PATTERNS: &[&str] = &[
    "unknown artist",
    "unknown album",
    "various artists",
    "track ",
    "audio track",
    "untitled",
    "new recording",
];

/// Assess the quality of a track's metadata.
///
/// # Arguments
///
/// * `title` - Track title
/// * `artist` - Artist name (if any)
/// * `album` - Album name (if any)
/// * `year` - Release year (if any)
/// * `track_num` - Track number (if any)
/// * `filename` - Original filename (without extension)
/// * `musicbrainz_id` - MusicBrainz recording ID (if any)
/// * `confidence` - AcoustID match confidence (if checked)
///
/// # Returns
///
/// A `TrackQuality` with flags and score.
#[allow(clippy::too_many_arguments)]
pub fn assess_quality(
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    year: Option<i64>,
    track_num: Option<i64>,
    filename: &str,
    musicbrainz_id: Option<&str>,
    confidence: Option<f32>,
) -> TrackQuality {
    let mut flags = QualityFlags::empty();
    let mut score: i32 = 100;

    // Check for missing fields
    if artist.is_none() || artist.map(|a| a.trim().is_empty()).unwrap_or(true) {
        flags |= QualityFlags::MISSING_ARTIST;
        score -= 25;
    }

    if album.is_none() || album.map(|a| a.trim().is_empty()).unwrap_or(true) {
        flags |= QualityFlags::MISSING_ALBUM;
        score -= 15;
    }

    if year.is_none() {
        flags |= QualityFlags::MISSING_YEAR;
        score -= 5;
    }

    if track_num.is_none() {
        flags |= QualityFlags::MISSING_TRACK_NUM;
        score -= 5;
    }

    // Check if title looks like filename
    let title_lower = title.to_lowercase();
    let filename_lower = filename.to_lowercase();
    let filename_clean = filename_lower
        .trim_end_matches(".mp3")
        .trim_end_matches(".flac")
        .trim_end_matches(".m4a")
        .trim_end_matches(".ogg")
        .trim_end_matches(".wav");

    if title_lower == filename_clean
        || title_lower.replace(' ', "_") == filename_clean
        || title_lower.replace(' ', "-") == filename_clean
    {
        flags |= QualityFlags::TITLE_IS_FILENAME;
        score -= 20;
    }

    // Check for generic placeholder text
    let has_generic = |text: &str| -> bool {
        let lower = text.to_lowercase();
        GENERIC_PATTERNS.iter().any(|p| lower.contains(p))
    };

    if has_generic(title)
        || artist.map(&has_generic).unwrap_or(false)
        || album.map(has_generic).unwrap_or(false)
    {
        flags |= QualityFlags::GENERIC_METADATA;
        score -= 15;
    }

    // Check MusicBrainz ID
    if musicbrainz_id.is_none() {
        flags |= QualityFlags::NO_MUSICBRAINZ_ID;
        score -= 10;
    }

    // Check confidence
    if let Some(conf) = confidence {
        if conf < 0.7 {
            flags |= QualityFlags::LOW_CONFIDENCE;
            score -= 10;
        } else if conf < 0.9 {
            flags |= QualityFlags::BETTER_MATCH_AVAILABLE;
            score -= 5;
        }
    } else {
        // Never checked
        flags |= QualityFlags::NEVER_CHECKED;
        score -= 10;
    }

    TrackQuality {
        flags,
        score: score.clamp(0, 100) as u8,
        confidence,
        musicbrainz_id: musicbrainz_id.map(String::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excellent_quality() {
        let quality = assess_quality(
            "Bohemian Rhapsody",
            Some("Queen"),
            Some("A Night at the Opera"),
            Some(1975),
            Some(11),
            "queen_bohemian_rhapsody",
            Some("mb-123"),
            Some(0.98),
        );

        assert!(quality.flags.is_empty() || quality.flags == QualityFlags::empty());
        assert!(quality.score >= 90);
        assert_eq!(quality.tier(), QualityTier::Excellent);
    }

    #[test]
    fn test_missing_artist() {
        let quality = assess_quality(
            "Some Song",
            None,
            Some("Album"),
            Some(2020),
            Some(1),
            "some_song",
            Some("mb-456"),
            Some(0.95),
        );

        assert!(quality.flags.contains(QualityFlags::MISSING_ARTIST));
        assert!(quality.needs_attention());
    }

    #[test]
    fn test_title_is_filename() {
        let quality = assess_quality(
            "01 - Track Name",
            Some("Artist"),
            Some("Album"),
            Some(2020),
            Some(1),
            "01 - Track Name",
            None,
            None,
        );

        assert!(quality.flags.contains(QualityFlags::TITLE_IS_FILENAME));
    }

    #[test]
    fn test_generic_metadata() {
        let quality = assess_quality(
            "Track 01",
            Some("Unknown Artist"),
            Some("Album"),
            None,
            None,
            "track01",
            None,
            None,
        );

        assert!(quality.flags.contains(QualityFlags::GENERIC_METADATA));
        assert!(quality.needs_attention());
    }

    #[test]
    fn test_quality_flags_roundtrip() {
        let flags = QualityFlags::MISSING_ARTIST | QualityFlags::LOW_CONFIDENCE;
        let bits = flags.to_bits_i64();
        let restored = QualityFlags::from_bits_i64(bits);
        assert_eq!(flags, restored);
    }
}
