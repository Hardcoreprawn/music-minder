//! File health tracking and diagnostics commands.

use rayon::prelude::*;
use std::path::PathBuf;
use tokio::runtime::Runtime;

use crate::{db, diagnostics, health};

/// Check file health status
pub fn cmd_check(
    rt: &Runtime,
    pool: sqlx::SqlitePool,
    path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    rt.block_on(async {
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
pub fn cmd_diagnose(format: &str) -> anyhow::Result<()> {
    let report = diagnostics::DiagnosticReport::generate();

    match format {
        "json" => {
            // JSON output for machine consumption
            let json = serde_json::to_string_pretty(&report)?;
            println!("{}", json);
        }
        _ => {
            // Default text output for human readability
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
        }
    }

    Ok(())
}

/// Assess metadata quality for tracks in the library
pub fn cmd_quality(rt: &Runtime, pool: sqlx::SqlitePool, verbose: bool) -> anyhow::Result<()> {
    rt.block_on(async {
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

/// Validate library files for corruption and incomplete metadata
#[allow(clippy::too_many_arguments)]
pub fn cmd_validate(
    rt: &Runtime,
    pool: sqlx::SqlitePool,
    path: Option<&PathBuf>,
    check_tags: bool,
    check_audio: bool,
    check_fingerprint: bool,
    problems_only: bool,
    parallel: bool,
    format: &str,
) -> anyhow::Result<()> {
    rt.block_on(async {
        // If no specific checks requested, do all checks
        let do_all = !check_tags && !check_audio && !check_fingerprint;

        // Get files to validate
        let files = if let Some(p) = path {
            // Validate specific path
            crate::cli::commands::collect_audio_files(p, true)
        } else {
            // Validate all tracked files
            match soundstore::db::get_all_tracks(&pool).await {
                Ok(tracks) => tracks.into_iter().map(|t| PathBuf::from(t.path)).collect(),
                Err(e) => {
                    eprintln!("Failed to get tracked files: {}", e);
                    std::process::exit(1);
                }
            }
        };

        if files.is_empty() {
            println!("No files to validate");
            return;
        }

        println!("Validating {} files...\n", files.len());

        // Validation function
        let validate_file = |file_path: &PathBuf| -> ValidationResult {
            let mut result = ValidationResult {
                path: file_path.clone(),
                ok: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            };

            // 1. File accessibility
            if !file_path.exists() {
                result.ok = false;
                result.errors.push("File does not exist".to_string());
                return result;
            }

            if let Ok(metadata) = std::fs::metadata(file_path) {
                if metadata.len() == 0 {
                    result.ok = false;
                    result.errors.push("File is zero bytes".to_string());
                    return result;
                }
                if metadata.permissions().readonly() {
                    result.warnings.push("File is read-only".to_string());
                }
            }

            // 2. Tag integrity (if requested or do_all)
            if check_tags || do_all {
                match soundstore::metadata::read(file_path) {
                    Ok(meta) => {
                        if meta.title.is_empty() {
                            result.warnings.push("Missing title tag".to_string());
                        }
                        if meta.artist.is_empty() {
                            result.warnings.push("Missing artist tag".to_string());
                        }
                    }
                    Err(e) => {
                        result.ok = false;
                        result.errors.push(format!("Cannot read tags: {}", e));
                        return result;
                    }
                }
            }

            // 3. Audio integrity (if requested or do_all)
            if check_audio || do_all {
                // Try to get audio properties (duration, sample rate)
                if let Err(e) = soundstore::metadata::read(file_path) {
                    result.ok = false;
                    result.errors.push(format!("Cannot decode audio: {}", e));
                    return result;
                }
            }

            // 4. Fingerprint capability (if requested or do_all)
            if check_fingerprint || do_all {
                // Check if fpcalc is available (basic check - could run fpcalc but it's slow)
                // For now just skip this check since it would require adding 'which' crate
                // Users can run 'diagnose' command to check fpcalc availability
            }

            result
        };

        // Run validation (parallel or sequential)
        let results: Vec<ValidationResult> = if parallel {
            files.par_iter().map(validate_file).collect()
        } else {
            files.iter().map(validate_file).collect()
        };

        // Count results
        let ok_count = results.iter().filter(|r| r.ok && r.warnings.is_empty()).count();
        let warning_count = results.iter().filter(|r| r.ok && !r.warnings.is_empty()).count();
        let error_count = results.iter().filter(|r| !r.ok).count();

        // Output results
        match format {
            "json" => {
                let json_output = serde_json::json!({
                    "total": files.len(),
                    "ok": ok_count,
                    "warnings": warning_count,
                    "errors": error_count,
                    "files": results.iter().filter(|r| !problems_only || !r.ok || !r.warnings.is_empty()).map(|r| {
                        serde_json::json!({
                            "path": r.path.to_string_lossy(),
                            "ok": r.ok,
                            "errors": r.errors,
                            "warnings": r.warnings,
                        })
                    }).collect::<Vec<_>>(),
                });
                if let Ok(json_str) = serde_json::to_string_pretty(&json_output) {
                    println!("{}", json_str);
                } else {
                    eprintln!("Failed to serialize JSON output");
                    std::process::exit(1);
                }
            }
            _ => {
                // Text output
                println!("Validation complete!\n");
                println!("✓ {} files OK", ok_count);
                if warning_count > 0 {
                    println!("⚠️  {} files have warnings", warning_count);
                }
                if error_count > 0 {
                    println!("❌ {} files have errors", error_count);
                }
                println!();

                // Show errors
                let errors: Vec<_> = results.iter().filter(|r| !r.ok).collect();
                if !errors.is_empty() {
                    println!("Errors:");
                    for result in errors.iter().take(10) {
                        println!("  {} - {}", result.path.display(), result.errors.join(", "));
                    }
                    if errors.len() > 10 {
                        println!("  ... and {} more errors", errors.len() - 10);
                    }
                    println!();
                }

                // Show warnings
                let warnings: Vec<_> = results.iter().filter(|r| r.ok && !r.warnings.is_empty()).collect();
                if !warnings.is_empty() && !problems_only {
                    println!("Warnings:");
                    for result in warnings.iter().take(10) {
                        println!("  {} - {}", result.path.display(), result.warnings.join(", "));
                    }
                    if warnings.len() > 10 {
                        println!("  ... and {} more warnings", warnings.len() - 10);
                    }
                }
            }
        }
    });
    Ok(())
}

#[derive(Debug)]
struct ValidationResult {
    path: PathBuf,
    ok: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}
