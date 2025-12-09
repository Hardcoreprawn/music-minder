//! Test utilities and fixtures for music-minder tests.
//!
//! This module provides common test helpers, mock factories, and
//! database utilities to reduce boilerplate in tests.
//!
//! # Example
//!
//! ```ignore
//! use music_minder::test_utils::{temp_db, mock_track_metadata};
//!
//! #[tokio::test]
//! async fn test_something() {
//!     let pool = temp_db().await;
//!     let meta = mock_track_metadata();
//!     // ... test logic
//! }
//! ```

use sqlx::sqlite::SqlitePool;
use tempfile::TempDir;

use crate::db::TrackWithMetadata;
use crate::metadata::TrackMetadata;

/// Creates a temporary database for testing.
///
/// The database is created in a temporary directory that is automatically
/// cleaned up when the returned `TempDir` is dropped. Migrations are run
/// automatically.
///
/// # Returns
///
/// A tuple of (connection pool, temp directory handle).
/// Keep the TempDir alive for the duration of your test.
///
/// # Example
///
/// ```ignore
/// let (pool, _dir) = temp_db().await;
/// // Use pool for database operations
/// // Database is deleted when _dir goes out of scope
/// ```
pub async fn temp_db() -> (SqlitePool, TempDir) {
    let dir = tempfile::tempdir().expect("Failed to create temp directory");
    let db_path = dir.path().join("test.db");
    let db_url = format!("sqlite:{}", db_path.display());

    let pool = crate::db::init_db(&db_url)
        .await
        .expect("Failed to initialize test database");

    (pool, dir)
}

/// Creates a mock TrackMetadata with sensible defaults.
///
/// Use the builder pattern with struct update syntax to customize:
///
/// ```ignore
/// let meta = mock_track_metadata();
/// let custom = TrackMetadata {
///     title: "Custom Title".to_string(),
///     ..mock_track_metadata()
/// };
/// ```
pub fn mock_track_metadata() -> TrackMetadata {
    TrackMetadata {
        title: "Test Track".to_string(),
        artist: "Test Artist".to_string(),
        album: "Test Album".to_string(),
        duration: 180,
        track_number: Some(1),
    }
}

/// Creates a mock TrackWithMetadata with sensible defaults.
///
/// The path is set to a non-existent test path. Customize using
/// struct update syntax:
///
/// ```ignore
/// let track = mock_track_with_metadata();
/// let custom = TrackWithMetadata {
///     title: "Custom".to_string(),
///     ..mock_track_with_metadata()
/// };
/// ```
pub fn mock_track_with_metadata() -> TrackWithMetadata {
    TrackWithMetadata {
        id: 1,
        title: "Test Track".to_string(),
        path: "/test/path/song.mp3".to_string(),
        duration: Some(180),
        track_number: Some(1),
        artist_name: "Test Artist".to_string(),
        album_name: "Test Album".to_string(),
        year: Some(2023),
    }
}

/// Creates a mock TrackWithMetadata with the specified ID and path.
///
/// Useful for testing file operations where path matters.
pub fn mock_track_at_path(id: i64, path: &str) -> TrackWithMetadata {
    TrackWithMetadata {
        id,
        title: format!("Track {}", id),
        path: path.to_string(),
        duration: Some(180),
        track_number: Some(id),
        artist_name: "Test Artist".to_string(),
        album_name: "Test Album".to_string(),
        year: Some(2023),
    }
}

/// Inserts a mock track into the database and returns its ID.
///
/// Creates artist and album records as needed.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `path` - File path for the track
///
/// # Returns
///
/// The database ID of the inserted track.
pub async fn insert_mock_track(pool: &SqlitePool, path: &str) -> i64 {
    let meta = mock_track_metadata();
    let artist_id = crate::db::get_or_create_artist(pool, &meta.artist)
        .await
        .expect("Failed to create artist");
    let album_id = crate::db::get_or_create_album(pool, &meta.album, Some(artist_id))
        .await
        .expect("Failed to create album");

    crate::db::insert_track(pool, &meta, path, Some(artist_id), Some(album_id))
        .await
        .expect("Failed to insert track")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_temp_db_creates_working_database() {
        let (pool, _dir) = temp_db().await;

        // Should be able to query
        let tracks = crate::db::get_all_tracks(&pool).await.unwrap();
        assert!(tracks.is_empty());
    }

    #[tokio::test]
    async fn test_insert_mock_track() {
        let (pool, _dir) = temp_db().await;

        let id = insert_mock_track(&pool, "/test/song.mp3").await;
        assert!(id > 0);

        let tracks = crate::db::get_all_tracks_with_metadata(&pool)
            .await
            .unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].path, "/test/song.mp3");
    }

    #[test]
    fn test_mock_track_metadata_defaults() {
        let meta = mock_track_metadata();
        assert_eq!(meta.title, "Test Track");
        assert_eq!(meta.artist, "Test Artist");
        assert_eq!(meta.album, "Test Album");
        assert_eq!(meta.duration, 180);
        assert_eq!(meta.track_number, Some(1));
    }

    #[test]
    fn test_mock_track_with_metadata_defaults() {
        let track = mock_track_with_metadata();
        assert_eq!(track.id, 1);
        assert_eq!(track.title, "Test Track");
        assert!(!track.path.is_empty());
    }

    #[test]
    fn test_mock_track_at_path() {
        let track = mock_track_at_path(42, "/music/song.flac");
        assert_eq!(track.id, 42);
        assert_eq!(track.path, "/music/song.flac");
        assert_eq!(track.title, "Track 42");
    }
}
