//! Database module for track, artist, and album persistence.
//!
//! Uses SQLx with SQLite for lightweight, embedded database storage.
//! Provides async operations for:
//! - Track CRUD operations
//! - Artist and album management  
//! - Batch updates for file organization
//!
//! # Example
//!
//! ```ignore
//! use music_minder::db::{init_db, get_all_tracks_with_metadata};
//!
//! let pool = init_db("sqlite:music.db").await?;
//! let tracks = get_all_tracks_with_metadata(&pool).await?;
//! ```

use std::path::PathBuf;

use crate::metadata::TrackMetadata;
use crate::model::Track;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

/// Default database filename.
pub const DEFAULT_DB_NAME: &str = "music_minder.db";

/// Build a SQLite database URL from an optional path.
///
/// If no path is provided, uses [`DEFAULT_DB_NAME`] in the current directory.
///
/// # Arguments
///
/// * `path` - Optional path to the database file
///
/// # Returns
///
/// A SQLite connection URL string (e.g., "sqlite:music_minder.db")
pub fn db_url(path: Option<&std::path::Path>) -> String {
    match path {
        Some(p) => format!("sqlite:{}", p.display()),
        None => format!("sqlite:{}", DEFAULT_DB_NAME),
    }
}

/// Initialize the database connection pool and run migrations.
///
/// Creates the database file if it doesn't exist, establishes a connection
/// pool with up to 5 connections, and runs all pending migrations.
///
/// # Arguments
///
/// * `db_url` - SQLite connection URL (e.g., "sqlite:music.db")
///
/// # Errors
///
/// Returns an error if:
/// - Database creation fails
/// - Connection cannot be established
/// - Migration fails
pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    if !sqlx::Sqlite::database_exists(db_url).await.unwrap_or(false) {
        sqlx::Sqlite::create_database(db_url).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Get or create an artist by name.
///
/// Looks up an artist by exact name match. If not found, creates a new
/// artist record. This is idempotent - calling with the same name always
/// returns the same ID.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `name` - Artist name to look up or create
///
/// # Returns
///
/// The database ID of the (existing or new) artist.
pub async fn get_or_create_artist(pool: &SqlitePool, name: &str) -> sqlx::Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT id FROM artists WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some((id,)) = row {
        Ok(id)
    } else {
        let result = sqlx::query("INSERT INTO artists (name) VALUES (?)")
            .bind(name)
            .execute(pool)
            .await?;
        Ok(result.last_insert_rowid())
    }
}

/// Get or create an album by title and artist.
///
/// Looks up an album by exact title and artist ID match. If not found,
/// creates a new album record.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `title` - Album title
/// * `artist_id` - Optional artist ID (albums can exist without artist)
///
/// # Returns
///
/// The database ID of the (existing or new) album.
pub async fn get_or_create_album(
    pool: &SqlitePool,
    title: &str,
    artist_id: Option<i64>,
) -> sqlx::Result<i64> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM albums WHERE title = ? AND artist_id IS ?")
            .bind(title)
            .bind(artist_id)
            .fetch_optional(pool)
            .await?;

    if let Some((id,)) = row {
        Ok(id)
    } else {
        let result = sqlx::query("INSERT INTO albums (title, artist_id) VALUES (?, ?)")
            .bind(title)
            .bind(artist_id)
            .execute(pool)
            .await?;
        Ok(result.last_insert_rowid())
    }
}

/// Insert or update a track record.
///
/// Uses SQLite's UPSERT to either insert a new track or update an existing
/// one based on the file path. Track metadata is updated from the provided
/// [`TrackMetadata`].
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `meta` - Track metadata (title, duration, track number)
/// * `path` - File path (unique identifier)
/// * `artist_id` - Optional artist ID
/// * `album_id` - Optional album ID
///
/// # Returns
///
/// The database ID of the inserted or updated track.
pub async fn insert_track(
    pool: &SqlitePool,
    meta: &TrackMetadata,
    path: &str,
    artist_id: Option<i64>,
    album_id: Option<i64>,
) -> sqlx::Result<i64> {
    let duration = meta.duration as i64;
    let track_number = meta.track_number.map(|n| n as i64);

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO tracks (title, artist_id, album_id, path, duration, track_number)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            artist_id = excluded.artist_id,
            album_id = excluded.album_id,
            duration = excluded.duration,
            track_number = excluded.track_number
        RETURNING id
        "#,
    )
    .bind(&meta.title)
    .bind(artist_id)
    .bind(album_id)
    .bind(path)
    .bind(duration)
    .bind(track_number)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Get all tracks from the database.
