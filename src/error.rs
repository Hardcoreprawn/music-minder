//! Application-wide error types.
//!
//! This module provides a unified error hierarchy for the application.
//! Library modules use specific error types via `thiserror`, while
//! CLI/main uses `anyhow` for convenient error propagation.
//!
//! # Design
//!
//! - [`Error`]: Top-level application error enum
//! - Module-specific errors (e.g., [`EnrichmentError`]) for detailed handling
//! - All errors implement `std::error::Error` for compatibility
//!
//! # Example
//!
//! ```ignore
//! use music_minder::error::{Error, Result};
//!
//! fn process_file(path: &Path) -> Result<()> {
//!     let pool = init_db()?;  // Database errors auto-convert
//!     let meta = read(path)?; // IO errors auto-convert
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;

/// Application-wide result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level application error.
///
/// Aggregates errors from all subsystems for unified handling.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Metadata reading/writing error
    #[error("Metadata error for {path}: {message}")]
    Metadata { path: PathBuf, message: String },

    /// Audio playback error
    #[error("Playback error: {0}")]
    Playback(String),

    /// File organization error
    #[error("Organization error: {0}")]
    Organization(String),

    /// Enrichment/identification error
    #[error("Enrichment error: {0}")]
    Enrichment(#[from] crate::enrichment::EnrichmentError),

    /// File not found
    #[error("File not found: {0}")]
    NotFound(PathBuf),

    /// Invalid file format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    /// Create a metadata error.
    pub fn metadata(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Metadata {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a not found error.
    pub fn not_found(path: impl Into<PathBuf>) -> Self {
        Self::NotFound(path.into())
    }

    /// Create a playback error.
    pub fn playback(message: impl Into<String>) -> Self {
        Self::Playback(message.into())
    }

    /// Create an organization error.
    pub fn organization(message: impl Into<String>) -> Self {
        Self::Organization(message.into())
    }

    /// Create a config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Add context to an error.
    pub fn context(self, ctx: impl Into<String>) -> Self {
        Self::WithContext {
            context: ctx.into(),
            source: Box::new(self),
        }
    }
}

/// Extension trait for adding context to Results.
pub trait ResultExt<T> {
    /// Add context to an error result.
    fn with_context(self, ctx: impl Into<String>) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn with_context(self, ctx: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.context(ctx))
    }
}

impl<T> ResultExt<T> for std::result::Result<T, std::io::Error> {
    fn with_context(self, ctx: impl Into<String>) -> Result<T> {
        self.map_err(|e| Error::Io(e).context(ctx))
    }
}

impl<T> ResultExt<T> for std::result::Result<T, sqlx::Error> {
    fn with_context(self, ctx: impl Into<String>) -> Result<T> {
        self.map_err(|e| Error::Database(e).context(ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::not_found("/path/to/file.mp3");
        assert!(err.to_string().contains("/path/to/file.mp3"));
    }

    #[test]
    fn test_error_with_context() {
        let err = Error::playback("buffer underrun").context("while playing track");
        let msg = err.to_string();
        assert!(msg.contains("while playing track"));
    }

    #[test]
    fn test_metadata_error() {
        let err = Error::metadata("/music/song.mp3", "unsupported format");
        let msg = err.to_string();
        assert!(msg.contains("song.mp3"));
        assert!(msg.contains("unsupported format"));
    }

    #[test]
    fn test_result_ext() {
        let result: Result<()> = Err(Error::playback("test"));
        let with_ctx = result.with_context("additional context");
        assert!(with_ctx.unwrap_err().to_string().contains("additional context"));
    }
}
