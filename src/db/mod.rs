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
        r#"SELECT id, title, artist_id, album_id, path, duration, track_number,
           quality_score, quality_flags, quality_checked_at, 
           acoustid_confidence, musicbrainz_recording_id
           FROM tracks WHERE id = ?"#,
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
    /// Quality score (0-100, None if never assessed)
    pub quality_score: Option<i64>,
    /// Quality flags as bitfield
    pub quality_flags: Option<i64>,
}

/// Lightweight track info for incremental scanning.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackFileInfo {
    /// Database ID
    pub id: i64,
    /// File path
    pub path: String,
    /// Last modified time (Unix timestamp)
    pub mtime: Option<i64>,
}

impl TrackWithMetadata {
    /// Convert the path string to a PathBuf.
    ///
    /// This is a convenience method to avoid repeated `PathBuf::from(&track.path)`
    /// throughout the codebase.
    pub fn path_buf(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    /// Check if this track needs attention based on quality score.
    pub fn needs_attention(&self) -> bool {
        match self.quality_score {
            None => true,
            Some(score) => score < 70,
        }
    }

    /// Get quality flags as the typed bitflags.
    pub fn quality_flags(&self) -> crate::health::QualityFlags {
        self.quality_flags
            .map(crate::health::QualityFlags::from_bits_i64)
            .unwrap_or_default()
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
            al.year,
            t.quality_score, t.quality_flags
        FROM tracks t
        LEFT JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        "#,
    )
    .fetch_all(pool)
    .await
}

// ============================================================================
// Incremental Scanning Support
// ============================================================================

/// Get all track paths and mtimes for incremental scanning.
///
/// Returns lightweight records for efficient comparison with filesystem.
pub async fn get_all_track_file_info(pool: &SqlitePool) -> sqlx::Result<Vec<TrackFileInfo>> {
    sqlx::query_as::<_, TrackFileInfo>("SELECT id, path, mtime FROM tracks")
        .fetch_all(pool)
        .await
}

/// Update the mtime for a track.
pub async fn update_track_mtime(pool: &SqlitePool, track_id: i64, mtime: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE tracks SET mtime = ? WHERE id = ?")
        .bind(mtime)
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert or update a track with mtime.
///
/// Like [`insert_track`] but also stores the file modification time.
pub async fn insert_track_with_mtime(
    pool: &SqlitePool,
    meta: &TrackMetadata,
    path: &str,
    artist_id: Option<i64>,
    album_id: Option<i64>,
    mtime: i64,
) -> sqlx::Result<i64> {
    let duration = meta.duration as i64;
    let track_number = meta.track_number.map(|n| n as i64);

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO tracks (title, artist_id, album_id, path, duration, track_number, mtime)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            artist_id = excluded.artist_id,
            album_id = excluded.album_id,
            duration = excluded.duration,
            track_number = excluded.track_number,
            mtime = excluded.mtime
        RETURNING id
        "#,
    )
    .bind(&meta.title)
    .bind(artist_id)
    .bind(album_id)
    .bind(path)
    .bind(duration)
    .bind(track_number)
    .bind(mtime)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Delete a track by path.
///
/// Used when a file is detected as removed from the filesystem.
pub async fn delete_track_by_path(pool: &SqlitePool, path: &str) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM tracks WHERE path = ?")
        .bind(path)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Get a track by path.
pub async fn get_track_by_path(
    pool: &SqlitePool,
    path: &str,
) -> sqlx::Result<Option<TrackFileInfo>> {
    sqlx::query_as::<_, TrackFileInfo>("SELECT id, path, mtime FROM tracks WHERE path = ?")
        .bind(path)
        .fetch_optional(pool)
        .await
}

// ============================================================================
// Quality Assessment Operations
// ============================================================================

use crate::health::TrackQuality;

/// Update the quality assessment for a track.
///
/// Stores the quality score, flags, and identification results.
pub async fn update_track_quality(
    pool: &SqlitePool,
    track_id: i64,
    quality: &TrackQuality,
) -> sqlx::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"UPDATE tracks SET 
           quality_score = ?,
           quality_flags = ?,
           quality_checked_at = ?,
           acoustid_confidence = ?,
           musicbrainz_recording_id = ?
           WHERE id = ?"#,
    )
    .bind(quality.score as i64)
    .bind(quality.flags.to_bits_i64())
    .bind(&now)
    .bind(quality.confidence.map(|c| c as f64))
    .bind(&quality.musicbrainz_id)
    .bind(track_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get tracks that need quality assessment.
///
/// Returns tracks that have never been checked or have changed since last check.
/// Limited to `batch_size` tracks for gradual background processing.
pub async fn get_tracks_needing_quality_check(
    pool: &SqlitePool,
    batch_size: u32,
) -> sqlx::Result<Vec<TrackWithMetadata>> {
    sqlx::query_as::<_, TrackWithMetadata>(
        r#"
        SELECT 
            t.id, t.title, t.path, t.duration, t.track_number,
            COALESCE(a.name, 'Unknown Artist') as artist_name,
            COALESCE(al.title, 'Unknown Album') as album_name,
            al.year,
            t.quality_score, t.quality_flags
        FROM tracks t
        LEFT JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.quality_score IS NULL
           OR t.quality_checked_at IS NULL
        ORDER BY t.id
        LIMIT ?
        "#,
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await
}

/// Get tracks needing attention (quality score < threshold).
///
/// Returns tracks that would benefit from enrichment.
pub async fn get_tracks_needing_attention(
    pool: &SqlitePool,
    score_threshold: i64,
) -> sqlx::Result<Vec<TrackWithMetadata>> {
    sqlx::query_as::<_, TrackWithMetadata>(
        r#"
        SELECT 
            t.id, t.title, t.path, t.duration, t.track_number,
            COALESCE(a.name, 'Unknown Artist') as artist_name,
            COALESCE(al.title, 'Unknown Album') as album_name,
            al.year,
            t.quality_score, t.quality_flags
        FROM tracks t
        LEFT JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.quality_score IS NOT NULL 
          AND t.quality_score < ?
        ORDER BY t.quality_score ASC
        "#,
    )
    .bind(score_threshold)
    .fetch_all(pool)
    .await
}

/// Get quality statistics for the library.
#[derive(Debug, Clone, Default)]
pub struct QualityStats {
    /// Total tracks in library
    pub total: i64,
    /// Tracks never quality-checked
    pub unchecked: i64,
    /// Tracks with excellent quality (90+)
    pub excellent: i64,
    /// Tracks with good quality (70-89)
    pub good: i64,
    /// Tracks with fair quality (50-69)
    pub fair: i64,
    /// Tracks with poor quality (<50)
    pub poor: i64,
}

/// Get quality statistics for the library.
pub async fn get_quality_stats(pool: &SqlitePool) -> sqlx::Result<QualityStats> {
    let row: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN quality_score IS NULL THEN 1 ELSE 0 END) as unchecked,
            SUM(CASE WHEN quality_score >= 90 THEN 1 ELSE 0 END) as excellent,
            SUM(CASE WHEN quality_score >= 70 AND quality_score < 90 THEN 1 ELSE 0 END) as good,
            SUM(CASE WHEN quality_score >= 50 AND quality_score < 70 THEN 1 ELSE 0 END) as fair,
            SUM(CASE WHEN quality_score < 50 THEN 1 ELSE 0 END) as poor
        FROM tracks
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(QualityStats {
        total: row.0,
        unchecked: row.1,
        excellent: row.2,
        good: row.3,
        fair: row.4,
        poor: row.5,
    })
}

// ============================================================================
// Alternative Matches
// ============================================================================

/// A candidate match from fingerprint identification.
#[derive(Debug, Clone)]
pub struct TrackMatch {
    pub id: i64,
    pub track_id: i64,
    pub source: String,
    pub confidence: f32,
    pub recording_id: Option<String>,
    pub recording_title: String,
    pub recording_artist: Option<String>,
    pub title_similarity: Option<f32>,
    pub artist_similarity: Option<f32>,
    pub is_selected: bool,
    pub is_rejected: bool,
}

/// A release (album) option for a match.
#[derive(Debug, Clone)]
pub struct MatchRelease {
    pub id: i64,
    pub match_id: i64,
    pub release_id: String,
    pub release_title: String,
    pub release_artist: Option<String>,
    pub release_year: Option<i32>,
    pub release_type: Option<String>,
    pub track_number: Option<i32>,
    pub is_original_release: bool,
    pub is_compilation: bool,
    pub is_preferred: bool,
}

/// Store a new fingerprint match for a track.
///
/// If a match with the same recording_id already exists, updates it.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_track_match(
    pool: &SqlitePool,
    track_id: i64,
    source: &str,
    confidence: f32,
    recording_id: Option<&str>,
    recording_title: &str,
    recording_artist: Option<&str>,
    title_similarity: Option<f32>,
    artist_similarity: Option<f32>,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        r#"INSERT INTO track_matches 
           (track_id, source, confidence, recording_id, recording_title, 
            recording_artist, title_similarity, artist_similarity)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(track_id, recording_id) DO UPDATE SET
             confidence = excluded.confidence,
             title_similarity = excluded.title_similarity,
             artist_similarity = excluded.artist_similarity,
             discovered_at = CURRENT_TIMESTAMP
           RETURNING id"#,
    )
    .bind(track_id)
    .bind(source)
    .bind(confidence as f64)
    .bind(recording_id)
    .bind(recording_title)
    .bind(recording_artist)
    .bind(title_similarity.map(|v| v as f64))
    .bind(artist_similarity.map(|v| v as f64))
    .fetch_one(pool)
    .await?;

