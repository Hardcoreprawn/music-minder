//! File health tracking module.
//!
//! Tracks the health status of audio files to identify corrupt,
//! problematic, or unidentifiable files in your music library.
//!
//! # Overview
//!
//! This module provides:
//! - [`HealthStatus`]: Health states (Ok, Error, NoMatch, LowConfidence)
//! - [`ErrorType`]: Categorized error types
//! - [`FileHealth`]: Complete health record for a file
//! - Database operations for persisting health data
//! - File hashing for change detection
//!
//! # Example
//!
//! ```ignore
//! use music_minder::health::{FileHealth, upsert_health, get_summary};
//!
//! // Record a successful identification
//! let health = FileHealth::ok("/path/to/song.mp3", 0.95, Some("mb-id".into()));
//! upsert_health(&pool, &health).await?;
//!
//! // Get summary counts
//! let summary = get_summary(&pool).await?;
//! println!("Total files: {}, OK: {}, Errors: {}", summary.total, summary.ok, summary.errors);
//! ```

mod db;
mod hash;
mod types;

// Re-export types
pub use types::{ErrorType, FileHealth, HealthStatus};

// Re-export hash function
pub use hash::compute_file_hash;

// Re-export database operations
pub use db::{
    HealthSummary, delete_health, get_by_status, get_errors, get_health, get_summary,
    has_file_changed, upsert_health,
};
