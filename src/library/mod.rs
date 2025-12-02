use crate::{db, metadata, scanner};
use futures::{Stream, StreamExt};
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Processed(PathBuf),
    Error(PathBuf, String),
}

/// Scans a directory and updates the database with found tracks.
/// Returns a stream of ScanEvents.
pub fn scan_library(
    pool: SqlitePool,
    root: PathBuf,
) -> impl Stream<Item = ScanEvent> {
    let paths = scanner::scan(root);

    paths
        .map(move |path| {
            let pool = pool.clone();
            async move {
                match metadata::read(&path) {
                    Ok(meta) => {
                        let artist_id = db::get_or_create_artist(&pool, &meta.artist).await.ok();
                        let album_id = db::get_or_create_album(&pool, &meta.album, artist_id)
                            .await
                            .ok();
                        match db::insert_track(
                            &pool,
                            &meta,
                            path.to_str().unwrap_or(""),
                            artist_id,
                            album_id,
                        )
                        .await
                        {
                            Ok(_) => ScanEvent::Processed(path),
                            Err(e) => ScanEvent::Error(path, e.to_string()),
                        }
                    }
                    Err(e) => ScanEvent::Error(path, e.to_string()),
                }
            }
        })
        .buffer_unordered(10) // Process 10 files in parallel
}
