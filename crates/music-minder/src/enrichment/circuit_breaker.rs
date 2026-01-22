//! Circuit breaker pattern for API calls
//!
//! Prevents cascading failures by failing fast when a service is down.
//!
//! ## States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Service is down, requests fail immediately without network calls
//! - **Half-Open**: Testing recovery, single request allowed to probe service
//!
//! ## Configuration
//!
//! - **Failure threshold**: 5 consecutive failures opens the circuit
//! - **Cooldown period**: 60 seconds before attempting recovery
//! - **Success threshold**: 1 successful request closes the circuit from half-open
//!
//! ## Example
//!
//! ```ignore
//! let breaker = CircuitBreaker::new("AcoustID");
//!
//! match breaker.call(|| async { api_client.lookup(fingerprint).await }).await {
//!     Ok(result) => // Success
//!     Err(CircuitBreakerError::CircuitOpen) => // Service is down, fail fast
//!     Err(CircuitBreakerError::Request(e)) => // Request failed
//! }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Number of consecutive failures before opening circuit
const FAILURE_THRESHOLD: u32 = 5;

/// How long to wait before testing recovery (in seconds)
const COOLDOWN_SECONDS: u64 = 60;

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq)]
enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Service is down - requests fail immediately
    Open { opened_at: Instant },
    /// Testing recovery - single request allowed
    HalfOpen,
}

/// Circuit breaker implementation
///
/// Thread-safe, can be shared across multiple tasks.
#[derive(Clone)]
pub struct CircuitBreaker {
    name: String,
    state: Arc<RwLock<CircuitState>>,
    consecutive_failures: Arc<RwLock<u32>>,
}

