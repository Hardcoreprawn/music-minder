//! File organization command.

use std::path::PathBuf;
use tokio::runtime::Runtime;

use crate::{db, metadata, organizer};
use soundstore;

/// Organize music files based on metadata
pub fn cmd_organize(
    rt: &Runtime,
    pool: sqlx::SqlitePool,
    destination: &PathBuf,
    pattern: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    rt.block_on(async {
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
            if let Ok(file_meta) = metadata::read(&source_path) {
                // Convert to soundstore metadata type
                let meta = soundstore::TrackMetadata {
                    title: file_meta.title.clone(),
                    artist: file_meta.artist.clone(),
                    album: file_meta.album.clone(),
                    duration: file_meta.duration,
                    track_number: file_meta.track_number,
                };

                match organizer::organize_track(&source_path, &file_meta, pattern, destination) {
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
