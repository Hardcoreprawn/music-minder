//! Health status types and file health records.
//!
//! This module defines the core types for tracking file health:
//! - [`HealthStatus`]: The health state of a file (Ok, Error, NoMatch, etc.)
//! - [`ErrorType`]: Categories of errors encountered
//! - [`FileHealth`]: Complete health record for a file

use chrono::{DateTime, Utc};
use std::path::Path;

use super::hash::compute_file_hash;

/// Health status of an audio file.
///
/// Represents the result of health checking a file, from successful
/// identification to various error states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// File is healthy - fingerprinted and identified successfully
    Ok,
    /// File has an error (decode failure, corrupt, etc.)
    Error,
    /// File fingerprinted but no match in AcoustID database
    NoMatch,
    /// File matched but with low confidence
    LowConfidence,
    /// Not yet checked
    Unknown,
}

impl HealthStatus {
    /// Convert to string representation for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "ok",
            HealthStatus::Error => "error",
            HealthStatus::NoMatch => "no_match",
            HealthStatus::LowConfidence => "low_confidence",
            HealthStatus::Unknown => "unknown",
        }
    }

    /// Get emoji representation for display.
    pub fn emoji(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "✓",
            HealthStatus::Error => "✗",
            HealthStatus::NoMatch => "?",
            HealthStatus::LowConfidence => "~",
            HealthStatus::Unknown => "-",
        }
    }
}

impl std::str::FromStr for HealthStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ok" => HealthStatus::Ok,
            "error" => HealthStatus::Error,
            "no_match" => HealthStatus::NoMatch,
            "low_confidence" => HealthStatus::LowConfidence,
            _ => HealthStatus::Unknown,
        })
    }
}

/// Type of error encountered during health check.
///
/// Categorizes errors for reporting and filtering purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    /// Audio decoding failed
    DecodeError,
    /// Fingerprinting produced empty result
    EmptyFingerprint,
    /// File I/O error
    IoError,
    /// Operation timed out
    Timeout,
    /// External API error
    ApiError,
    /// Other error with description
    Other(String),
}

impl ErrorType {
    /// Convert to string representation for storage.
    pub fn as_str(&self) -> &str {
        match self {
            ErrorType::DecodeError => "decode_error",
            ErrorType::EmptyFingerprint => "empty_fingerprint",
            ErrorType::IoError => "io_error",
            ErrorType::Timeout => "timeout",
            ErrorType::ApiError => "api_error",
            ErrorType::Other(s) => s,
        }
    }
}

impl std::str::FromStr for ErrorType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "decode_error" => ErrorType::DecodeError,
            "empty_fingerprint" => ErrorType::EmptyFingerprint,
            "io_error" => ErrorType::IoError,
            "timeout" => ErrorType::Timeout,
            "api_error" => ErrorType::ApiError,
            s => ErrorType::Other(s.to_string()),
        })
    }
}

/// A file health record.
///
/// Contains complete health information for an audio file including
/// identification results, errors, and file metadata.
#[derive(Debug, Clone)]
pub struct FileHealth {
    /// Database ID (None if not yet persisted)
    pub id: Option<i64>,
    /// File path
    pub path: String,
    /// Current health status
    pub status: HealthStatus,
    /// Type of error if status is Error
    pub error_type: Option<ErrorType>,
    /// Error message details
    pub error_message: Option<String>,
    /// AcoustID fingerprint
    pub acoustid_fingerprint: Option<String>,
    /// AcoustID match confidence (0.0-1.0)
    pub acoustid_confidence: Option<f64>,
    /// MusicBrainz recording ID
    pub musicbrainz_id: Option<String>,
    /// File size in bytes
    pub file_size: Option<i64>,
    /// Partial file hash for change detection
    pub file_hash: Option<String>,
    /// When this file was last checked
    pub last_checked: DateTime<Utc>,
}

impl FileHealth {
    /// Create a new healthy file record.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    /// * `confidence` - AcoustID match confidence (0.0-1.0)
    /// * `musicbrainz_id` - Optional MusicBrainz recording ID
    pub fn ok(path: impl Into<String>, confidence: f64, musicbrainz_id: Option<String>) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::Ok,
            error_type: None,
            error_message: None,
            acoustid_fingerprint: None,
            acoustid_confidence: Some(confidence),
            musicbrainz_id,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Create a record for a file with an error.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    /// * `error_type` - Category of error
    /// * `message` - Human-readable error message
    pub fn error(
        path: impl Into<String>,
        error_type: ErrorType,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::Error,
            error_type: Some(error_type),
            error_message: Some(message.into()),
            acoustid_fingerprint: None,
            acoustid_confidence: None,
            musicbrainz_id: None,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Create a record for a file with no match.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    pub fn no_match(path: impl Into<String>) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::NoMatch,
            error_type: None,
            error_message: None,
            acoustid_fingerprint: None,
            acoustid_confidence: None,
            musicbrainz_id: None,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Create a record for a file with low confidence match.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    /// * `confidence` - AcoustID match confidence (0.0-1.0)
    pub fn low_confidence(path: impl Into<String>, confidence: f64) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::LowConfidence,
            error_type: None,
            error_message: None,
            acoustid_fingerprint: None,
            acoustid_confidence: Some(confidence),
            musicbrainz_id: None,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Add file metadata (size and hash).
    ///
    /// Computes and attaches file size and partial hash for change detection.
    pub fn with_file_info(mut self, path: &Path) -> Self {
        if let Ok(metadata) = std::fs::metadata(path) {
            self.file_size = Some(metadata.len() as i64);
        }
        if let Ok(hash) = compute_file_hash(path) {
            self.file_hash = Some(hash);
        }
        self
    }

    /// Add fingerprint to the record.
    pub fn with_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.acoustid_fingerprint = Some(fingerprint.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_roundtrip() {
        for status in [
            HealthStatus::Ok,
            HealthStatus::Error,
            HealthStatus::NoMatch,
            HealthStatus::LowConfidence,
            HealthStatus::Unknown,
        ] {
            assert_eq!(status.as_str().parse::<HealthStatus>().unwrap(), status);
        }
    }

    #[test]
    fn test_error_type_roundtrip() {
        for error_type in [
            ErrorType::DecodeError,
            ErrorType::EmptyFingerprint,
            ErrorType::IoError,
            ErrorType::Timeout,
            ErrorType::ApiError,
        ] {
            assert_eq!(
                error_type.as_str().parse::<ErrorType>().unwrap(),
                error_type
            );
        }
    }

    #[test]
    fn test_file_health_ok() {
        let health = FileHealth::ok("/test/file.mp3", 0.95, Some("mb-123".to_string()));
        assert_eq!(health.status, HealthStatus::Ok);
        assert_eq!(health.acoustid_confidence, Some(0.95));
        assert_eq!(health.musicbrainz_id, Some("mb-123".to_string()));
    }

    #[test]
    fn test_file_health_error() {
        let health = FileHealth::error("/test/file.mp3", ErrorType::DecodeError, "corrupt audio");
        assert_eq!(health.status, HealthStatus::Error);
        assert_eq!(health.error_type, Some(ErrorType::DecodeError));
        assert_eq!(health.error_message, Some("corrupt audio".to_string()));
    }
}