/// Circuit breaker errors
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open - service is down
    #[error("Circuit breaker '{0}' is open - service appears to be down")]
    CircuitOpen(String),

    /// Request failed
    #[error("Request failed: {0}")]
    Request(#[source] E),
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            consecutive_failures: Arc::new(RwLock::new(0)),
        }
    }

    /// Execute a request through the circuit breaker
    ///
    /// Returns:
    /// - `Ok(T)` on successful request
    /// - `Err(CircuitBreakerError::CircuitOpen)` if circuit is open
    /// - `Err(CircuitBreakerError::Request(E))` if request fails
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        // Check circuit state
        let should_attempt = {
            let mut state = self.state.write().await;
            match *state {
                CircuitState::Closed => true,
                CircuitState::Open { opened_at } => {
                    // Check if cooldown period has elapsed
                    if opened_at.elapsed() >= Duration::from_secs(COOLDOWN_SECONDS) {
                        tracing::info!(
                            "Circuit breaker '{}' entering half-open state (cooldown elapsed)",
                            self.name
                        );
                        *state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                }
                CircuitState::HalfOpen => true,
            }
        };

        if !should_attempt {
            tracing::debug!("Circuit breaker '{}' is open - failing fast", self.name);
            return Err(CircuitBreakerError::CircuitOpen(self.name.clone()));
        }

        // Attempt the request
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(CircuitBreakerError::Request(e))
            }
        }
    }

    /// Record a successful request
    async fn on_success(&self) {
        let mut state = self.state.write().await;
        let mut failures = self.consecutive_failures.write().await;

        match *state {
            CircuitState::Closed => {
                // Already closed, just reset failure count
                *failures = 0;
            }
            CircuitState::HalfOpen => {
                tracing::info!(
                    "Circuit breaker '{}' closing (recovery successful)",
                    self.name
                );
                *state = CircuitState::Closed;
                *failures = 0;
            }
            CircuitState::Open { .. } => {
                // Should not happen, but handle gracefully
                tracing::warn!(
                    "Circuit breaker '{}' received success while open",
                    self.name
                );
                *state = CircuitState::Closed;
                *failures = 0;
            }
        }
    }

    /// Record a failed request
    async fn on_failure(&self) {
        let mut state = self.state.write().await;
        let mut failures = self.consecutive_failures.write().await;

        match *state {
            CircuitState::Closed => {
                *failures += 1;
                if *failures >= FAILURE_THRESHOLD {
                    tracing::warn!(
                        "Circuit breaker '{}' opening ({} consecutive failures)",
                        self.name,
                        failures
                    );
                    *state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!(
                    "Circuit breaker '{}' reopening (recovery failed)",
                    self.name
                );
                *state = CircuitState::Open {
                    opened_at: Instant::now(),
                };
                *failures += 1;
            }
            CircuitState::Open { .. } => {
                // Already open, nothing to do
                *failures += 1;
            }
        }
    }

    /// Get current state for diagnostics
    #[allow(dead_code)]
    pub async fn state(&self) -> String {
        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => "closed".to_string(),
            CircuitState::Open { opened_at } => {
                let elapsed = opened_at.elapsed().as_secs();
                format!("open ({}s ago)", elapsed)
            }
            CircuitState::HalfOpen => "half-open".to_string(),
        }
    }

    /// Get consecutive failure count for diagnostics
    #[allow(dead_code)]
    pub async fn failure_count(&self) -> u32 {
        *self.consecutive_failures.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_closed_state_allows_requests() {
        let breaker = CircuitBreaker::new("test");

        let result = breaker
            .call(|| async { Ok::<_, String>(42) })
            .await
            .unwrap();

        assert_eq!(result, 42);
        assert_eq!(breaker.state().await, "closed");
    }

    #[tokio::test]
    async fn test_opens_after_threshold_failures() {
        let breaker = CircuitBreaker::new("test");

        // Fail FAILURE_THRESHOLD times
        for _ in 0..FAILURE_THRESHOLD {
            let _ = breaker
                .call(|| async { Err::<i32, _>("error".to_string()) })
                .await;
        }

        assert!(breaker.state().await.starts_with("open"));

        // Next request should fail fast
        let result = breaker.call(|| async { Ok::<_, String>(42) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen(_))));
    }

    #[tokio::test]
    async fn test_half_open_after_cooldown() {
        let breaker = CircuitBreaker::new("test");

        // Open the circuit
        for _ in 0..FAILURE_THRESHOLD {
            let _ = breaker
                .call(|| async { Err::<i32, _>("error".to_string()) })
                .await;
        }

        // Manually set opened_at to past to simulate cooldown
        {
            let mut state = breaker.state.write().await;
            *state = CircuitState::Open {
                opened_at: Instant::now() - Duration::from_secs(COOLDOWN_SECONDS + 1),
            };
        }

        // Next request should enter half-open and execute
        let result = breaker.call(|| async { Ok::<_, String>(42) }).await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(breaker.state().await, "closed");
    }

    #[tokio::test]
    async fn test_resets_on_success() {
        let breaker = CircuitBreaker::new("test");

        // Some failures
        for _ in 0..3 {
            let _ = breaker
                .call(|| async { Err::<i32, _>("error".to_string()) })
                .await;
        }

        assert_eq!(breaker.failure_count().await, 3);

        // Success resets counter
        let _ = breaker.call(|| async { Ok::<_, String>(42) }).await;
        assert_eq!(breaker.failure_count().await, 0);
    }

    #[tokio::test]
    async fn test_reopens_from_half_open_on_failure() {
        let breaker = CircuitBreaker::new("test");

        // Open the circuit
        for _ in 0..FAILURE_THRESHOLD {
            let _ = breaker
                .call(|| async { Err::<i32, _>("error".to_string()) })
                .await;
        }

        // Simulate cooldown
        {
            let mut state = breaker.state.write().await;
            *state = CircuitState::Open {
                opened_at: Instant::now() - Duration::from_secs(COOLDOWN_SECONDS + 1),
            };
        }

        // Request in half-open fails - should reopen
        let _ = breaker
            .call(|| async { Err::<i32, _>("error".to_string()) })
            .await;

        assert!(breaker.state().await.starts_with("open"));
    }
}
