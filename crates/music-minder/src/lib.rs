//! Music Minder library - Core functionality for music library management.
//!
//! This library exposes the core modules for scanning, organizing, enriching,
//! and managing music files. It can be used programmatically or via the CLI.

pub mod cli;
pub mod config;
pub mod cover;
pub mod diagnostics;
pub mod enrichment;
pub mod error;
pub mod health;
pub mod library;
#[cfg(test)]
pub mod test_utils;
pub mod ui;

// Re-export database and models from soundstore crate
pub use soundstore::{db, model};
// Also re-export the audio player from symphonium crate
pub use symphonium as player;
// Re-export file management from musicographer crate
pub use musicographer::scanner;
// Re-export metadata and organizer from respective crates
pub use discographer::organizer;
pub use soundstore::metadata;
