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
    pub fn quality_flags(&self) -> crate::health::QualityFlags {
        self.quality_flags
            .map(crate::health::QualityFlags::from_bits_i64)
            .unwrap_or_default()
    }
}
