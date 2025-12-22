//! Library scanning and file watching commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tracing::{debug, info, warn};

use crate::db;
use crate::library;
use crate::scanner;

use crate::scanner::is_audio_file;

/// Scan a directory for music files
pub fn cmd_scan(rt: &Runtime, path: &PathBuf) -> anyhow::Result<()> {
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

/// List all tracks in the database
pub fn cmd_list(rt: &Runtime) -> anyhow::Result<()> {
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

/// Watch a directory for file changes
pub fn cmd_watch(
    rt: &Runtime,
    path: &PathBuf,
    verbose: bool,
    db_path: Option<&PathBuf>,
    scan_first: bool,
) -> anyhow::Result<()> {
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
                            let created = batch
                                .iter()
                                .filter(|(_, e)| matches!(e, scanner::WatchEvent::Created(_)))
                                .count();
                            let modified = batch
                                .iter()
                                .filter(|(_, e)| matches!(e, scanner::WatchEvent::Modified(_)))
                                .count();
                            let removed = batch
                                .iter()
                                .filter(|(_, e)| matches!(e, scanner::WatchEvent::Removed(_)))
                                .count();

                            if created > 0 || modified > 0 || removed > 0 {
                                println!(
                                    "Batch: {} created, {} modified, {} removed",
                                    created, modified, removed
                                );
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
