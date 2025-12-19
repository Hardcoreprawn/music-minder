//! File organization command.

use std::path::PathBuf;
use tokio::runtime::Runtime;

use crate::{db, metadata, organizer};

/// Organize music files based on metadata
pub fn cmd_organize(
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
