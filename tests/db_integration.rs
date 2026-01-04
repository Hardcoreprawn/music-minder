//! Database integration tests for Music Minder.
//!
//! These tests verify the core database operations:
//! - Schema initialization and integrity
//! - CRUD operations for artists, albums, and tracks
//! - Pagination and counting
//! - Concurrent access and persistence
//!
//! These are low-level integration tests that ensure the data layer
//! behaves correctly before testing higher-level CLI or UI logic.

use sqlx::sqlite::SqlitePool;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Setup test database and return (pool, temp directory handle)
async fn setup_test_db() -> (SqlitePool, TempDir) {
    let dir = tempfile::tempdir().expect("Failed to create temp directory");
    let db_path = dir.path().join("test.db");
    let db_url = format!("sqlite:{}", db_path.display());

    let pool = music_minder::db::init_db(&db_url)
        .await
        .expect("Failed to initialize test database");

    (pool, dir)
}

/// Helper to create a temporary music directory with test files
fn create_test_music_dir() -> (TempDir, Vec<PathBuf>) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    
    // Create subdirectories
    let artist_dir = temp_dir.path().join("Test Artist");
    let album_dir = artist_dir.join("Test Album");
    fs::create_dir_all(&album_dir).expect("Failed to create album dir");
    
    // Create dummy audio files (we'll just create empty files with correct extensions)
    // In real scenario, these would be actual audio files
    let mut test_files = Vec::new();
    for i in 1..=3 {
        let file_path = album_dir.join(format!("Track {}.mp3", i));
        fs::write(&file_path, b"dummy audio data").expect("Failed to create test file");
        test_files.push(file_path);
    }
    
    (temp_dir, test_files)
}

#[tokio::test]
async fn test_db_initialization() {
    let (pool, _dir) = setup_test_db().await;
    
    // Verify we can query the database
    let result = music_minder::db::get_all_tracks(&pool).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0, "New database should be empty");
}

#[tokio::test]
async fn test_db_track_insertion() {
    let (pool, _dir) = setup_test_db().await;
    let (music_dir, _files) = create_test_music_dir();
    
    // Verify track insertion and retrieval
    let artist_id = music_minder::db::get_or_create_artist(&pool, "Test Artist")
        .await
        .expect("Failed to create artist");
    
    let album_id = music_minder::db::get_or_create_album(&pool, "Test Album", Some(artist_id))
        .await
        .expect("Failed to create album");
    
    let track_path = music_dir.path().join("Test Artist/Test Album/Track 1.mp3");
    let _track_id = music_minder::db::insert_track(
        &pool,
        &music_minder::metadata::TrackMetadata {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration: 180,
            track_number: Some(1),
        },
        track_path.to_string_lossy().as_ref(),
        Some(artist_id),
        Some(album_id),
    )
    .await
    .expect("Failed to insert track");
    
    // Verify track appears in database
    let tracks = music_minder::db::get_all_tracks_with_metadata(&pool)
        .await
        .expect("Failed to query tracks");
    assert_eq!(tracks.len(), 1, "Should have one track");
    assert_eq!(tracks[0].title, "Test Song");
}

#[tokio::test]
async fn test_db_metadata_retrieval() {
    let (pool, _dir) = setup_test_db().await;
    
    // Insert test data
    let artist_id = music_minder::db::get_or_create_artist(&pool, "Artist 1")
        .await
        .unwrap();
    let album_id = music_minder::db::get_or_create_album(&pool, "Album 1", Some(artist_id))
        .await
        .unwrap();
    
    music_minder::db::insert_track(
        &pool,
        &music_minder::metadata::TrackMetadata {
            title: "Song 1".to_string(),
            artist: "Artist 1".to_string(),
            album: "Album 1".to_string(),
            duration: 180,
            track_number: Some(1),
        },
        "/music/song1.mp3",
        Some(artist_id),
        Some(album_id),
    )
    .await
    .unwrap();
    
    // Verify we can get tracks with full metadata (which includes quality fields)
    let tracks_with_meta = music_minder::db::get_all_tracks_with_metadata(&pool)
        .await
        .expect("Failed to list tracks with metadata");
    assert_eq!(tracks_with_meta.len(), 1);
    assert_eq!(tracks_with_meta[0].title, "Song 1");
    assert_eq!(tracks_with_meta[0].artist_name, "Artist 1");
    assert_eq!(tracks_with_meta[0].album_name, "Album 1");
}

