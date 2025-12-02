//! File health tracking module
//! 
//! Tracks the health status of audio files to identify corrupt, 
//! problematic, or unidentifiable files in your music library.

use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use sqlx::sqlite::SqlitePool;
use std::path::Path;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Health status of an audio file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// File is healthy - fingerprinted and identified successfully
    Ok,
    /// File has an error (decode failure, corrupt, etc.)
    Error,
    /// File fingerprinted but no match in AcoustID database
    NoMatch,
    /// File matched but with low confidence
    LowConfidence,
    /// Not yet checked
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "ok",
            HealthStatus::Error => "error",
            HealthStatus::NoMatch => "no_match",
            HealthStatus::LowConfidence => "low_confidence",
            HealthStatus::Unknown => "unknown",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            HealthStatus::Ok => "✓",
            HealthStatus::Error => "✗",
            HealthStatus::NoMatch => "?",
            HealthStatus::LowConfidence => "~",
            HealthStatus::Unknown => "-",
        }
    }
}

impl std::str::FromStr for HealthStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ok" => HealthStatus::Ok,
            "error" => HealthStatus::Error,
            "no_match" => HealthStatus::NoMatch,
            "low_confidence" => HealthStatus::LowConfidence,
            _ => HealthStatus::Unknown,
        })
    }
}

/// Type of error encountered
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    DecodeError,
    EmptyFingerprint,
    IoError,
    Timeout,
    ApiError,
    Other(String),
}

impl ErrorType {
    pub fn as_str(&self) -> &str {
        match self {
            ErrorType::DecodeError => "decode_error",
            ErrorType::EmptyFingerprint => "empty_fingerprint",
            ErrorType::IoError => "io_error",
            ErrorType::Timeout => "timeout",
            ErrorType::ApiError => "api_error",
            ErrorType::Other(s) => s,
        }
    }
}

impl std::str::FromStr for ErrorType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "decode_error" => ErrorType::DecodeError,
            "empty_fingerprint" => ErrorType::EmptyFingerprint,
            "io_error" => ErrorType::IoError,
            "timeout" => ErrorType::Timeout,
            "api_error" => ErrorType::ApiError,
            s => ErrorType::Other(s.to_string()),
        })
    }
}

/// A file health record
#[derive(Debug, Clone)]
pub struct FileHealth {
    pub id: Option<i64>,
    pub path: String,
    pub status: HealthStatus,
    pub error_type: Option<ErrorType>,
    pub error_message: Option<String>,
    pub acoustid_fingerprint: Option<String>,
    pub acoustid_confidence: Option<f64>,
    pub musicbrainz_id: Option<String>,
    pub file_size: Option<i64>,
    pub file_hash: Option<String>,
    pub last_checked: DateTime<Utc>,
}

impl FileHealth {
    /// Create a new healthy file record
    pub fn ok(path: impl Into<String>, confidence: f64, musicbrainz_id: Option<String>) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::Ok,
            error_type: None,
            error_message: None,
            acoustid_fingerprint: None,
            acoustid_confidence: Some(confidence),
            musicbrainz_id,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Create a record for a file with an error
    pub fn error(path: impl Into<String>, error_type: ErrorType, message: impl Into<String>) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::Error,
            error_type: Some(error_type),
            error_message: Some(message.into()),
            acoustid_fingerprint: None,
            acoustid_confidence: None,
            musicbrainz_id: None,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Create a record for a file with no match
    pub fn no_match(path: impl Into<String>) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::NoMatch,
            error_type: None,
            error_message: None,
            acoustid_fingerprint: None,
            acoustid_confidence: None,
            musicbrainz_id: None,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Create a record for a file with low confidence match
    pub fn low_confidence(path: impl Into<String>, confidence: f64) -> Self {
        Self {
            id: None,
            path: path.into(),
            status: HealthStatus::LowConfidence,
            error_type: None,
            error_message: None,
            acoustid_fingerprint: None,
            acoustid_confidence: Some(confidence),
            musicbrainz_id: None,
            file_size: None,
            file_hash: None,
            last_checked: Utc::now(),
        }
    }

    /// Add file metadata (size and hash)
    pub fn with_file_info(mut self, path: &Path) -> Self {
        if let Ok(metadata) = std::fs::metadata(path) {
            self.file_size = Some(metadata.len() as i64);
        }
        if let Ok(hash) = compute_file_hash(path) {
            self.file_hash = Some(hash);
        }
        self
    }

    /// Add fingerprint
    pub fn with_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.acoustid_fingerprint = Some(fingerprint.into());
        self
    }
}

/// Compute a partial hash of a file (first 1MB + last 1MB)
/// This is fast for large files while still detecting changes
pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    
    let mut hasher = Sha256::new();
    
    // Hash file size first (so different sized files have different hashes)
    hasher.update(file_size.to_le_bytes());
    
    const CHUNK_SIZE: u64 = 1024 * 1024; // 1MB
    
    if file_size <= CHUNK_SIZE * 2 {
        // Small file - hash the whole thing
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        hasher.update(&buffer);
    } else {
        // Large file - hash first and last 1MB
        let mut buffer = vec![0u8; CHUNK_SIZE as usize];
        
        // First chunk
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);
        
        // Last chunk
        file.seek(SeekFrom::End(-(CHUNK_SIZE as i64)))?;
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

// ============================================================================
// Database Operations
// ============================================================================

/// Database row for file_health
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
            error_type: row.error_type.map(|s| s.parse().unwrap_or(ErrorType::Other("unknown".into()))),
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

