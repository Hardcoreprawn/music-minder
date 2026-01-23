//! Core data models for the music library.
//!
//! Defines the primary entities: [`Track`], [`Artist`], and [`Album`].
//! These are derived from SQLx for database mapping.
//!
//! # Database Schema
//!
//! The models map to the following tables:
//! - `artists` - Artist records with unique names
//! - `albums` - Albums with optional artist reference
//! - `tracks` - Individual audio files with metadata

use sqlx::FromRow;
use std::str::FromStr;

/// An artist in the music library.
#[derive(Debug, Clone, FromRow)]
pub struct Artist {
    /// Database ID (auto-generated)
    pub id: i64,
    /// Artist name (unique)
    pub name: String,
}

/// An album in the music library.
#[derive(Debug, Clone, FromRow)]
pub struct Album {
    /// Database ID (auto-generated)
    pub id: i64,
    /// Album title
    pub title: String,
    /// Optional artist ID (albums can exist without artist)
    pub artist_id: Option<i64>,
    /// Release year (optional)
    pub year: Option<i64>,
}

/// A track (audio file) in the music library.
#[derive(Debug, Clone, FromRow)]
pub struct Track {
    /// Database ID (auto-generated)
    pub id: i64,
    /// Track title (from metadata or filename)
    pub title: String,
    /// Foreign key to artists table
    pub artist_id: Option<i64>,
    /// Foreign key to albums table
    pub album_id: Option<i64>,
    /// Absolute file path (unique identifier)
    pub path: String,
    /// Duration in seconds
    pub duration: Option<i64>,
    /// Track number on album
    pub track_number: Option<i64>,
    /// Quality score (0-100, None if never assessed)
    pub quality_score: Option<i64>,
    /// Quality flags as bitfield (see QualityFlags)
    pub quality_flags: Option<i64>,
    /// When quality was last assessed (ISO 8601)
    pub quality_checked_at: Option<String>,
    /// AcoustID match confidence (0.0-1.0)
    pub acoustid_confidence: Option<f64>,
    /// MusicBrainz recording ID
    pub musicbrainz_recording_id: Option<String>,
    /// Enrichment level (minimal, basic, enhanced, complete)
    pub enrichment_level: Option<String>,
    /// Whether cover art is available
    pub cover_art_available: Option<i64>,
}

/// Enrichment level for a track.
///
/// Tracks the completeness of metadata enrichment:
/// - **Minimal**: File metadata only
/// - **Basic**: + AcoustID fingerprint match (title, artist, album)
/// - **Enhanced**: + MusicBrainz enrichment (genres, release types, IDs)
/// - **Complete**: + Cover art downloaded
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnrichmentLevel {
    Minimal,
    Basic,
    Enhanced,
    Complete,
}

impl EnrichmentLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnrichmentLevel::Minimal => "minimal",
            EnrichmentLevel::Basic => "basic",
            EnrichmentLevel::Enhanced => "enhanced",
            EnrichmentLevel::Complete => "complete",
        }
    }

    /// Get a human-readable description of what's included at this level
    pub fn description(&self) -> &'static str {
        match self {
            EnrichmentLevel::Minimal => "File metadata only",
            EnrichmentLevel::Basic => "AcoustID fingerprint match",
            EnrichmentLevel::Enhanced => "MusicBrainz enrichment (genres, release info)",
            EnrichmentLevel::Complete => "Full enrichment with cover art",
        }
    }
}

impl FromStr for EnrichmentLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "minimal" => Ok(EnrichmentLevel::Minimal),
            "basic" => Ok(EnrichmentLevel::Basic),
            "enhanced" => Ok(EnrichmentLevel::Enhanced),
            "complete" => Ok(EnrichmentLevel::Complete),
            _ => Err(()),
        }
    }
}

impl Track {
    /// Check if this track has been quality-assessed.
    pub fn is_quality_checked(&self) -> bool {
        self.quality_score.is_some()
    }

    /// Check if this track needs attention based on quality score.
    pub fn needs_attention(&self) -> bool {
        match self.quality_score {
            None => true,
            Some(score) => score < 70,
        }
    }

    /// Get quality flags as the typed bitflags.
    pub fn quality_flags(&self) -> crate::quality::QualityFlags {
        self.quality_flags
            .map(crate::quality::QualityFlags::from_bits_i64)
            .unwrap_or_default()
    }

    /// Get enrichment level as typed enum.
    pub fn enrichment_level(&self) -> EnrichmentLevel {
        self.enrichment_level
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(EnrichmentLevel::Minimal)
    }

    /// Check if track has cover art available.
    pub fn has_cover_art(&self) -> bool {
        self.cover_art_available.unwrap_or(0) != 0
    }
}
