//! # Soundstore
//!
//! Database schema and repository layer for Music Minder.
//!
//! This crate provides:
//! - Database initialization and migrations
//! - Entity models (Track, Artist, Album, TrackHealth)
//! - Repository operations with SQLx
//! - Database health and verification
//!
//! # Architecture
//!
//! The database layer follows a simple pattern:
//! - `model/` - Entity definitions (Track, Artist, Album)
//! - `db/` - Direct SQLx query execution
//! - `migrations/` - SQL schema definitions
//!
//! # Usage
//!
//! ```rust,no_run
//! use soundstore::db;
//! use sqlx::sqlite::SqlitePool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = db::init_db("sqlite::memory:").await?;
//! let tracks = db::get_all_tracks(&pool).await?;
//! # Ok(())
//! # }
//! ```

pub mod db;
pub mod model;
pub mod quality;

// Re-export commonly used types
pub use db::TrackMetadata;
pub use model::{Album, Artist, Track};
pub use quality::{QualityFlags, QualityTier, TrackQuality};
