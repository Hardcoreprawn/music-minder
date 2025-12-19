//! CLI command definitions and dispatch.
//!
//! This module provides the command-line interface for Music Minder.
//! Each subcommand is implemented in its own submodule for maintainability:
//! - `scan`: Library scanning and file watching
//! - `organize`: File organization by metadata
//! - `enrich`: Audio fingerprinting and metadata enrichment
//! - `health`: File health checking and diagnostics

mod enrich;
mod health;
mod organize;
mod scan;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub use enrich::{cmd_check_tools, cmd_enrich, cmd_identify, cmd_write_tags};
pub use health::{cmd_check, cmd_diagnose, cmd_quality};
pub use organize::cmd_organize;
pub use scan::{cmd_list, cmd_scan, cmd_watch};

/// Music Minder CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Scan a directory for music
    Scan {
        /// Path to the directory to scan
        path: PathBuf,
    },
    /// List all tracks in the database
    List,
    /// Organize music files based on metadata
    Organize {
        /// Destination root directory
        #[arg(short, long)]
        destination: PathBuf,
        /// Pattern for organizing files (default: {Artist}/{Album}/{TrackNum} - {Title}.{ext})
        #[arg(
            short,
            long,
            default_value = "{Artist}/{Album}/{TrackNum} - {Title}.{ext}"
        )]
        pattern: String,
        /// Dry run - show what would be done without actually moving files
        #[arg(long)]
        dry_run: bool,
    },
    /// Identify a track using audio fingerprinting
    Identify {
        /// Path to the audio file
        path: PathBuf,
        /// AcoustID API key (or set ACOUSTID_API_KEY env var)
        #[arg(short, long, env = "ACOUSTID_API_KEY")]
        api_key: Option<String>,
        /// Write identified metadata to the file
        #[arg(long)]
        write: bool,
        /// Only fill empty tags when writing
        #[arg(long)]
        fill_only: bool,
    },
    /// Check if fingerprinting tools are installed
    CheckTools,
    /// Write metadata to an audio file
    WriteTags {
        /// Path to the audio file
        path: PathBuf,
        /// Track title
        #[arg(long)]
        title: Option<String>,
        /// Artist name
        #[arg(long)]
        artist: Option<String>,
        /// Album name
        #[arg(long)]
        album: Option<String>,
        /// Track number
        #[arg(long)]
        track: Option<u32>,
        /// Release year
        #[arg(long)]
        year: Option<i32>,
        /// Only fill empty tags
        #[arg(long)]
        fill_only: bool,
        /// Preview changes without writing
        #[arg(long)]
        preview: bool,
    },
    /// Batch enrich multiple audio files
    Enrich {
        /// Path to file or directory to enrich
        path: PathBuf,
        /// AcoustID API key (or set ACOUSTID_API_KEY env var)
        #[arg(short, long, env = "ACOUSTID_API_KEY")]
        api_key: Option<String>,
        /// Write identified metadata to files
        #[arg(long)]
        write: bool,
        /// Only fill empty tags when writing
        #[arg(long)]
        fill_only: bool,
        /// Recursive directory scan
        #[arg(short, long)]
        recursive: bool,
        /// Minimum confidence threshold (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        min_confidence: f32,
        /// Dry run - show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
        /// Database path for tracking file health (enables health tracking)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Check file health status
    Check {
        /// Path to file or directory to check
        path: Option<PathBuf>,
        /// Database path
        #[arg(long, default_value = "music_minder.db")]
        db: PathBuf,
        /// Show only files with errors
        #[arg(long)]
        errors_only: bool,
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run system diagnostics for audio readiness
    Diagnose {
        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
        /// Run quick check (skip slow measurements)
        #[arg(long)]
        quick: bool,
    },
    /// Assess metadata quality for library tracks
    Quality {
        /// Database path
        #[arg(long, default_value = "music_minder.db")]
        db: PathBuf,
        /// Show detailed output for each track
        #[arg(short, long)]
        verbose: bool,
    },
    /// Watch a directory for file changes (for debugging/testing)
    Watch {
        /// Path to the directory to watch
        path: PathBuf,
        /// Show verbose output including all file events
        #[arg(short, long)]
        verbose: bool,
        /// Database path for incremental scan comparison
        #[arg(long)]
        db: Option<PathBuf>,
        /// Run an initial incremental scan before watching
        #[arg(long)]
        scan_first: bool,
    },
}

/// Run the specified CLI command.
///
/// Returns `Ok(true)` if a command was run, `Ok(false)` if no command was specified
/// (meaning the GUI should launch).
pub fn run_command(cli: &Cli) -> anyhow::Result<bool> {
    let rt = Runtime::new()?;

    match &cli.command {
        Some(Commands::Scan { path }) => {
            cmd_scan(&rt, path)?;
            Ok(true)
        }
        Some(Commands::List) => {
            cmd_list(&rt)?;
            Ok(true)
        }
        Some(Commands::Organize {
            destination,
            pattern,
            dry_run,
        }) => {
            cmd_organize(&rt, destination, pattern, *dry_run)?;
            Ok(true)
        }
        Some(Commands::Identify {
            path,
            api_key,
            write,
            fill_only,
        }) => {
            cmd_identify(&rt, path, api_key.as_deref(), *write, *fill_only)?;
            Ok(true)
        }
        Some(Commands::CheckTools) => {
            cmd_check_tools()?;
            Ok(true)
        }
        Some(Commands::WriteTags {
            path,
            title,
            artist,
            album,
            track,
            year,
            fill_only,
            preview,
        }) => {
            cmd_write_tags(
                path,
                title.as_deref(),
                artist.as_deref(),
                album.as_deref(),
                *track,
                *year,
                *fill_only,
                *preview,
            )?;
            Ok(true)
        }
        Some(Commands::Enrich {
            path,
            api_key,
            write,
            fill_only,
            recursive,
            min_confidence,
            dry_run,
            db,
        }) => {
            cmd_enrich(
                &rt,
                path,
                api_key.as_deref(),
                *write,
                *fill_only,
                *recursive,
                *min_confidence,
                *dry_run,
                db.as_ref(),
            )?;
            Ok(true)
        }
        Some(Commands::Check {
            path,
            db,
            errors_only: _,
            verbose: _,
        }) => {
            cmd_check(&rt, db, path.as_ref())?;
            Ok(true)
        }
        Some(Commands::Diagnose {
            format: _,
            quick: _,
        }) => {
            cmd_diagnose()?;
            Ok(true)
        }
        Some(Commands::Quality { db, verbose }) => {
            cmd_quality(&rt, db, *verbose)?;
            Ok(true)
        }
        Some(Commands::Watch {
            path,
            verbose,
            db,
            scan_first,
        }) => {
            cmd_watch(&rt, path, *verbose, db.as_ref(), *scan_first)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

// ============================================================================
// Shared helper functions
// ============================================================================

/// Print installation instructions for fpcalc
pub(crate) fn print_fpcalc_install_instructions() {
    eprintln!("Error: fpcalc not found.");
    eprintln!("Install Chromaprint:");
    eprintln!("  Windows: winget install AcoustID.Chromaprint");
    eprintln!("  macOS:   brew install chromaprint");
    eprintln!("  Linux:   apt install libchromaprint-tools");
}

/// Collect audio files from a path (file or directory)
pub(crate) fn collect_audio_files(path: &PathBuf, recursive: bool) -> Vec<PathBuf> {
    if path.is_dir() {
        if recursive {
            walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| is_audio_file(e.path()))
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            std::fs::read_dir(path)
                .expect("Failed to read directory")
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .filter(|e| is_audio_file(&e.path()))
                .map(|e| e.path())
                .collect()
        }
    } else {
        vec![path.clone()]
    }
}

/// Check if a path has an audio file extension
pub(crate) fn is_audio_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    matches!(ext.as_deref(), Some("mp3" | "flac" | "ogg" | "m4a" | "wav"))
}
