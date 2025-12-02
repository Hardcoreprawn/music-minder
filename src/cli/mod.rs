//! Command-line interface for music-minder.
//!
//! This module provides CLI commands for scanning, organizing, enriching,
//! and checking music files without launching the GUI.

mod commands;

pub use commands::{Cli, Commands, run_command};
