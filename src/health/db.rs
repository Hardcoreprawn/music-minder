//! Database operations for file health records.
//!
//! Provides CRUD operations and queries for the `file_health` table.

use chrono::Utc;
use sqlx::sqlite::SqlitePool;
use std::path::Path;

use super::hash::compute_file_hash;
use super::types::{ErrorType, FileHealth, HealthStatus};

// ============================================================================
// Database Row Types
// ============================================================================

/// Database row for file_health table.
#[derive(Debug, sqlx::FromRow)]
struct FileHealthRow {
    id: i64,
    path: String,
    status: String,
    error_type: Option<String>,
    error_message: Option<String>,
    acoustid_fingerprint: Option<String>,
    acoustid_confidence: Option<f64>,
    musicbrainz_id: Option<String>,
    file_size: Option<i64>,
    file_hash: Option<String>,
    last_checked: String,
}

impl From<FileHealthRow> for FileHealth {
    fn from(row: FileHealthRow) -> Self {
        FileHealth {
            id: Some(row.id),
            path: row.path,
            status: row.status.parse().unwrap_or(HealthStatus::Unknown),
            error_type: row
                .error_type
                .map(|s| s.parse().unwrap_or(ErrorType::Other("unknown".into()))),
            error_message: row.error_message,
            acoustid_fingerprint: row.acoustid_fingerprint,
            acoustid_confidence: row.acoustid_confidence,
            musicbrainz_id: row.musicbrainz_id,
            file_size: row.file_size,
            file_hash: row.file_hash,
            last_checked: row.last_checked.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Insert or update a file health record.
///
/// Uses SQLite's UPSERT (INSERT ON CONFLICT UPDATE) to either create
/// a new record or update an existing one based on the file path.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `health` - Health record to upsert
///
/// # Returns
///
/// The database ID of the upserted record.
pub async fn upsert_health(pool: &SqlitePool, health: &FileHealth) -> sqlx::Result<i64> {
    let last_checked = health.last_checked.to_rfc3339();
    let error_type = health.error_type.as_ref().map(|e| e.as_str().to_string());

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO file_health (
            path, status, error_type, error_message,
            acoustid_fingerprint, acoustid_confidence, musicbrainz_id,
            file_size, file_hash, last_checked
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
            status = excluded.status,
            error_type = excluded.error_type,
            error_message = excluded.error_message,
            acoustid_fingerprint = excluded.acoustid_fingerprint,
            acoustid_confidence = excluded.acoustid_confidence,
            musicbrainz_id = excluded.musicbrainz_id,
            file_size = excluded.file_size,
            file_hash = excluded.file_hash,
            last_checked = excluded.last_checked
        RETURNING id
        "#,
    )
    .bind(&health.path)
    .bind(health.status.as_str())
    .bind(&error_type)
    .bind(&health.error_message)
    .bind(&health.acoustid_fingerprint)
    .bind(health.acoustid_confidence)
    .bind(&health.musicbrainz_id)
    .bind(health.file_size)
    .bind(&health.file_hash)
    .bind(&last_checked)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Get health record for a specific file.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `path` - File path to look up
///
/// # Returns
///
/// The health record if found, or None.
pub async fn get_health(pool: &SqlitePool, path: &str) -> sqlx::Result<Option<FileHealth>> {
    let row: Option<FileHealthRow> = sqlx::query_as("SELECT * FROM file_health WHERE path = ?")
        .bind(path)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.into()))
}

/// Get all files with a specific status.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `status` - Status to filter by
///
/// # Returns
///
/// All health records matching the status, ordered by path.
pub async fn get_by_status(
    pool: &SqlitePool,
    status: HealthStatus,
) -> sqlx::Result<Vec<FileHealth>> {
    let rows: Vec<FileHealthRow> =
        sqlx::query_as("SELECT * FROM file_health WHERE status = ? ORDER BY path")
            .bind(status.as_str())
            .fetch_all(pool)
            .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Get all error files with their messages.
///
/// # Returns
///
/// All health records with error status, ordered by error type then path.
pub async fn get_errors(pool: &SqlitePool) -> sqlx::Result<Vec<FileHealth>> {
    let rows: Vec<FileHealthRow> = sqlx::query_as(
        "SELECT * FROM file_health WHERE status = 'error' ORDER BY error_type, path",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Delete health record for a file.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `path` - File path to delete
///
/// # Returns
///
/// True if a record was deleted, false if no record existed.
pub async fn delete_health(pool: &SqlitePool, path: &str) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM file_health WHERE path = ?")
        .bind(path)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Aggregation & Analysis
// ============================================================================

/// Health summary counts by status.
#[derive(Debug, Default)]
pub struct HealthSummary {
    /// Total number of records
    pub total: i64,
    /// Files with Ok status
    pub ok: i64,
    /// Files with Error status
    pub errors: i64,
    /// Files with NoMatch status
    pub no_match: i64,
    /// Files with LowConfidence status
    pub low_confidence: i64,
}

/// Get health summary counts grouped by status.
///
/// # Returns
///
/// Aggregated counts for each health status.
pub async fn get_summary(pool: &SqlitePool) -> sqlx::Result<HealthSummary> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT status, COUNT(*) as count FROM file_health GROUP BY status")
            .fetch_all(pool)
            .await?;

    let mut summary = HealthSummary::default();
    for (status, count) in rows {
        summary.total += count;
        match status.as_str() {
            "ok" => summary.ok = count,
            "error" => summary.errors = count,
            "no_match" => summary.no_match = count,
            "low_confidence" => summary.low_confidence = count,
            _ => {}
        }
    }

    Ok(summary)
}

/// Check if a file has changed since last check (by comparing hash).
///
/// Computes the current file hash and compares it to the stored hash.
/// Returns true if the file has changed or if no previous record exists.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `path` - Path to the file to check
pub async fn has_file_changed(pool: &SqlitePool, path: &Path) -> sqlx::Result<bool> {
    let path_str = path.to_string_lossy().to_string();

    if let Some(health) = get_health(pool, &path_str).await?
        && let Some(stored_hash) = health.file_hash
        && let Ok(current_hash) = compute_file_hash(path)
    {
        return Ok(stored_hash != current_hash);
    }

    // No previous record or can't compute hash - treat as changed
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_upsert_and_get_health() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = crate::db::init_db(&db_url).await.unwrap();

        let health = FileHealth::ok("/test/file.mp3", 0.95, None);
        let id = upsert_health(&pool, &health).await.unwrap();
        assert!(id > 0);

        let retrieved = get_health(&pool, "/test/file.mp3").await.unwrap().unwrap();
        assert_eq!(retrieved.status, HealthStatus::Ok);
        assert_eq!(retrieved.acoustid_confidence, Some(0.95));
    }

    #[tokio::test]
    async fn test_get_summary() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = crate::db::init_db(&db_url).await.unwrap();

        // Add some records
        upsert_health(&pool, &FileHealth::ok("/ok1.mp3", 0.9, None))
            .await
            .unwrap();
        upsert_health(&pool, &FileHealth::ok("/ok2.mp3", 0.85, None))
            .await
            .unwrap();
        upsert_health(
            &pool,
            &FileHealth::error("/bad.mp3", ErrorType::DecodeError, "corrupt"),
        )
        .await
        .unwrap();
        upsert_health(&pool, &FileHealth::no_match("/unknown.mp3"))
            .await
            .unwrap();

        let summary = get_summary(&pool).await.unwrap();
        assert_eq!(summary.total, 4);
        assert_eq!(summary.ok, 2);
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.no_match, 1);
    }

    #[tokio::test]
    async fn test_get_by_status() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = crate::db::init_db(&db_url).await.unwrap();

        upsert_health(&pool, &FileHealth::ok("/ok1.mp3", 0.9, None))
            .await
            .unwrap();
        upsert_health(
            &pool,
            &FileHealth::error("/bad1.mp3", ErrorType::DecodeError, "err1"),
        )
        .await
        .unwrap();
        upsert_health(
            &pool,
            &FileHealth::error("/bad2.mp3", ErrorType::EmptyFingerprint, "err2"),
        )
        .await
        .unwrap();

        let errors = get_by_status(&pool, HealthStatus::Error).await.unwrap();
        assert_eq!(errors.len(), 2);

        let ok = get_by_status(&pool, HealthStatus::Ok).await.unwrap();
        assert_eq!(ok.len(), 1);
    }
}
