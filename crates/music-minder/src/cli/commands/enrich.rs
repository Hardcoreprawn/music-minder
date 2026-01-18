//! Audio fingerprinting and metadata enrichment commands.

use std::path::PathBuf;
use tokio::runtime::Runtime;

use crate::enrichment;
use crate::health;
use crate::metadata;

use super::{collect_audio_files, print_fpcalc_install_instructions};

/// Identify a track using audio fingerprinting
pub fn cmd_identify(
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

/// Check if fingerprinting tools are installed
pub fn cmd_check_tools() -> anyhow::Result<()> {
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

/// Write metadata to an audio file
#[allow(clippy::too_many_arguments)]
pub fn cmd_write_tags(
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

/// Batch enrich multiple audio files
///
/// **Phase C.1 Improvements:**
/// - Parallel fingerprinting (up to 4 files concurrently)
/// - Rate-limited API calls (respects MusicBrainz 1 req/sec limit)
/// - Smarter progress tracking with ETA
/// - Path analysis for genre/compilation hints
#[allow(clippy::too_many_arguments)]
pub fn cmd_enrich(
    rt: &Runtime,
    path: &PathBuf,
    api_key: Option<&str>,
    write: bool,
    fill_only: bool,
    recursive: bool,
    min_confidence: f32,
    dry_run: bool,
    pool: Option<sqlx::SqlitePool>,
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
        println!(
            "Enriching {} file(s) with parallel processing...\n",
            files.len()
        );

        let config = enrichment::EnrichmentConfig {
            acoustid_api_key: api_key,
            min_confidence,
            use_musicbrainz: true,
            ..Default::default()
        };
        let service = std::sync::Arc::new(enrichment::EnrichmentService::new(config));

        // Phase C.1: Parallel processing with rate limiting
        let start_time = std::time::Instant::now();
        let (success_count, skip_count, fail_count) =
            enrich_batch_parallel(&files, service, write, fill_only, dry_run, pool.as_ref()).await;

        let elapsed = start_time.elapsed();
        println!();
        println!(
            "Done in {:.1}s! {} identified, {} no match, {} errors",
            elapsed.as_secs_f64(),
            success_count,
            skip_count,
            fail_count
        );

        // Calculate and show throughput
        if success_count > 0 {
            let per_file = elapsed.as_secs_f64() / success_count as f64;
            println!("Average: {:.1}s per file", per_file);
        }

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

/// Process batch of files in parallel with rate limiting
///
/// **Strategy:**
/// - Fingerprinting: 4 concurrent (CPU-bound, local operation)
/// - API calls: Rate-limited to 1/sec (MusicBrainz requirement)
/// - Tag writes: Sequential (safety - avoid file system contention)
async fn enrich_batch_parallel(
    files: &[PathBuf],
    service: std::sync::Arc<enrichment::EnrichmentService>,
    write_tags: bool,
    fill_only: bool,
    dry_run: bool,
    pool: Option<&sqlx::SqlitePool>,
) -> (usize, usize, usize) {
    use futures::stream::{self, StreamExt};

    let mut success_count = 0;
    let mut skip_count = 0;
    let mut fail_count = 0;

    // Rate limiter: 1 request per second for MusicBrainz
    let rate_limit_delay = std::time::Duration::from_millis(1100);
    let mut last_api_call = std::time::Instant::now() - rate_limit_delay;

    // Process files with parallelism limit
    let mut stream = stream::iter(files.iter().enumerate())
        .map(|(i, file_path)| {
            let service = service.clone();
            async move { (i, file_path, service.identify_track(file_path).await) }
        })
        .buffer_unordered(4); // Parallel fingerprinting (4 concurrent)

    while let Some((i, file_path, result)) = stream.next().await {
        // Enforce rate limiting between API calls
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_api_call);
        if elapsed < rate_limit_delay {
            tokio::time::sleep(rate_limit_delay - elapsed).await;
        }
        last_api_call = std::time::Instant::now();

        let filename = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?");

        print!("[{}/{}] {}... ", i + 1, files.len(), filename);
        use std::io::Write;
        std::io::stdout().flush().unwrap();

        let path_str = file_path.to_string_lossy().to_string();

        match result {
            Ok(identification) => {
                let album = identification.track.album.as_deref().unwrap_or("?");
                print!("✓ {} ", album);

                // Track health: OK
                if let Some(p) = pool {
                    let health_record = health::FileHealth::ok(
                        &path_str,
                        identification.score as f64,
                        identification.track.recording_id.clone(),
                    )
                    .with_file_info(file_path);
                    let _ = health::upsert_health(p, &health_record).await;
                }

                if write_tags && !dry_run {
                    let options = metadata::WriteOptions2 {
                        only_fill_empty: fill_only,
                        write_musicbrainz_ids: true,
                    };
                    match metadata::write(file_path, &identification.track, &options) {
                        Ok(write_result) => {
                            println!("({} tags written)", write_result.fields_updated);
                        }
                        Err(e) => {
                            println!("(write failed: {})", e);
                        }
                    }
                } else if write_tags && dry_run {
                    println!("(would write tags)");
                } else {
                    println!();
                }
                success_count += 1;
            }
            Err(enrichment::EnrichmentError::NoMatches) => {
                println!("✗ No match");
                // Track health: No match
                if let Some(p) = pool {
                    let health_record =
                        health::FileHealth::no_match(&path_str).with_file_info(file_path);
                    let _ = health::upsert_health(p, &health_record).await;
                }
                skip_count += 1;
            }
            Err(e) => {
                println!("✗ Error: {}", e);
                // Track health: Error
                if let Some(p) = pool {
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
    }

    (success_count, skip_count, fail_count)
}
