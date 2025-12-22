//! Music Minder - A music library management application.
//!
//! This application provides tools for scanning, organizing, enriching, and
//! playing music files. It can be run as a GUI application or used via CLI
//! commands.

// Hide console window on Windows when running as GUI
// CLI commands will attach to the parent console or allocate one
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

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
use iced::window;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use ui::MusicMinder;

/// Embedded app icon (32x32 RGBA PNG)
const APP_ICON: &[u8] = include_bytes!("../assets/icon-32.png");

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    // If running CLI commands on Windows, attach to console for output
    #[cfg(target_os = "windows")]
    if args.command.is_some() {
        attach_console();
    }

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(EnvFilter::from_default_env().add_directive("music_minder=info".parse().unwrap()))
        .init();

    // Try to run a CLI command
    if cli::run_command(&args)? {
        // A command was executed, exit normally
        return Ok(());
    }

    // Load window icon from embedded PNG
    let icon = load_icon(APP_ICON);

    // No command specified, launch the GUI
    application("Music Minder", MusicMinder::update, MusicMinder::view)
        .subscription(MusicMinder::subscription)
        .font(ui::icons::ICON_FONT_BYTES)
        .window(window::Settings {
            icon,
            ..Default::default()
        })
        .run_with(MusicMinder::new)
        .map_err(|e| anyhow::anyhow!("GUI Error: {}", e))
}

/// Load a PNG icon from bytes into an iced window icon
fn load_icon(png_bytes: &[u8]) -> Option<window::Icon> {
    // Decode PNG using iced's image feature
    let image = image::load_from_memory(png_bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    window::icon::from_rgba(image.into_raw(), width, height).ok()
}

/// Attach to parent console on Windows for CLI output.
/// This is needed because windows_subsystem = "windows" detaches from console.
#[cfg(target_os = "windows")]
fn attach_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    unsafe {
        // Try to attach to parent console (e.g., PowerShell, cmd)
        // If that fails, we just won't have console output (acceptable for GUI launch)
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
