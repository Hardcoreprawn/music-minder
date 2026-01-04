//! # Discographer
//!
//! File discovery, metadata reading, and organization services for Music Minder.
//!
//! This crate provides:
//! - File discovery via `walkdir`
//! - Metadata reading via `lofty`
//! - File organization by pattern
//! - Path handling with UTF-8 safety via `camino`
//!
//! # Architecture
//!
//! - `scanner/` - Recursive file discovery
//! - `metadata/` - Lofty-based metadata reading
//! - `organizer/` - Pattern-based file organization
//!
//! # Usage
//!
//! ```rust,no_run
//! use discographer::metadata;
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let track = metadata::read(Path::new("song.mp3"))?;
//! println!("{} - {}", track.artist, track.title);
//! # Ok(())
//! # }
//! ```

pub mod metadata;
pub mod organizer;
pub mod scanner;

// Re-export commonly used types
pub use metadata::TrackMetadata;
