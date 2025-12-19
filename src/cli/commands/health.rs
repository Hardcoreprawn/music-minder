//! File health tracking and diagnostics commands.

use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

use crate::{db, diagnostics, health};

/// Check file health status
pub fn cmd_check(rt: &Runtime, db_path: &Path, path: Option<&PathBuf>) -> anyhow::Result<()> {
    rt.block_on(async {
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = match db::init_db(&db_url).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to open database: {}", e);
                std::process::exit(1);
            }
        };

        if let Some(file_path) = path {
            // Check specific file
            let path_str = file_path.to_string_lossy().to_string();
            match health::get_health(&pool, &path_str).await {
                Ok(Some(record)) => {
                    println!("File: {}", record.path);
                    println!("Status: {:?}", record.status);
                    println!("Checked: {:?}", record.last_checked);
                    if let Some(conf) = record.acoustid_confidence {
                        println!("Confidence: {:.0}%", conf * 100.0);
                    }
                    if let Some(ref rec_id) = record.musicbrainz_id {
                        println!("MusicBrainz: https://musicbrainz.org/recording/{}", rec_id);
                    }
                    if let Some(ref err) = record.error_message {
                        println!("Error: {}", err);
                    }
                }
                Ok(None) => {
                    println!("No health record found for {:?}", file_path);
                    println!("Run `enrich` with --db to track file health.");
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            // Show summary
            match health::get_summary(&pool).await {
                Ok(summary) => {
                    println!("File Health Summary");
                    println!("===================");
                    println!("Total tracked: {}", summary.total);
                    println!("  ✓ OK:        {}", summary.ok);
                    println!("  ? No match:  {}", summary.no_match);
                    println!("  ✗ Errors:    {}", summary.errors);
                    println!();

                    if summary.errors > 0 {
                        println!("Files with errors:");
                        if let Ok(errors) =
                            health::get_by_status(&pool, health::HealthStatus::Error).await
                        {
                            for record in errors.iter().take(10) {
                                let filename = std::path::Path::new(&record.path)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("?");
                                let err_msg = record.error_message.as_deref().unwrap_or("unknown");
                                println!("  {} - {}", filename, err_msg);
                            }
                            if errors.len() > 10 {
                                println!("  ... and {} more", errors.len() - 10);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    });
    Ok(())
}

/// Run system diagnostics
pub fn cmd_diagnose() -> anyhow::Result<()> {
    let report = diagnostics::DiagnosticReport::generate();

    println!("System Diagnostics Report");
    println!("=========================\n");
    println!(
        "Overall Rating: {} {}\n",
        report.overall_rating.emoji(),
        report.overall_rating.as_str()
    );

    for check in &report.checks {
        println!(
            "  {} {} : {}",
            check.status.emoji(),
            check.name,
            check.value
        );
        if let Some(ref rec) = check.recommendation {
            println!("    → {}", rec);
        }
    }

    println!();
    Ok(())
}

/// Assess metadata quality for tracks in the library
pub fn cmd_quality(rt: &Runtime, db_path: &Path, verbose: bool) -> anyhow::Result<()> {
    rt.block_on(async {
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = match db::init_db(&db_url).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to open database: {}", e);
                std::process::exit(1);
            }
        };

        // Get tracks needing quality check
        let tracks = match db::get_tracks_needing_quality_check(&pool, 1000).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to get tracks: {}", e);
                std::process::exit(1);
            }
        };

        if tracks.is_empty() {
            println!("All tracks have been quality-checked!");

            // Show stats
            if let Ok(stats) = db::get_quality_stats(&pool).await {
                print_quality_stats(&stats);
            }
            return;
        }

        println!("Assessing {} tracks...\n", tracks.len());

        let mut assessed = 0;
        let mut by_tier = [0usize; 4]; // excellent, good, fair, poor

        for track in &tracks {
            let quality = health::assess_track_quality(track);

            // Update database
            if let Err(e) = db::update_track_quality(&pool, track.id, &quality).await {
                eprintln!("Failed to update track {}: {}", track.id, e);
                continue;
            }

            // Count by tier
            match quality.score {
                90..=100 => by_tier[0] += 1,
                70..=89 => by_tier[1] += 1,
                50..=69 => by_tier[2] += 1,
                _ => by_tier[3] += 1,
            }

            assessed += 1;

            if verbose {
                let tier = quality.tier();
                let icon = tier.emoji();
                let flags = quality.flags.descriptions();

                if flags.is_empty() {
                    println!(
                        "  {} {} - {} ({}%)",
                        icon, track.title, track.artist_name, quality.score
                    );
                } else {
                    println!(
                        "  {} {} - {} ({}%): {}",
                        icon,
                        track.title,
                        track.artist_name,
                        quality.score,
                        flags.join(", ")
                    );
                }
            }

            // Progress indicator for large batches
            if !verbose && assessed % 100 == 0 {
                print!("\rAssessed {}/{} tracks...", assessed, tracks.len());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        if !verbose {
            println!();
        }

        println!("\nAssessed {} tracks:", assessed);
        println!("  ★ Excellent (90+): {}", by_tier[0]);
        println!("  ● Good (70-89):    {}", by_tier[1]);
        println!("  ◐ Fair (50-69):    {}", by_tier[2]);
        println!("  ○ Poor (<50):      {}", by_tier[3]);

        // Show overall stats
        if let Ok(stats) = db::get_quality_stats(&pool).await {
            println!();
            print_quality_stats(&stats);
        }
    });
    Ok(())
}

fn print_quality_stats(stats: &db::QualityStats) {
    println!("Library Quality Summary");
    println!("=======================");
    println!("Total tracks:  {}", stats.total);
    println!("  ★ Excellent: {}", stats.excellent);
    println!("  ● Good:      {}", stats.good);
    println!("  ◐ Fair:      {}", stats.fair);
    println!("  ○ Poor:      {}", stats.poor);
    println!("  ? Unchecked: {}", stats.unchecked);

    if stats.total > 0 {
        let checked = stats.total - stats.unchecked;
        if checked > 0 {
            let avg = (stats.excellent * 95 + stats.good * 80 + stats.fair * 60 + stats.poor * 25)
                / checked;
            println!("\nAverage quality score: ~{}%", avg);
        }
    }
}