///
/// Returns basic track information without joined artist/album names.
/// For display purposes, prefer [`get_all_tracks_with_metadata`].
pub async fn get_all_tracks(pool: &SqlitePool) -> sqlx::Result<Vec<Track>> {
    sqlx::query_as::<_, Track>(
        "SELECT id, title, artist_id, album_id, path, duration, track_number FROM tracks",
    )
    .fetch_all(pool)
    .await
}

/// Update the file path for a single track.
///
/// Used after file organization to keep the database in sync with
/// the filesystem.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `track_id` - ID of the track to update
/// * `new_path` - New file path
pub async fn update_track_path(
    pool: &SqlitePool,
    track_id: i64,
    new_path: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE tracks SET path = ? WHERE id = ?")
        .bind(new_path)
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Batch update multiple track paths in a single transaction
/// Returns the number of successfully updated tracks
pub async fn batch_update_track_paths(
    pool: &SqlitePool,
    updates: &[(i64, String)],
) -> sqlx::Result<usize> {
    let mut tx = pool.begin().await?;
    let mut success_count = 0;

    for (track_id, new_path) in updates {
        let result = sqlx::query("UPDATE tracks SET path = ? WHERE id = ?")
            .bind(new_path)
            .bind(track_id)
            .execute(&mut *tx)
            .await;

        if result.is_ok() {
            success_count += 1;
        }
    }

    tx.commit().await?;
    Ok(success_count)
}

/// Get a track by its database ID.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `track_id` - ID of the track to retrieve
///
/// # Returns
///
/// The track if found, or None.
pub async fn get_track_by_id(pool: &SqlitePool, track_id: i64) -> sqlx::Result<Option<Track>> {
    sqlx::query_as::<_, Track>(
        "SELECT id, title, artist_id, album_id, path, duration, track_number FROM tracks WHERE id = ?"
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
}

/// Track with joined artist and album names.
///
/// Used for display and file organization where human-readable names
/// are needed rather than foreign key IDs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackWithMetadata {
    /// Database ID
    pub id: i64,
    /// Track title
    pub title: String,
    /// File path
    pub path: String,
    /// Duration in seconds
    pub duration: Option<i64>,
    /// Track number on album
    pub track_number: Option<i64>,
    /// Artist name (or "Unknown Artist")
    pub artist_name: String,
    /// Album name (or "Unknown Album")
    pub album_name: String,
    /// Release year (from album)
    pub year: Option<i64>,
}

impl TrackWithMetadata {
    /// Convert the path string to a PathBuf.
    ///
    /// This is a convenience method to avoid repeated `PathBuf::from(&track.path)`
    /// throughout the codebase.
    pub fn path_buf(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }
}

/// Get all tracks with artist and album names.
///
/// Performs a LEFT JOIN to include tracks even if they have no artist
/// or album. Missing values are replaced with "Unknown Artist" or
/// "Unknown Album".
///
/// This is the primary method for loading the library for display.
pub async fn get_all_tracks_with_metadata(
    pool: &SqlitePool,
) -> sqlx::Result<Vec<TrackWithMetadata>> {
    sqlx::query_as::<_, TrackWithMetadata>(
        r#"
        SELECT 
            t.id, t.title, t.path, t.duration, t.track_number,
            COALESCE(a.name, 'Unknown Artist') as artist_name,
            COALESCE(al.title, 'Unknown Album') as album_name,
            al.year
        FROM tracks t
        LEFT JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        "#,
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_db_creates_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = init_db(&db_url).await.expect("Failed to init db");
        assert!(db_path.exists());

        // Verify we can query the tables
        let tracks = get_all_tracks(&pool).await.expect("Failed to query tracks");
        assert!(tracks.is_empty());
    }

    #[tokio::test]
    async fn test_artist_creation_and_retrieval() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = init_db(&db_url).await.unwrap();

        // Create artist
        let id1 = get_or_create_artist(&pool, "Test Artist").await.unwrap();
        assert!(id1 > 0);

        // Get same artist - should return same ID
        let id2 = get_or_create_artist(&pool, "Test Artist").await.unwrap();
        assert_eq!(id1, id2);

        // Different artist - different ID
        let id3 = get_or_create_artist(&pool, "Another Artist").await.unwrap();
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn test_album_creation_and_retrieval() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = init_db(&db_url).await.unwrap();

        let artist_id = get_or_create_artist(&pool, "Test Artist").await.unwrap();

        // Create album
        let album_id1 = get_or_create_album(&pool, "Test Album", Some(artist_id))
            .await
            .unwrap();
        assert!(album_id1 > 0);

        // Get same album - should return same ID
        let album_id2 = get_or_create_album(&pool, "Test Album", Some(artist_id))
            .await
            .unwrap();
        assert_eq!(album_id1, album_id2);
    }

    #[tokio::test]
    async fn test_track_insertion_and_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = init_db(&db_url).await.unwrap();

        let meta = TrackMetadata {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration: 180,
            track_number: Some(1),
        };

        let artist_id = get_or_create_artist(&pool, &meta.artist).await.unwrap();
        let album_id = get_or_create_album(&pool, &meta.album, Some(artist_id))
            .await
            .unwrap();

        // Insert track
        let track_id = insert_track(
            &pool,
            &meta,
            "/test/path.mp3",
            Some(artist_id),
            Some(album_id),
        )
        .await
        .unwrap();
        assert!(track_id > 0);

        // Verify track exists
        let track = get_track_by_id(&pool, track_id).await.unwrap().unwrap();
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.path, "/test/path.mp3");

        // Update path
        update_track_path(&pool, track_id, "/new/path.mp3")
            .await
            .unwrap();
        let updated = get_track_by_id(&pool, track_id).await.unwrap().unwrap();
        assert_eq!(updated.path, "/new/path.mp3");
    }

    #[tokio::test]
    async fn test_get_all_tracks_with_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = init_db(&db_url).await.unwrap();

        let meta = TrackMetadata {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration: 180,
            track_number: Some(5),
        };

        let artist_id = get_or_create_artist(&pool, &meta.artist).await.unwrap();
        let album_id = get_or_create_album(&pool, &meta.album, Some(artist_id))
            .await
            .unwrap();
        insert_track(
            &pool,
            &meta,
            "/test/path.mp3",
            Some(artist_id),
            Some(album_id),
        )
        .await
        .unwrap();

        let tracks = get_all_tracks_with_metadata(&pool).await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].artist_name, "Test Artist");
        assert_eq!(tracks[0].album_name, "Test Album");
        assert_eq!(tracks[0].track_number, Some(5));
    }

    #[tokio::test]
    async fn test_batch_update_track_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = init_db(&db_url).await.unwrap();

        // Insert multiple tracks
        let meta1 = TrackMetadata {
            title: "Song 1".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration: 100,
            track_number: Some(1),
        };
        let meta2 = TrackMetadata {
            title: "Song 2".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration: 100,
            track_number: Some(2),
        };

        let artist_id = get_or_create_artist(&pool, "Artist").await.unwrap();
        let album_id = get_or_create_album(&pool, "Album", Some(artist_id))
            .await
            .unwrap();

        let id1 = insert_track(
            &pool,
            &meta1,
            "/old/path1.mp3",
            Some(artist_id),
            Some(album_id),
        )
        .await
        .unwrap();
        let id2 = insert_track(
            &pool,
            &meta2,
            "/old/path2.mp3",
            Some(artist_id),
            Some(album_id),
        )
        .await
        .unwrap();

        // Batch update paths
        let updates = vec![
            (id1, "/new/path1.mp3".to_string()),
            (id2, "/new/path2.mp3".to_string()),
        ];
        let updated = batch_update_track_paths(&pool, &updates).await.unwrap();
        assert_eq!(updated, 2);

        // Verify updates
        let track1 = get_track_by_id(&pool, id1).await.unwrap().unwrap();
        let track2 = get_track_by_id(&pool, id2).await.unwrap().unwrap();
        assert_eq!(track1.path, "/new/path1.mp3");
        assert_eq!(track2.path, "/new/path2.mp3");
    }
}