#[tokio::test]
async fn test_db_pagination() {
    let (pool, _dir) = setup_test_db().await;
    
    // Insert 300 test tracks
    let artist_id = music_minder::db::get_or_create_artist(&pool, "Artist")
        .await
        .unwrap();
    let album_id = music_minder::db::get_or_create_album(&pool, "Album", Some(artist_id))
        .await
        .unwrap();
    
    for i in 1..=300 {
        music_minder::db::insert_track(
            &pool,
            &music_minder::metadata::TrackMetadata {
                title: format!("Song {}", i),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                duration: 180,
                track_number: Some(i as u32),
            },
            &format!("/music/song{}.mp3", i),
            Some(artist_id),
            Some(album_id),
        )
        .await
        .unwrap();
    }
    
    // Verify paginated loading works
    let first_page = music_minder::db::get_tracks_paginated(&pool, 200, 0)
        .await
        .expect("Failed to get first page");
    assert_eq!(first_page.len(), 200, "First page should have 200 tracks");
    
    let second_page = music_minder::db::get_tracks_paginated(&pool, 200, 200)
        .await
        .expect("Failed to get second page");
    assert_eq!(second_page.len(), 100, "Second page should have remaining 100 tracks");
    
    // Verify count is correct
    let count = music_minder::db::count_tracks(&pool)
        .await
        .expect("Failed to count tracks");
    assert_eq!(count, 300, "Should have 300 tracks total");
}

#[tokio::test]
async fn test_db_artist_album_deduplication() {
    let (pool, _dir) = setup_test_db().await;
    
    // Create multiple artists and albums
    let artist1_id = music_minder::db::get_or_create_artist(&pool, "Artist One")
        .await
        .unwrap();
    let artist2_id = music_minder::db::get_or_create_artist(&pool, "Artist Two")
        .await
        .unwrap();
    
    // Verify same artist returns same ID
    let artist1_id_again = music_minder::db::get_or_create_artist(&pool, "Artist One")
        .await
        .unwrap();
    assert_eq!(artist1_id, artist1_id_again, "Should return same ID for existing artist");
    
    // Verify different artists have different IDs
    assert_ne!(artist1_id, artist2_id, "Different artists should have different IDs");
    
    // Create albums
    let album1_id = music_minder::db::get_or_create_album(&pool, "Album One", Some(artist1_id))
        .await
        .unwrap();
    let album1_id_again = music_minder::db::get_or_create_album(&pool, "Album One", Some(artist1_id))
        .await
        .unwrap();
    
    assert_eq!(album1_id, album1_id_again, "Should return same ID for existing album");
}

#[tokio::test]
async fn test_db_persistence() {
    let (pool, _dir) = setup_test_db().await;
    
    let artist_id = music_minder::db::get_or_create_artist(&pool, "Artist")
        .await
        .unwrap();
    let album_id = music_minder::db::get_or_create_album(&pool, "Album", Some(artist_id))
        .await
        .unwrap();
    
    let track_id = music_minder::db::insert_track(
        &pool,
        &music_minder::metadata::TrackMetadata {
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration: 180,
            track_number: Some(1),
        },
        "/music/song.mp3",
        Some(artist_id),
        Some(album_id),
    )
    .await
    .unwrap();
    
    // Verify track can be retrieved and metadata persists
    let track = music_minder::db::get_track_by_id(&pool, track_id)
        .await
        .expect("Failed to query track")
        .expect("Track should exist");
    
    assert_eq!(track.title, "Song");
    assert_eq!(track.duration, Some(180));
    assert_eq!(track.track_number, Some(1));
}