    Ok(sqlx::Row::get(&result, 0))
}

/// Store a release option for a match.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_match_release(
    pool: &SqlitePool,
    match_id: i64,
    release_id: &str,
    release_title: &str,
    release_artist: Option<&str>,
    release_year: Option<i32>,
    release_type: Option<&str>,
    track_number: Option<i32>,
    is_original_release: bool,
    is_compilation: bool,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        r#"INSERT INTO match_releases
           (match_id, release_id, release_title, release_artist, release_year,
            release_type, track_number, is_original_release, is_compilation)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(match_id, release_id) DO UPDATE SET
             release_title = excluded.release_title,
             release_year = excluded.release_year,
             track_number = excluded.track_number
           RETURNING id"#,
    )
    .bind(match_id)
    .bind(release_id)
    .bind(release_title)
    .bind(release_artist)
    .bind(release_year)
    .bind(release_type)
    .bind(track_number)
    .bind(is_original_release)
    .bind(is_compilation)
    .fetch_one(pool)
    .await?;

    Ok(sqlx::Row::get(&result, 0))
}

/// Get all matches for a track, ordered by confidence.
pub async fn get_track_matches(pool: &SqlitePool, track_id: i64) -> sqlx::Result<Vec<TrackMatch>> {
    sqlx::query_as::<_, TrackMatch>(
        r#"SELECT id, track_id, source, confidence, recording_id,
                  recording_title, recording_artist, title_similarity,
                  artist_similarity, is_selected, is_rejected
           FROM track_matches
           WHERE track_id = ?
           ORDER BY confidence DESC"#,
    )
    .bind(track_id)
    .fetch_all(pool)
    .await
}

