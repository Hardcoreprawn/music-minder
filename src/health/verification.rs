//! Metadata verification through audio fingerprinting.
//!
//! This module compares existing track metadata against fingerprint identification
//! results to detect mislabeled files. It handles:
//!
//! - **Mismatches**: Tags say "Song A" but audio is actually "Song B"
//! - **Alternatives**: Multiple possible matches with different confidence levels
//! - **Compilations**: Same recording appearing on multiple albums
//!
//! # Verification Flow
//!
//! ```text
//! 1. Read existing metadata from file
//! 2. Generate audio fingerprint
//! 3. Query AcoustID for matches
//! 4. Compare top match against existing tags
//! 5. Flag discrepancies, store alternatives
//! ```

/// Result of verifying a track's metadata against its audio fingerprint.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Overall verification status
    pub status: VerificationStatus,
    /// Existing metadata from the file
    pub existing: ExistingMetadata,
    /// Best fingerprint match (if any)
    pub best_match: Option<FingerprintMatch>,
    /// Alternative matches (sorted by confidence descending)
    pub alternatives: Vec<FingerprintMatch>,
    /// Specific issues detected
    pub issues: Vec<VerificationIssue>,
}

/// High-level verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Metadata matches fingerprint identification
    Verified,
    /// Metadata partially matches (e.g., right song, wrong album)
    PartialMatch,
    /// Metadata doesn't match fingerprint - likely mislabeled
    Mismatch,
    /// No fingerprint match found (can't verify)
    NoMatch,
    /// Fingerprinting failed (corrupt file, etc.)
    Error,
    /// Not yet verified
    Pending,
}

impl VerificationStatus {
    /// Get emoji for display
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Verified => "✓",
            Self::PartialMatch => "~",
            Self::Mismatch => "✗",
            Self::NoMatch => "?",
            Self::Error => "!",
            Self::Pending => "…",
        }
    }

    /// Get color name for UI
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Verified => "success",
            Self::PartialMatch => "warning",
            Self::Mismatch => "error",
            Self::NoMatch => "muted",
            Self::Error => "error",
            Self::Pending => "muted",
        }
    }
}

/// Existing metadata read from the file.
#[derive(Debug, Clone, Default)]
pub struct ExistingMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<u32>,
    pub musicbrainz_recording_id: Option<String>,
}

/// A fingerprint match from AcoustID/MusicBrainz.
#[derive(Debug, Clone)]
pub struct FingerprintMatch {
    /// AcoustID match confidence (0.0-1.0)
    pub confidence: f32,
    /// MusicBrainz recording ID
    pub recording_id: String,
    /// Identified title
    pub title: String,
    /// Identified artist(s)
    pub artist: String,
    /// Releases this recording appears on (compilations, albums, singles)
    pub releases: Vec<ReleaseInfo>,
    /// Best matching release (closest to existing album tag, or first)
    pub best_release: Option<ReleaseInfo>,
}

/// Information about a release (album/single/compilation).
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// MusicBrainz release ID
    pub release_id: String,
    /// Album/release title
    pub title: String,
    /// Release year
    pub year: Option<i32>,
    /// Release type
    pub release_type: ReleaseType,
    /// Track number on this release
    pub track_number: Option<u32>,
    /// How well this matches the existing album tag (0.0-1.0)
    pub album_match_score: f32,
}

/// Type of release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReleaseType {
    #[default]
    Album,
    Single,
    EP,
    Compilation,
    Soundtrack,
    Live,
    Remix,
    Other,
}

impl ReleaseType {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "album" => Self::Album,
            "single" => Self::Single,
            "ep" => Self::EP,
            "compilation" => Self::Compilation,
            "soundtrack" => Self::Soundtrack,
            "live" => Self::Live,
            "remix" => Self::Remix,
            _ => Self::Other,
        }
    }
}

/// Specific verification issue detected.
#[derive(Debug, Clone)]
pub enum VerificationIssue {
    /// Title in tags doesn't match fingerprint
    TitleMismatch {
        existing: String,
        identified: String,
        similarity: f32,
    },
    /// Artist in tags doesn't match fingerprint
    ArtistMismatch {
        existing: String,
        identified: String,
        similarity: f32,
    },
    /// Album might be wrong (recording found on different album)
    AlbumMismatch {
        existing: String,
        identified: String,
    },
    /// Better album match available (e.g., original vs compilation)
    BetterAlbumAvailable {
        current: String,
        suggested: String,
        reason: String,
    },
    /// Multiple high-confidence matches - ambiguous
    AmbiguousMatch { count: usize },
    /// Low confidence match
    LowConfidence { confidence: f32 },
    /// MusicBrainz ID in file doesn't match fingerprint result
    RecordingIdMismatch {
        existing: String,
        identified: String,
    },
}

