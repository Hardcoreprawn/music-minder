//! Command-line interface for music-minder.
//!
//! This module provides CLI commands for scanning, organizing, enriching,
//! and checking music files without launching the GUI.

mod commands;

pub use commands::{
    Cli, Commands, cmd_check, cmd_check_tools, cmd_diagnose, cmd_enrich, cmd_identify, cmd_list,
    cmd_organize, cmd_quality, cmd_scan, cmd_watch, cmd_write_tags, run_command,
};
