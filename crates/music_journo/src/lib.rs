//! # Music Journo
//!
//! File management and library scanning utilities for music collections.
//!
//! This crate provides:
//! - Directory scanning for audio files
//! - Real-time file system watching with debouncing
//! - Support for MP3, FLAC, OGG, M4A, WAV formats
//!
//! # Architecture
//!
//! - `scanner/` - Directory traversal and audio file discovery

pub mod scanner;

pub use scanner::{AUDIO_EXTENSIONS, FileWatcher, WatchError, WatchEvent, is_audio_file, scan};