/// Upsert a file health record
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

/// Get health record for a specific file
pub async fn get_health(pool: &SqlitePool, path: &str) -> sqlx::Result<Option<FileHealth>> {
    let row: Option<FileHealthRow> = sqlx::query_as(
        "SELECT * FROM file_health WHERE path = ?"
    )
    .bind(path)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| r.into()))
}

/// Get all files with a specific status
pub async fn get_by_status(pool: &SqlitePool, status: HealthStatus) -> sqlx::Result<Vec<FileHealth>> {
    let rows: Vec<FileHealthRow> = sqlx::query_as(
        "SELECT * FROM file_health WHERE status = ? ORDER BY path"
    )
    .bind(status.as_str())
    .fetch_all(pool)
    .await?;
    
    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Get health summary counts
#[derive(Debug, Default)]
pub struct HealthSummary {
    pub total: i64,
    pub ok: i64,
    pub errors: i64,
    pub no_match: i64,
    pub low_confidence: i64,
}

pub async fn get_summary(pool: &SqlitePool) -> sqlx::Result<HealthSummary> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) as count FROM file_health GROUP BY status"
    )
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

/// Get all error files with their messages
pub async fn get_errors(pool: &SqlitePool) -> sqlx::Result<Vec<FileHealth>> {
    let rows: Vec<FileHealthRow> = sqlx::query_as(
        "SELECT * FROM file_health WHERE status = 'error' ORDER BY error_type, path"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Delete health record for a file
pub async fn delete_health(pool: &SqlitePool, path: &str) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM file_health WHERE path = ?")
        .bind(path)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}

/// Check if a file has changed since last check (by comparing hash)
pub async fn has_file_changed(pool: &SqlitePool, path: &Path) -> sqlx::Result<bool> {
    let path_str = path.to_string_lossy().to_string();
    
    if let Some(health) = get_health(pool, &path_str).await?
        && let Some(stored_hash) = health.file_hash
            && let Ok(current_hash) = compute_file_hash(path) {
                return Ok(stored_hash != current_hash);
            }
    
    // No previous record or can't compute hash - treat as changed
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn test_health_status_roundtrip() {
        for status in [
            HealthStatus::Ok,
            HealthStatus::Error,
            HealthStatus::NoMatch,
            HealthStatus::LowConfidence,
            HealthStatus::Unknown,
        ] {
            assert_eq!(status.as_str().parse::<HealthStatus>().unwrap(), status);
        }
    }

    #[test]
    fn test_error_type_roundtrip() {
        for error_type in [
            ErrorType::DecodeError,
            ErrorType::EmptyFingerprint,
            ErrorType::IoError,
            ErrorType::Timeout,
            ErrorType::ApiError,
        ] {
            assert_eq!(error_type.as_str().parse::<ErrorType>().unwrap(), error_type);
        }
    }

    #[test]
    fn test_file_health_ok() {
        let health = FileHealth::ok("/test/file.mp3", 0.95, Some("mb-123".to_string()));
        assert_eq!(health.status, HealthStatus::Ok);
        assert_eq!(health.acoustid_confidence, Some(0.95));
        assert_eq!(health.musicbrainz_id, Some("mb-123".to_string()));
    }

    #[test]
    fn test_file_health_error() {
        let health = FileHealth::error("/test/file.mp3", ErrorType::DecodeError, "corrupt audio");
        assert_eq!(health.status, HealthStatus::Error);
        assert_eq!(health.error_type, Some(ErrorType::DecodeError));
        assert_eq!(health.error_message, Some("corrupt audio".to_string()));
    }

    #[test]
    fn test_compute_file_hash_small_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
        drop(file);
        
        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex
        
        // Same content should give same hash
        let hash2 = compute_file_hash(&file_path).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_compute_file_hash_different_content() {
        let dir = tempdir().unwrap();
        
        let file1_path = dir.path().join("test1.txt");
        let file2_path = dir.path().join("test2.txt");
        
        std::fs::write(&file1_path, b"Content A").unwrap();
        std::fs::write(&file2_path, b"Content B").unwrap();
        
        let hash1 = compute_file_hash(&file1_path).unwrap();
        let hash2 = compute_file_hash(&file2_path).unwrap();
        
        assert_ne!(hash1, hash2);
    }

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
        upsert_health(&pool, &FileHealth::ok("/ok1.mp3", 0.9, None)).await.unwrap();
        upsert_health(&pool, &FileHealth::ok("/ok2.mp3", 0.85, None)).await.unwrap();
        upsert_health(&pool, &FileHealth::error("/bad.mp3", ErrorType::DecodeError, "corrupt")).await.unwrap();
        upsert_health(&pool, &FileHealth::no_match("/unknown.mp3")).await.unwrap();
        
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
        
        upsert_health(&pool, &FileHealth::ok("/ok1.mp3", 0.9, None)).await.unwrap();
        upsert_health(&pool, &FileHealth::error("/bad1.mp3", ErrorType::DecodeError, "err1")).await.unwrap();
        upsert_health(&pool, &FileHealth::error("/bad2.mp3", ErrorType::EmptyFingerprint, "err2")).await.unwrap();
        
        let errors = get_by_status(&pool, HealthStatus::Error).await.unwrap();
        assert_eq!(errors.len(), 2);
        
        let ok = get_by_status(&pool, HealthStatus::Ok).await.unwrap();
        assert_eq!(ok.len(), 1);
    }
}
