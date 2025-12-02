use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::migrate::MigrateDatabase;
use crate::model::Track;
use crate::metadata::TrackMetadata;

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    if !sqlx::Sqlite::database_exists(db_url).await.unwrap_or(false) {
        sqlx::Sqlite::create_database(db_url).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}

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

pub async fn get_or_create_album(pool: &SqlitePool, title: &str, artist_id: Option<i64>) -> sqlx::Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT id FROM albums WHERE title = ? AND artist_id IS ?")
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

pub async fn get_all_tracks(pool: &SqlitePool) -> sqlx::Result<Vec<Track>> {
    sqlx::query_as::<_, Track>("SELECT id, title, artist_id, album_id, path, duration, track_number FROM tracks")
        .fetch_all(pool)
        .await
}

pub async fn update_track_path(pool: &SqlitePool, track_id: i64, new_path: &str) -> sqlx::Result<()> {
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

pub async fn get_track_by_id(pool: &SqlitePool, track_id: i64) -> sqlx::Result<Option<Track>> {
    sqlx::query_as::<_, Track>(
        "SELECT id, title, artist_id, album_id, path, duration, track_number FROM tracks WHERE id = ?"
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
}

/// Gets track with artist and album names for organizing
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackWithMetadata {
    pub id: i64,
    pub title: String,
    pub path: String,
    pub duration: Option<i64>,
    pub track_number: Option<i64>,
    pub artist_name: String,
    pub album_name: String,
}

pub async fn get_all_tracks_with_metadata(pool: &SqlitePool) -> sqlx::Result<Vec<TrackWithMetadata>> {
    sqlx::query_as::<_, TrackWithMetadata>(
        r#"
        SELECT 
            t.id, t.title, t.path, t.duration, t.track_number,
            COALESCE(a.name, 'Unknown Artist') as artist_name,
            COALESCE(al.title, 'Unknown Album') as album_name
        FROM tracks t
        LEFT JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        "#
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
        let album_id1 = get_or_create_album(&pool, "Test Album", Some(artist_id)).await.unwrap();
        assert!(album_id1 > 0);

        // Get same album - should return same ID
        let album_id2 = get_or_create_album(&pool, "Test Album", Some(artist_id)).await.unwrap();
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
        let album_id = get_or_create_album(&pool, &meta.album, Some(artist_id)).await.unwrap();
        
        // Insert track
        let track_id = insert_track(&pool, &meta, "/test/path.mp3", Some(artist_id), Some(album_id))
            .await
            .unwrap();
        assert!(track_id > 0);

        // Verify track exists
        let track = get_track_by_id(&pool, track_id).await.unwrap().unwrap();
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.path, "/test/path.mp3");

        // Update path
        update_track_path(&pool, track_id, "/new/path.mp3").await.unwrap();
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
        let album_id = get_or_create_album(&pool, &meta.album, Some(artist_id)).await.unwrap();
        insert_track(&pool, &meta, "/test/path.mp3", Some(artist_id), Some(album_id))
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
        let album_id = get_or_create_album(&pool, "Album", Some(artist_id)).await.unwrap();
        
        let id1 = insert_track(&pool, &meta1, "/old/path1.mp3", Some(artist_id), Some(album_id)).await.unwrap();
        let id2 = insert_track(&pool, &meta2, "/old/path2.mp3", Some(artist_id), Some(album_id)).await.unwrap();

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
