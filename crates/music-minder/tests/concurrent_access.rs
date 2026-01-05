//! Concurrent database access tests
//!
//! Tests that verify the database layer handles multi-threaded access safely.
//! Ensures no race conditions, deadlocks, or data corruption under concurrent load.

#[cfg(test)]
mod concurrent_access_tests {
    use soundstore::db;
    use soundstore::metadata::TrackMetadata;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;
    use tokio::task::JoinHandle;

    /// Helper to create test database
    async fn create_test_db() -> (sqlx::sqlite::SqlitePool, TempDir) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = db::init_db(&db_url)
            .await
            .expect("Failed to create database");

        (pool, dir)
    }

    #[tokio::test]
    async fn test_concurrent_artist_creation() {
        let (pool, _dir) = create_test_db().await;
        let pool = Arc::new(pool);

        let mut handles: Vec<JoinHandle<_>> = vec![];
        let success_count = Arc::new(AtomicUsize::new(0));

        // Spawn 10 concurrent tasks creating artists
        for i in 0..10 {
            let pool = Arc::clone(&pool);
            let success = Arc::clone(&success_count);

            let handle = tokio::spawn(async move {
                let artist_name = format!("Test Artist {}", i);
                match db::get_or_create_artist(&pool, &artist_name).await {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("Error creating artist: {}", e);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }

        // All tasks should succeed
        let count = success_count.load(Ordering::Relaxed);
        assert_eq!(count, 10, "All concurrent artist creations should succeed");
    }

    #[tokio::test]
    async fn test_concurrent_album_creation() {
        let (pool, _dir) = create_test_db().await;
        let pool = Arc::new(pool);

        // Create artist first
        let artist_id = db::get_or_create_artist(&pool, "Album Test Artist")
            .await
            .expect("Failed to create artist");

        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles: Vec<JoinHandle<_>> = vec![];

        // Spawn 10 concurrent tasks creating albums
        for i in 0..10 {
            let pool = Arc::clone(&pool);
            let success = Arc::clone(&success_count);

            let handle = tokio::spawn(async move {
                let album_name = format!("Test Album {}", i);
                match db::get_or_create_album(&pool, &album_name, Some(artist_id)).await {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("Error creating album: {}", e);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            let _ = handle.await;
        }

        let count = success_count.load(Ordering::Relaxed);
        assert_eq!(count, 10, "All concurrent album creations should succeed");
    }

    #[tokio::test]
    async fn test_concurrent_duplicate_artist_creation() {
        let (pool, _dir) = create_test_db().await;
        let pool = Arc::new(pool);

        let mut handles: Vec<JoinHandle<_>> = vec![];
        let success_count = Arc::new(AtomicUsize::new(0));

        // Spawn multiple tasks trying to create the SAME artist concurrently
        // This tests upsert behavior
        for _ in 0..5 {
            let pool = Arc::clone(&pool);
            let success = Arc::clone(&success_count);

            let handle = tokio::spawn(async move {
                match db::get_or_create_artist(&pool, "Duplicate Artist").await {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let count = success_count.load(Ordering::Relaxed);
        assert_eq!(
            count, 5,
            "All concurrent duplicate creations should succeed (upsert)"
        );
    }

    #[tokio::test]
    async fn test_concurrent_track_insertion() {
        let (pool, _dir) = create_test_db().await;
        let pool = Arc::new(pool);

        // Create test data first
        let artist_id = db::get_or_create_artist(&pool, "Concurrent Test Artist")
            .await
            .expect("Failed to create artist");

        let album_id = db::get_or_create_album(&pool, "Concurrent Test Album", Some(artist_id))
            .await
            .expect("Failed to create album");

        let mut handles: Vec<JoinHandle<_>> = vec![];
        let success_count = Arc::new(AtomicUsize::new(0));
        let task_count = 10;

        // Spawn 10 concurrent tasks inserting tracks
        for i in 0..task_count {
            let pool = Arc::clone(&pool);
            let success = Arc::clone(&success_count);

            let handle = tokio::spawn(async move {
                let track_path = format!("/test/track_{}.mp3", i);
                let meta = TrackMetadata {
                    title: format!("Track {}", i),
                    artist: "Test Artist".to_string(),
                    album: "Test Album".to_string(),
                    duration: 180,
                    track_number: Some((i as u32) + 1),
                };
                match db::insert_track(&pool, &meta, &track_path, Some(artist_id), Some(album_id))
                    .await
                {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("Error inserting track: {}", e);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let count = success_count.load(Ordering::Relaxed);
        assert_eq!(
            count, task_count,
            "All concurrent track insertions should succeed"
        );

        // Verify count tracks
        let track_count: (i64,) = sqlx::query_as("SELECT COUNT(*) as cnt FROM tracks")
            .fetch_one(pool.as_ref())
            .await
            .expect("Failed to count tracks");

        assert_eq!(
            track_count.0 as usize, task_count,
            "All tracks should be in DB"
        );
    }

    #[tokio::test]
    async fn test_concurrent_transaction_isolation() {
        let (pool, _dir) = create_test_db().await;
        let pool = Arc::new(pool);

        // Create initial state
        let artist_id = db::get_or_create_artist(&pool, "Isolation Test Artist")
            .await
            .expect("Failed to create artist");

        let mut handles: Vec<JoinHandle<_>> = vec![];
        let task_count = 20;

        // Spawn multiple concurrent tasks that read and write
        for i in 0..task_count {
            let pool = Arc::clone(&pool);

            let handle = tokio::spawn(async move {
                let album_name = format!("Isolation Album {}", i);

                // Write
                let _album_id = db::get_or_create_album(&pool, &album_name, Some(artist_id))
                    .await
                    .expect("Failed to create album");

                // Immediately read back
                let _tracks = db::get_all_tracks(&pool)
                    .await
                    .expect("Failed to read tracks");

                i
            });

            handles.push(handle);
        }

        // Collect all results
        let mut results = vec![];
        for handle in handles {
            match handle.await {
                Ok(id) => results.push(id),
                Err(e) => panic!("Task failed: {}", e),
            }
        }

        assert_eq!(results.len(), task_count, "All tasks should complete");

        // Verify all albums were created
        let track_count: (i64,) = sqlx::query_as("SELECT COUNT(*) as cnt FROM albums")
            .fetch_one(pool.as_ref())
            .await
            .expect("Failed to count albums");

        assert!(track_count.0 > 0, "Albums should have been created");
    }
}
