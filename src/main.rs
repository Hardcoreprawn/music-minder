//! Music Minder - A music library management application.
//!
//! This application provides tools for scanning, organizing, enriching, and
//! playing music files. It can be run as a GUI application or used via CLI
//! commands.

pub mod cli;
pub mod cover;
pub mod db;
pub mod diagnostics;
pub mod enrichment;
pub mod error;
pub mod health;
pub mod library;
pub mod metadata;
pub mod model;
pub mod organizer;
pub mod player;
pub mod scanner;
#[cfg(test)]
pub mod test_utils;
pub mod ui;

use clap::Parser;
use iced::application;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use ui::MusicMinder;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(EnvFilter::from_default_env().add_directive("music_minder=info".parse().unwrap()))
        .init();

    let args = cli::Cli::parse();

    // Try to run a CLI command
    if cli::run_command(&args)? {
        // A command was executed, exit normally
        return Ok(());
    }

    // No command specified, launch the GUI
    application("Music Minder", MusicMinder::update, MusicMinder::view)
        .subscription(MusicMinder::subscription)
        .run_with(MusicMinder::new)
        .map_err(|e| anyhow::anyhow!("GUI Error: {}", e))
}