impl VerificationIssue {
    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            Self::TitleMismatch {
                existing,
                identified,
                ..
            } => {
                format!("Title mismatch: '{}' vs '{}'", existing, identified)
            }
            Self::ArtistMismatch {
                existing,
                identified,
                ..
            } => {
                format!("Artist mismatch: '{}' vs '{}'", existing, identified)
            }
            Self::AlbumMismatch {
                existing,
                identified,
            } => {
                format!("Album mismatch: '{}' vs '{}'", existing, identified)
            }
            Self::BetterAlbumAvailable {
                current,
                suggested,
                reason,
            } => {
                format!("Better album: '{}' → '{}' ({})", current, suggested, reason)
            }
            Self::AmbiguousMatch { count } => {
                format!("{} possible matches - review recommended", count)
            }
            Self::LowConfidence { confidence } => {
                format!("Low confidence: {:.0}%", confidence * 100.0)
            }
            Self::RecordingIdMismatch {
                existing,
                identified,
            } => {
                format!("Recording ID mismatch: {} vs {}", existing, identified)
            }
        }
    }

    /// Is this a critical issue that likely means the file is mislabeled?
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::TitleMismatch { similarity, .. } if *similarity < 0.5
        ) || matches!(
            self,
            Self::ArtistMismatch { similarity, .. } if *similarity < 0.5
        )
    }
}

/// Compare two strings for similarity (0.0-1.0).
///
/// Uses normalized Levenshtein distance with some music-specific handling:
/// - Ignores case
/// - Ignores common variations like "The Beatles" vs "Beatles"
/// - Handles featuring artists: "Song (feat. X)" ≈ "Song"
pub fn string_similarity(a: &str, b: &str) -> f32 {
    let a = normalize_for_comparison(a);
    let b = normalize_for_comparison(b);

    if a == b {
        return 1.0;
    }

    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Simple Levenshtein-based similarity
    let distance = levenshtein_distance(&a, &b);
    let max_len = a.len().max(b.len());
    1.0 - (distance as f32 / max_len as f32)
}

