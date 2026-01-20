//! Database operation retry logic with exponential backoff.
//!
//! SQLite can return `SQLITE_BUSY` or `SQLITE_LOCKED` errors when the database
//! is locked by another connection. This module provides retry logic to handle
//! these transient errors gracefully.
//!
//! # Example
//!
//! ```ignore
//! use soundstore::db::retry::with_retry;
//!
//! let result = with_retry(|| async {
//!     // Your database operation
//!     sqlx::query!("INSERT INTO tracks ...")
//!         .execute(&pool)
//!         .await
//! }).await?;
//! ```

use std::future::Future;
use std::time::Duration;

/// Maximum number of retry attempts for database operations
const MAX_ATTEMPTS: u32 = 3;

/// Initial backoff delay in milliseconds
const INITIAL_BACKOFF_MS: u64 = 100;

/// Execute a database operation with retry logic for transient errors.
///
/// Automatically retries operations that fail with `SQLITE_BUSY` or `SQLITE_LOCKED`
/// errors using exponential backoff (100ms, 200ms, 400ms).
///
/// # Arguments
///
/// * `operation` - An async function that returns a `Result<T, sqlx::Error>`
///
/// # Returns
///
/// The result of the operation, or the last error if all retries are exhausted.
///
/// # Errors
///
/// Returns the last error encountered if:
/// - All retry attempts are exhausted
/// - A non-retryable error occurs (e.g., constraint violation, syntax error)
pub async fn with_retry<F, Fut, T>(mut operation: F) -> Result<T, sqlx::Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    let mut attempts = 0;
    let mut last_error: Option<sqlx::Error> = None;

    while attempts < MAX_ATTEMPTS {
        attempts += 1;

        // Exponential backoff: 0ms (first attempt), 100ms, 200ms
        if attempts > 1 {
            let delay_ms = INITIAL_BACKOFF_MS * (1 << (attempts - 2)); // 100ms, 200ms, 400ms
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            tracing::debug!(
                "Retrying database operation (attempt {}/{})",
                attempts,
                MAX_ATTEMPTS
            );
        }

        // Execute the operation
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if this is a retryable error
                if is_retryable_error(&e) {
                    tracing::warn!(
                        "Database operation failed with retryable error (attempt {}/{}): {}",
                        attempts,
                        MAX_ATTEMPTS,
                        e
                    );
                    last_error = Some(e);
                    continue;
                } else {
                    // Non-retryable error - fail immediately
                    tracing::error!("Database operation failed with non-retryable error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    // All retries exhausted - return the last error
    Err(last_error
        .unwrap_or_else(|| sqlx::Error::Io(std::io::Error::other("All retry attempts exhausted"))))
}

/// Check if a database error is retryable (transient lock/busy condition).
///
/// Returns `true` for:
/// - `SQLITE_BUSY` - Database file is locked
/// - `SQLITE_LOCKED` - Table is locked
///
/// Returns `false` for:
/// - Constraint violations (UNIQUE, FOREIGN KEY, etc.)
/// - Syntax errors
/// - Type mismatches
/// - Connection errors
fn is_retryable_error(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_err) => {
            // Check SQLite error codes
            // SQLITE_BUSY = 5, SQLITE_LOCKED = 6
            let code = db_err.code();
            if let Some(code_str) = code {
                matches!(
                    code_str.as_ref(),
                    "5" | "6" | "SQLITE_BUSY" | "SQLITE_LOCKED"
                )
            } else {
                false
            }
        }
        // Pool timeout might also be retryable
        sqlx::Error::PoolTimedOut => true,
        // All other errors are not retryable
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let result = with_retry(move || {
            let count = Arc::clone(&call_count_clone);
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    // Simulate SQLITE_BUSY on first two attempts
                    Err(sqlx::Error::Database(Box::new(MockDatabaseError {
                        code: "5".to_string(),
                    })))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_fails_on_non_retryable_error() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let result: sqlx::Result<()> = with_retry(move || {
            let count = Arc::clone(&call_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                // Simulate constraint violation (non-retryable)
                Err(sqlx::Error::Database(Box::new(MockDatabaseError {
                    code: "19".to_string(), // SQLITE_CONSTRAINT
                })))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // Should fail immediately without retry
    }

    #[tokio::test]
    async fn test_retry_exhausts_attempts() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let result: sqlx::Result<()> = with_retry(move || {
            let count = Arc::clone(&call_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                // Always return SQLITE_BUSY
                Err(sqlx::Error::Database(Box::new(MockDatabaseError {
                    code: "5".to_string(),
                })))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), MAX_ATTEMPTS as usize);
    }

    // Mock database error for testing
    #[derive(Debug)]
    struct MockDatabaseError {
        code: String,
    }

    impl std::fmt::Display for MockDatabaseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Mock database error: {}", self.code)
        }
    }

    impl std::error::Error for MockDatabaseError {}

    impl sqlx::error::DatabaseError for MockDatabaseError {
        fn message(&self) -> &str {
            "Mock database error"
        }

        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(std::borrow::Cow::Borrowed(&self.code))
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
    }
}