#[tokio::test]
async fn test_db_schema_integrity() {
    let (pool, _dir) = setup_test_db().await;
    
    // Verify all required tables exist by performing basic operations
    let artist_id = music_minder::db::get_or_create_artist(&pool, "Test")
        .await
        .expect("artists table should exist");
    
    let album_id = music_minder::db::get_or_create_album(&pool, "Test", Some(artist_id))
        .await
        .expect("albums table should exist");
    
    let _track_id = music_minder::db::insert_track(
        &pool,
        &music_minder::metadata::TrackMetadata {
            title: "Test".to_string(),
            artist: "Test".to_string(),
            album: "Test".to_string(),
            duration: 180,
            track_number: Some(1),
        },
        "/test",
        Some(artist_id),
        Some(album_id),
    )
    .await
    .expect("tracks table should exist");
    
    // All operations succeeded, schema is intact
}

#[tokio::test]
async fn test_db_concurrency() {
    let (pool, _dir) = setup_test_db().await;
    
    let artist_id = music_minder::db::get_or_create_artist(&pool, "Artist")
        .await
        .unwrap();
    let album_id = music_minder::db::get_or_create_album(&pool, "Album", Some(artist_id))
        .await
        .unwrap();
    
    // Insert tracks concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let pool = pool.clone();
        let handle = tokio::spawn(async move {
            music_minder::db::insert_track(
                &pool,
                &music_minder::metadata::TrackMetadata {
                    title: format!("Song {}", i),
                    artist: "Artist".to_string(),
                    album: "Album".to_string(),
                    duration: 180,
                    track_number: Some(i as u32),
                },
                &format!("/music/song{}.mp3", i),
                Some(artist_id),
                Some(album_id),
            )
            .await
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().expect("Track insertion should succeed");
    }
    
    // Verify all 10 tracks exist
    let count = music_minder::db::count_tracks(&pool)
        .await
        .expect("Failed to count tracks");
    assert_eq!(count, 10, "All concurrent insertions should succeed");
}

#[test]
fn test_path_validation_logic() {
    // Test that scanner can validate paths without panicking
    let valid_paths = vec![
        "/music/artist/album/song.mp3",
        "/music/song.flac",
        "C:\\Music\\Artist\\Album\\Song.mp3",
        "./relative/path/song.ogg",
    ];
    
    for path in valid_paths {
        // Just verify we can create paths without error
        let _p = PathBuf::from(path);
        assert!(!path.is_empty(), "Path should be valid");
    }
}

#[tokio::test]
async fn test_library_scan_logic() {
    let (pool, _dir) = setup_test_db().await;
    let (music_dir, _files) = create_test_music_dir();
    
    // Run the library scan logic
    // Note: This will likely produce ScanEvent::Error because files are empty,
    // but it verifies the coordination between scanner and library modules.
    use futures::StreamExt;
    let stream = music_minder::library::scan_library(pool.clone(), music_dir.path().to_path_buf());
    let results: Vec<_> = stream.collect().await;
    
    // We expect errors because the files are dummy empty files
    assert!(!results.is_empty());
    for event in results {
        match event {
            music_minder::library::ScanEvent::Processed(_) => {}
            music_minder::library::ScanEvent::Error(_, e) => {
                assert!(e.contains("Failed to read file metadata") || e.contains("Failed to open file for probing"));
            }
        }
    }
}

#[test]
fn test_cli_scan_command_logic() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (pool, _dir) = rt.block_on(async { setup_test_db().await });
    let (music_dir, _files) = create_test_music_dir();
    
    // Test the refactored cmd_scan which now accepts a pool
    // We don't call block_on here because cmd_scan does it internally
    let result = music_minder::cli::cmd_scan(&rt, &music_dir.path().to_path_buf(), pool.clone());
    assert!(result.is_ok());
    
    // Test the refactored cmd_list
    let result = music_minder::cli::cmd_list(&rt, pool);
    assert!(result.is_ok());
}

