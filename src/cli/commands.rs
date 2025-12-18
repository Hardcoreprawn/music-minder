//! CLI command definitions and handlers.
//!
//! Each subcommand is implemented as a function that takes the parsed arguments
//! and returns an `anyhow::Result<()>`.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tracing::{debug, info, warn};

use crate::{db, diagnostics, enrichment, health, library, metadata, organizer, scanner};

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
            errors_only,
            verbose,
        }) => {
            cmd_check(&rt, path.as_ref(), db, *errors_only, *verbose)?;
            Ok(true)
        }
        Some(Commands::Diagnose { format, quick: _ }) => {
            cmd_diagnose(format)?;
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
// Individual command implementations
// ============================================================================

fn cmd_scan(rt: &Runtime, path: &PathBuf) -> anyhow::Result<()> {
    rt.block_on(async {
        let db_url = "sqlite:music_minder.db";
        let pool = db::init_db(db_url).await.expect("Failed to init DB");
        println!("Scanning directory: {:?}", path);

        use futures::StreamExt;
        let stream = library::scan_library(pool, path.clone());
        let mut stream = std::pin::pin!(stream);
        let mut count = 0;

        while let Some(event) = stream.next().await {
            match event {
                library::ScanEvent::Processed(_) => {
                    count += 1;
                    if count % 100 == 0 {
                        print!("\rScanned {} tracks...", count);
                        use std::io::Write;
                        std::io::stdout().flush().unwrap();
                    }
                }
                library::ScanEvent::Error(p, e) => {
                    eprintln!("\nError processing {:?}: {}", p, e);
                }
            }
        }
        println!("\nScan complete. Total scanned: {} tracks.", count);
    });
    Ok(())
}

fn cmd_list(rt: &Runtime) -> anyhow::Result<()> {
    rt.block_on(async {
        let db_url = "sqlite:music_minder.db";
        let pool = db::init_db(db_url).await.expect("Failed to init DB");
        let tracks = db::get_all_tracks(&pool)
            .await
            .expect("Failed to get tracks");
        for track in tracks {
            println!("{} - {}", track.title, track.path);
        }
    });
    Ok(())
}

fn cmd_organize(
    rt: &Runtime,
    destination: &PathBuf,
    pattern: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    rt.block_on(async {
        let db_url = "sqlite:music_minder.db";
        let pool = db::init_db(db_url).await.expect("Failed to init DB");
        let tracks = db::get_all_tracks(&pool)
            .await
            .expect("Failed to get tracks");

        println!("Organizing {} tracks...", tracks.len());
        println!("Pattern: {}", pattern);
        println!("Destination: {:?}", destination);

        if dry_run {
            println!("\n[DRY RUN MODE - No files will be moved]\n");
        }

        let mut success_count = 0;
        let mut error_count = 0;

        for track in tracks {
            let source_path = PathBuf::from(&track.path);

            // Read metadata from file
            if let Ok(meta) = metadata::read(&source_path) {
                match organizer::organize_track(&source_path, &meta, pattern, destination) {
                    Ok(new_path) => {
                        if dry_run {
                            println!("WOULD MOVE: {} -> {:?}", track.path, new_path);
                        } else {
                            println!("MOVED: {} -> {:?}", track.path, new_path);
                            // Update database with new path
                            let _ = db::insert_track(
                                &pool,
                                &meta,
                                new_path.to_str().unwrap_or(""),
                                track.artist_id,
                                track.album_id,
                            )
                            .await;
                        }
                        success_count += 1;
                    }
                    Err(e) => {
                        eprintln!("ERROR organizing {}: {}", track.path, e);
                        error_count += 1;
                    }
                }
            }
        }

        println!(
            "\nCompleted: {} successful, {} errors",
            success_count, error_count
        );
    });
    Ok(())
}

fn cmd_identify(
    rt: &Runtime,
    path: &PathBuf,
    api_key: Option<&str>,
    write: bool,
    fill_only: bool,
) -> anyhow::Result<()> {
    rt.block_on(async {
        // Check for API key
        let api_key = match api_key {
            Some(key) => key.to_string(),
            None => {
                eprintln!("Error: AcoustID API key required.");
                eprintln!("Get one at: https://acoustid.org/new-application");
                eprintln!("Then use: --api-key YOUR_KEY or set ACOUSTID_API_KEY env var");
                std::process::exit(1);
            }
        };

        // Check if fpcalc is available
        if !enrichment::fingerprint::is_fpcalc_available() {
            print_fpcalc_install_instructions();
            std::process::exit(1);
        }

        println!("Identifying: {:?}", path);
        println!();

        let config = enrichment::EnrichmentConfig {
            acoustid_api_key: api_key,
            min_confidence: 0.5,
            use_musicbrainz: true,
            ..Default::default()
        };
        let service = enrichment::EnrichmentService::new(config);

        match service.identify_track(path).await {
            Ok(result) => {
                println!("✓ Match found! (confidence: {:.0}%)", result.score * 100.0);
                println!();
                if let Some(title) = &result.track.title {
                    println!("  Title:  {}", title);
                }
                if let Some(artist) = &result.track.artist {
                    println!("  Artist: {}", artist);
                }
                if let Some(album) = &result.track.album {
                    println!("  Album:  {}", album);
                }
                if let Some(track_num) = result.track.track_number {
                    if let Some(total) = result.track.total_tracks {
                        println!("  Track:  {}/{}", track_num, total);
                    } else {
                        println!("  Track:  {}", track_num);
                    }
                }
                if let Some(year) = result.track.year {
                    println!("  Year:   {}", year);
                }
                if let Some(ref recording_id) = result.track.recording_id {
                    println!();
                    println!(
                        "  MusicBrainz: https://musicbrainz.org/recording/{}",
                        recording_id
                    );
                }

                // Write tags if requested
                if write {
                    println!();
                    let options = metadata::WriteOptions2 {
                        only_fill_empty: fill_only,
                        write_musicbrainz_ids: true,
                    };
                    match metadata::write(path, &result.track, &options) {
                        Ok(write_result) => {
                            println!(
                                "✓ Tags written ({} fields updated)",
                                write_result.fields_updated
                            );
                            if !write_result.fields_skipped.is_empty() {
                                println!("  Skipped: {}", write_result.fields_skipped.join(", "));
                            }
                        }
                        Err(e) => {
                            eprintln!("✗ Failed to write tags: {}", e);
                        }
                    }
                }
            }
            Err(enrichment::EnrichmentError::NoMatches) => {
                println!("✗ No matches found for this track.");
                println!("  The audio may not be in the AcoustID database.");
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    });
    Ok(())
}

fn cmd_check_tools() -> anyhow::Result<()> {
    println!("Checking enrichment tools...\n");

    // Check fpcalc
    if let Some(version) = enrichment::fingerprint::get_fpcalc_version() {
        println!("✓ fpcalc: {}", version);
    } else {
        println!("✗ fpcalc: NOT FOUND");
        print_fpcalc_install_instructions();
    }

    println!();
    println!("API Keys:");
    if std::env::var("ACOUSTID_API_KEY").is_ok() {
        println!("✓ ACOUSTID_API_KEY: set");
    } else {
        println!("✗ ACOUSTID_API_KEY: not set");
        println!("  Get one at: https://acoustid.org/new-application");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_write_tags(
    path: &std::path::Path,
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    track: Option<u32>,
    year: Option<i32>,
    fill_only: bool,
    preview: bool,
) -> anyhow::Result<()> {
    // Build identified track from CLI args
    let identified = enrichment::domain::IdentifiedTrack {
        title: title.map(String::from),
        artist: artist.map(String::from),
        album: album.map(String::from),
        track_number: track,
        year,
        ..Default::default()
    };

    let options = metadata::WriteOptions2 {
        only_fill_empty: fill_only,
        write_musicbrainz_ids: false,
    };

    if preview {
        match metadata::preview_write(path, &identified, &options) {
            Ok(preview_result) => {
                if preview_result.changes.is_empty() {
                    println!("No changes would be made.");
                } else {
                    println!("Preview of changes to {:?}:\n", path);
                    for change in &preview_result.changes {
                        if change.current_value.is_empty() {
                            println!("  {} : (empty) → {}", change.field, change.new_value);
                        } else {
                            println!(
                                "  {} : {} → {}",
                                change.field, change.current_value, change.new_value
                            );
                        }
                    }
                    println!("\nRun without --preview to apply changes.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match metadata::write(path, &identified, &options) {
            Ok(result) => {
                println!("✓ Tags written to {:?}", path);
                println!("  {} fields updated", result.fields_updated);
                if !result.fields_skipped.is_empty() {
                    println!("  Skipped: {}", result.fields_skipped.join(", "));
                }
            }
            Err(e) => {
                eprintln!("Error writing tags: {}", e);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_enrich(
    rt: &Runtime,
    path: &PathBuf,
    api_key: Option<&str>,
    write: bool,
    fill_only: bool,
    recursive: bool,
    min_confidence: f32,
    dry_run: bool,
    db_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let api_key = match api_key {
        Some(key) => key.to_string(),
        None => {
            eprintln!("Error: AcoustID API key required.");
            eprintln!("Get one at: https://acoustid.org/new-application");
            eprintln!("Then use: --api-key YOUR_KEY or set ACOUSTID_API_KEY env var");
            std::process::exit(1);
        }
    };

    // Check fpcalc is available
    if !enrichment::fingerprint::is_fpcalc_available() {
        print_fpcalc_install_instructions();
        std::process::exit(1);
    }

    rt.block_on(async {
        // Initialize database if --db is provided
        let pool = if let Some(db_path) = db_path {
            let db_url = format!("sqlite:{}", db_path.display());
            match db::init_db(&db_url).await {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("Warning: Failed to initialize database: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Collect files to process
        let files = collect_audio_files(path, recursive);

        if files.is_empty() {
            println!("No audio files found.");
            return;
        }

        if dry_run {
            println!("DRY RUN - no changes will be made\n");
        }
        if pool.is_some() {
            println!("Health tracking enabled\n");
        }
        println!("Enriching {} file(s)...\n", files.len());

        let config = enrichment::EnrichmentConfig {
            acoustid_api_key: api_key,
            min_confidence,
            use_musicbrainz: true,
            ..Default::default()
        };
        let service = enrichment::EnrichmentService::new(config);

        let mut success_count = 0;
        let mut skip_count = 0;
        let mut fail_count = 0;

        for (i, file_path) in files.iter().enumerate() {
            let filename = file_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?");

            print!("[{}/{}] {}... ", i + 1, files.len(), filename);
            use std::io::Write;
            std::io::stdout().flush().unwrap();

            let path_str = file_path.to_string_lossy().to_string();

            match service.identify_track(file_path).await {
                Ok(result) => {
                    let album = result.track.album.as_deref().unwrap_or("?");
                    print!("✓ {} ", album);

                    // Track health: OK
                    if let Some(ref p) = pool {
                        let health_record = health::FileHealth::ok(
                            &path_str,
                            result.score as f64,
                            result.track.recording_id.clone(),
                        )
                        .with_file_info(file_path);
                        let _ = health::upsert_health(p, &health_record).await;
                    }

                    if write && !dry_run {
                        let options = metadata::WriteOptions2 {
                            only_fill_empty: fill_only,
                            write_musicbrainz_ids: true,
                        };
                        match metadata::write(file_path, &result.track, &options) {
                            Ok(write_result) => {
                                println!("({} tags written)", write_result.fields_updated);
                            }
                            Err(e) => {
                                println!("(write failed: {})", e);
                            }
                        }
                    } else if write && dry_run {
                        println!("(would write tags)");
                    } else {
                        println!();
                    }
                    success_count += 1;
                }
                Err(enrichment::EnrichmentError::NoMatches) => {
                    println!("✗ No match");
                    // Track health: No match
                    if let Some(ref p) = pool {
                        let health_record =
                            health::FileHealth::no_match(&path_str).with_file_info(file_path);
                        let _ = health::upsert_health(p, &health_record).await;
                    }
                    skip_count += 1;
                }
                Err(e) => {
                    println!("✗ Error: {}", e);
                    // Track health: Error
                    if let Some(ref p) = pool {
                        let error_type = if e.to_string().contains("fingerprint") {
                            health::ErrorType::EmptyFingerprint
                        } else if e.to_string().contains("decode") {
                            health::ErrorType::DecodeError
                        } else {
                            health::ErrorType::Other("enrichment_error".to_string())
                        };
                        let health_record =
                            health::FileHealth::error(&path_str, error_type, e.to_string())
                                .with_file_info(file_path);
                        let _ = health::upsert_health(p, &health_record).await;
                    }
                    fail_count += 1;
                }
            }

            // Small delay between files to be nice to APIs
            if i < files.len() - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        println!();
        println!(
            "Done! {} identified, {} no match, {} errors",
            success_count, skip_count, fail_count
        );

        // Show health summary if tracking
        if let Some(ref p) = pool
            && let Ok(summary) = health::get_summary(p).await
        {
            println!(
                "\nHealth Summary: {} ok, {} errors, {} no match",
                summary.ok, summary.errors, summary.no_match
            );
        }

        if dry_run && write {
            println!("\nRun without --dry-run to write tags.");
        }
    });
    Ok(())
}

fn cmd_check(
    rt: &Runtime,
    path: Option<&PathBuf>,
    db_path: &std::path::Path,
    errors_only: bool,
    verbose: bool,
) -> anyhow::Result<()> {
    rt.block_on(async {
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = match db::init_db(&db_url).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: Failed to open database: {}", e);
                std::process::exit(1);
            }
        };

        // If path provided, filter to that path
        let records = if errors_only {
            health::get_errors(&pool).await.unwrap_or_default()
        } else {
            // Get all records
            let all_statuses = [
                health::HealthStatus::Ok,
                health::HealthStatus::Error,
                health::HealthStatus::NoMatch,
                health::HealthStatus::LowConfidence,
            ];
            let mut records = Vec::new();
            for status in all_statuses {
                records.extend(
                    health::get_by_status(&pool, status)
                        .await
                        .unwrap_or_default(),
                );
            }
            records
        };

        // Filter by path if provided
        let records: Vec<_> = if let Some(filter_path) = path {
            let filter_str = filter_path.to_string_lossy();
            records
                .into_iter()
                .filter(|r| r.path.starts_with(filter_str.as_ref()))
                .collect()
        } else {
            records
        };

        if records.is_empty() {
            println!("No health records found.");
            if path.is_some() {
                println!(
                    "Try running 'enrich --db {}' first to scan files.",
                    db_path.display()
                );
            }
        } else {
            // Print summary
            let summary = health::get_summary(&pool).await.unwrap_or_default();
            println!("File Health Report");
            println!("==================");
            println!("Total: {} files", summary.total);
            println!("  ✓ OK:           {}", summary.ok);
            println!("  ✗ Errors:       {}", summary.errors);
            println!("  ? No match:     {}", summary.no_match);
            println!("  ~ Low conf:     {}", summary.low_confidence);
            println!();

            // List files
            if verbose || errors_only {
                for record in &records {
                    let status_icon = record.status.emoji();
                    println!("{} {}", status_icon, record.path);
                    if verbose {
                        if let Some(ref err) = record.error_message {
                            println!("    Error: {}", err);
                        }
                        if let Some(conf) = record.acoustid_confidence {
                            println!("    Confidence: {:.0}%", conf * 100.0);
                        }
                        if let Some(ref mb_id) = record.musicbrainz_id {
                            println!("    MusicBrainz: {}", mb_id);
                        }
                    }
                }
            }
        }
    });
    Ok(())
}

fn cmd_diagnose(format: &str) -> anyhow::Result<()> {
    println!("Running system diagnostics...\n");

    let report = diagnostics::DiagnosticReport::generate();

    match format {
        "json" => {
            println!("{}", report.to_json());
        }
        _ => {
            report.print();
        }
    }

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Print installation instructions for fpcalc
fn print_fpcalc_install_instructions() {
    eprintln!("Error: fpcalc not found.");
    eprintln!("Install Chromaprint:");
    eprintln!("  Windows: winget install AcoustID.Chromaprint");
    eprintln!("  macOS:   brew install chromaprint");
    eprintln!("  Linux:   apt install libchromaprint-tools");
}

/// Collect audio files from a path (file or directory)
fn collect_audio_files(path: &PathBuf, recursive: bool) -> Vec<PathBuf> {
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
fn is_audio_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    matches!(ext.as_deref(), Some("mp3" | "flac" | "ogg" | "m4a" | "wav"))
}

fn cmd_watch(
    rt: &Runtime,
    path: &PathBuf,
    verbose: bool,
    db_path: Option<&PathBuf>,
    scan_first: bool,
) -> anyhow::Result<()> {
    use std::collections::HashMap;

    rt.block_on(async {
        // Initialize database if provided
        let pool = if let Some(db_path) = db_path {
            let db_url = format!("sqlite:{}", db_path.display());
            match db::init_db(&db_url).await {
                Ok(p) => {
                    info!(target: "scanner::watch", db = %db_path.display(), "Database connected");
                    Some(p)
                }
                Err(e) => {
                    warn!(target: "scanner::watch", error = %e, "Failed to initialize database, proceeding without");
                    None
                }
            }
        } else {
            None
        };

        // Run incremental scan if requested
        if scan_first {
            if let Some(ref pool) = pool {
                println!("Running incremental scan...");
                run_incremental_scan(pool, path, verbose).await;
                println!();
            } else {
                eprintln!("Warning: --scan-first requires --db");
            }
        }

        // Start file watcher
        println!("Watching for changes in: {}", path.display());
        println!("Press Ctrl+C to stop.\n");

        let (mut watcher, rx) = match scanner::FileWatcher::new(vec![]) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create file watcher: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = watcher.watch(path) {
            eprintln!("Failed to watch directory: {}", e);
            std::process::exit(1);
        }

        info!(target: "scanner::watch", path = %path.display(), "File watcher started");

        // Track pending changes for batching
        let mut pending: HashMap<PathBuf, scanner::WatchEvent> = HashMap::new();
        let mut last_event_time = std::time::Instant::now();
        let batch_delay = std::time::Duration::from_millis(100);

        loop {
            // Check for events with timeout
            match rx.recv_timeout(batch_delay) {
                Ok(event) => {
                    last_event_time = std::time::Instant::now();

                    match &event {
                        scanner::WatchEvent::Created(p) => {
                            if verbose {
                                println!("+ CREATED: {}", p.display());
                            }
                            pending.insert(p.clone(), event);
                        }
                        scanner::WatchEvent::Modified(p) => {
                            if verbose {
                                println!("~ MODIFIED: {}", p.display());
                            }
                            pending.insert(p.clone(), event);
                        }
                        scanner::WatchEvent::Removed(p) => {
                            if verbose {
                                println!("- REMOVED: {}", p.display());
                            }
                            pending.insert(p.clone(), event);
                        }
                        scanner::WatchEvent::DirCreated(p) => {
                            if verbose {
                                println!("+ DIR CREATED: {}", p.display());
                            }
                        }
                        scanner::WatchEvent::Error(e) => {
                            eprintln!("! ERROR: {}", e);
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Process batch if we have pending events and enough time has passed
                    if !pending.is_empty() && last_event_time.elapsed() >= batch_delay {
                        let batch: Vec<_> = pending.drain().collect();

                        if let Some(ref pool) = pool {
                            process_watch_batch(pool, &batch, verbose).await;
                        } else {
                            // Just summarize without DB
                            let created = batch.iter().filter(|(_, e)| matches!(e, scanner::WatchEvent::Created(_))).count();
                            let modified = batch.iter().filter(|(_, e)| matches!(e, scanner::WatchEvent::Modified(_))).count();
                            let removed = batch.iter().filter(|(_, e)| matches!(e, scanner::WatchEvent::Removed(_))).count();

                            if created > 0 || modified > 0 || removed > 0 {
                                println!("Batch: {} created, {} modified, {} removed", created, modified, removed);
                            }
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    eprintln!("Watcher disconnected");
                    break;
                }
            }
        }

        info!(target: "scanner::watch", "File watcher stopped");
    });
    Ok(())
}

/// Run an incremental scan comparing filesystem to database
async fn run_incremental_scan(pool: &sqlx::SqlitePool, root: &PathBuf, verbose: bool) {
    use std::collections::HashMap;
    use std::time::SystemTime;

    // Get all known tracks from DB
    let db_tracks = match db::get_all_track_file_info(pool).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to query database: {}", e);
            return;
        }
    };

    // Build map of path -> (id, mtime)
    let mut db_map: HashMap<String, (i64, Option<i64>)> = HashMap::new();
    for track in db_tracks {
        db_map.insert(track.path.clone(), (track.id, track.mtime));
    }

    debug!(target: "scanner::incremental", count = db_map.len(), "Loaded tracks from database");

    // Scan filesystem
    let mut new_files = 0;
    let mut modified_files = 0;
    let mut unchanged_files = 0;
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_audio_file(e.path()))
    {
        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();
        seen_paths.insert(path_str.clone());

        let fs_mtime = path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        if let Some((id, db_mtime)) = db_map.get(&path_str) {
            // File exists in DB - check if modified
            if db_mtime.is_none() || fs_mtime != *db_mtime {
                modified_files += 1;
                if verbose {
                    println!("~ MODIFIED: {}", path.display());
                }
                // Would re-scan metadata here
                if let Some(mtime) = fs_mtime {
                    let _ = db::update_track_mtime(pool, *id, mtime).await;
                }
            } else {
                unchanged_files += 1;
            }
        } else {
            // New file
            new_files += 1;
            if verbose {
                println!("+ NEW: {}", path.display());
            }
            // Would scan and add to DB here
        }
    }

    // Check for removed files
    let mut removed_files = 0;
    for path_str in db_map.keys() {
        if !seen_paths.contains(path_str) {
            removed_files += 1;
            if verbose {
                println!("- REMOVED: {}", path_str);
            }
            let _ = db::delete_track_by_path(pool, path_str).await;
        }
    }

    println!(
        "Scan complete: {} new, {} modified, {} unchanged, {} removed",
        new_files, modified_files, unchanged_files, removed_files
    );
}

/// Process a batch of watch events with database updates
async fn process_watch_batch(
    pool: &sqlx::SqlitePool,
    batch: &[(PathBuf, scanner::WatchEvent)],
    verbose: bool,
) {
    use std::time::SystemTime;

    let mut created = 0;
    let mut modified = 0;
    let mut removed = 0;

    for (path, event) in batch {
        match event {
            scanner::WatchEvent::Created(_) | scanner::WatchEvent::Modified(_) => {
                // Get file mtime
                let mtime = path
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64);

                let path_str = path.to_string_lossy();

                // Check if file exists in DB
                if let Ok(Some(existing)) = db::get_track_by_path(pool, &path_str).await {
                    // Update mtime
                    if let Some(mt) = mtime {
                        let _ = db::update_track_mtime(pool, existing.id, mt).await;
                    }
                    modified += 1;
                } else {
                    // Would add to DB here (full scan logic)
                    created += 1;
                }
            }
            scanner::WatchEvent::Removed(_) => {
                let path_str = path.to_string_lossy();
                if let Ok(true) = db::delete_track_by_path(pool, &path_str).await {
                    removed += 1;
                }
            }
            _ => {}
        }
    }

    if created > 0 || modified > 0 || removed > 0 {
        info!(
            target: "scanner::watch",
            created = created,
            modified = modified,
            removed = removed,
            "Processed batch"
        );
        if !verbose {
            println!(
                "Processed: {} created, {} modified, {} removed",
                created, modified, removed
            );
        }
    }
}