/// Get releases for a match.
pub async fn get_match_releases(
    pool: &SqlitePool,
    match_id: i64,
) -> sqlx::Result<Vec<MatchRelease>> {
    sqlx::query_as::<_, MatchRelease>(
        r#"SELECT id, match_id, release_id, release_title, release_artist,
                  release_year, release_type, track_number, 
                  is_original_release, is_compilation, is_preferred
           FROM match_releases
           WHERE match_id = ?
           ORDER BY is_original_release DESC, release_year ASC"#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

/// Mark a match as selected (user chose this one).
pub async fn select_track_match(pool: &SqlitePool, match_id: i64) -> sqlx::Result<()> {
    // First, get the track_id for this match
    let track_id: (i64,) = sqlx::query_as("SELECT track_id FROM track_matches WHERE id = ?")
        .bind(match_id)
        .fetch_one(pool)
        .await?;

    // Clear selection on all matches for this track
    sqlx::query("UPDATE track_matches SET is_selected = FALSE WHERE track_id = ?")
        .bind(track_id.0)
        .execute(pool)
        .await?;

    // Set this match as selected
    sqlx::query(
        "UPDATE track_matches SET is_selected = TRUE, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(match_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a match as rejected (user doesn't want this one).
pub async fn reject_track_match(pool: &SqlitePool, match_id: i64) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE track_matches SET is_rejected = TRUE, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(match_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark a release as preferred for a match.
pub async fn prefer_release(pool: &SqlitePool, release_id: i64) -> sqlx::Result<()> {
    // Get the match_id for this release
    let match_id: (i64,) = sqlx::query_as("SELECT match_id FROM match_releases WHERE id = ?")
        .bind(release_id)
        .fetch_one(pool)
        .await?;

    // Clear preference on all releases for this match
    sqlx::query("UPDATE match_releases SET is_preferred = FALSE WHERE match_id = ?")
        .bind(match_id.0)
        .execute(pool)
        .await?;

    // Set this release as preferred
    sqlx::query("UPDATE match_releases SET is_preferred = TRUE WHERE id = ?")
        .bind(release_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get tracks that have unreviewed matches.
pub async fn get_tracks_with_pending_matches(
    pool: &SqlitePool,
    limit: u32,
) -> sqlx::Result<Vec<(i64, i64)>> {
    // Returns (track_id, match_count)
    sqlx::query_as::<_, (i64, i64)>(
        r#"SELECT track_id, COUNT(*) as match_count
           FROM track_matches
           WHERE is_selected = FALSE AND is_rejected = FALSE
           GROUP BY track_id
           ORDER BY match_count DESC
           LIMIT ?"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Delete all matches for a track (e.g., before re-fingerprinting).
pub async fn clear_track_matches(pool: &SqlitePool, track_id: i64) -> sqlx::Result<u64> {
    let result = sqlx::query("DELETE FROM track_matches WHERE track_id = ?")
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// Implement FromRow for TrackMatch
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for TrackMatch {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(TrackMatch {
            id: row.get("id"),
            track_id: row.get("track_id"),
            source: row.get("source"),
            confidence: row.get::<f64, _>("confidence") as f32,
            recording_id: row.get("recording_id"),
            recording_title: row.get("recording_title"),
            recording_artist: row.get("recording_artist"),
            title_similarity: row
                .get::<Option<f64>, _>("title_similarity")
                .map(|v| v as f32),
            artist_similarity: row
                .get::<Option<f64>, _>("artist_similarity")
                .map(|v| v as f32),
            is_selected: row.get("is_selected"),
            is_rejected: row.get("is_rejected"),
        })
    }
}

// Implement FromRow for MatchRelease
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for MatchRelease {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(MatchRelease {
            id: row.get("id"),
            match_id: row.get("match_id"),
            release_id: row.get("release_id"),
            release_title: row.get("release_title"),
            release_artist: row.get("release_artist"),
            release_year: row.get("release_year"),
            release_type: row.get("release_type"),
            track_number: row.get("track_number"),
            is_original_release: row.get("is_original_release"),
            is_compilation: row.get("is_compilation"),
            is_preferred: row.get("is_preferred"),
        })
    }
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