/// Normalize a string for comparison.
fn normalize_for_comparison(s: &str) -> String {
    let mut result = s.to_lowercase();

    // Remove common prefixes
    for prefix in &["the ", "a ", "an "] {
        if result.starts_with(prefix) {
            result = result[prefix.len()..].to_string();
        }
    }

    // Remove featuring suffixes
    for pattern in &[" (feat.", " (ft.", " feat.", " ft.", " featuring "] {
        if let Some(pos) = result.find(pattern) {
            result = result[..pos].to_string();
        }
    }

    // Remove extra whitespace and punctuation
    result
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Simple Levenshtein distance.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    #[allow(clippy::needless_range_loop)]
    for i in 0..=m {
        dp[i][0] = i;
    }
    #[allow(clippy::needless_range_loop)]
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Verify a track by comparing metadata against fingerprint results.
///
/// This is the main entry point for verification.
pub fn verify_metadata(
    existing: &ExistingMetadata,
    matches: &[FingerprintMatch],
) -> VerificationResult {
    if matches.is_empty() {
        return VerificationResult {
            status: VerificationStatus::NoMatch,
            existing: existing.clone(),
            best_match: None,
            alternatives: vec![],
            issues: vec![],
        };
    }

    let best_match = &matches[0];
    let alternatives: Vec<_> = matches.iter().skip(1).cloned().collect();
    let mut issues = Vec::new();

    // Check confidence
    if best_match.confidence < 0.5 {
        issues.push(VerificationIssue::LowConfidence {
            confidence: best_match.confidence,
        });
    }

    // Check for ambiguous matches (multiple high-confidence results)
    let high_confidence_count = matches.iter().filter(|m| m.confidence > 0.8).count();
    if high_confidence_count > 1 {
        issues.push(VerificationIssue::AmbiguousMatch {
            count: high_confidence_count,
        });
    }

    // Compare title
    if let Some(ref existing_title) = existing.title {
        let similarity = string_similarity(existing_title, &best_match.title);
        if similarity < 0.8 {
            issues.push(VerificationIssue::TitleMismatch {
                existing: existing_title.clone(),
                identified: best_match.title.clone(),
                similarity,
            });
        }
    }

    // Compare artist
    if let Some(ref existing_artist) = existing.artist {
        let similarity = string_similarity(existing_artist, &best_match.artist);
        if similarity < 0.7 {
            issues.push(VerificationIssue::ArtistMismatch {
                existing: existing_artist.clone(),
                identified: best_match.artist.clone(),
                similarity,
            });
        }
    }

    // Compare MusicBrainz recording ID if present
    if let Some(ref existing_id) = existing.musicbrainz_recording_id
        && existing_id != &best_match.recording_id
    {
        issues.push(VerificationIssue::RecordingIdMismatch {
            existing: existing_id.clone(),
            identified: best_match.recording_id.clone(),
        });
    }

    // Check album matches
    if let (Some(existing_album), Some(best_release)) = (&existing.album, &best_match.best_release)
    {
        let album_similarity = string_similarity(existing_album, &best_release.title);
        if album_similarity < 0.7 {
            issues.push(VerificationIssue::AlbumMismatch {
                existing: existing_album.clone(),
                identified: best_release.title.clone(),
            });

            // Check if there's a better album match in releases
            if let Some(better) = best_match
                .releases
                .iter()
                .find(|r| string_similarity(existing_album, &r.title) > album_similarity + 0.2)
            {
                issues.push(VerificationIssue::BetterAlbumAvailable {
                    current: best_release.title.clone(),
                    suggested: better.title.clone(),
                    reason: "closer match to existing tag".to_string(),
                });
            }
        }

        // Check if original album is available when tagged as compilation
        if best_release.release_type == ReleaseType::Compilation
            && let Some(original) = best_match
                .releases
                .iter()
                .find(|r| r.release_type == ReleaseType::Album)
        {
            issues.push(VerificationIssue::BetterAlbumAvailable {
                current: best_release.title.clone(),
                suggested: original.title.clone(),
                reason: "original album available".to_string(),
            });
        }
    }

    // Determine overall status
    let status = if issues.is_empty() {
        VerificationStatus::Verified
    } else if issues.iter().any(|i| i.is_critical()) {
        VerificationStatus::Mismatch
    } else {
        VerificationStatus::PartialMatch
    };

    VerificationResult {
        status,
        existing: existing.clone(),
        best_match: Some(best_match.clone()),
        alternatives,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_similarity() {
        assert!((string_similarity("Hello", "Hello") - 1.0).abs() < 0.01);
        assert!((string_similarity("hello", "HELLO") - 1.0).abs() < 0.01);
        assert!((string_similarity("The Beatles", "Beatles") - 1.0).abs() < 0.01);
        assert!(string_similarity("Song (feat. Artist)", "Song") > 0.9);
        assert!(string_similarity("completely different", "nothing alike") < 0.3);
    }

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(normalize_for_comparison("The Beatles"), "beatles");
        assert_eq!(normalize_for_comparison("Song (feat. Guest)"), "song");
        assert_eq!(
            normalize_for_comparison("  Multiple   Spaces  "),
            "multiple spaces"
        );
    }

    #[test]
    fn test_verify_empty_matches() {
        let existing = ExistingMetadata::default();
        let result = verify_metadata(&existing, &[]);
        assert_eq!(result.status, VerificationStatus::NoMatch);
    }

    #[test]
    fn test_verify_mismatch() {
        let existing = ExistingMetadata {
            // Use completely different strings to trigger critical mismatch (similarity < 0.5)
            title: Some("ABCDEFGHIJ".to_string()),
            artist: Some("ZYXWVUTSRQ".to_string()),
            ..Default::default()
        };

        let matches = vec![FingerprintMatch {
            confidence: 0.95,
            recording_id: "abc-123".to_string(),
            title: "1234567890".to_string(),
            artist: "0987654321".to_string(),
            releases: vec![],
            best_release: None,
        }];

        let result = verify_metadata(&existing, &matches);
        assert_eq!(
            result.status,
            VerificationStatus::Mismatch,
            "Expected Mismatch status when title/artist have low similarity. Issues: {:?}",
            result.issues
        );
        assert!(!result.issues.is_empty());
    }
}
