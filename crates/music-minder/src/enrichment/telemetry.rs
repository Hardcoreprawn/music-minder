//! Telemetry and metrics for enrichment operations.
//!
//! Tracks success/failure rates, latencies, and error patterns to enable:
//! - Data-driven prioritization of robustness improvements
//! - Visibility into production failure patterns
//! - Performance monitoring of external APIs

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::domain::EnrichmentError;

/// Global enrichment metrics collector
#[derive(Debug, Clone)]
pub struct EnrichmentMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    // Fingerprint metrics
    fingerprint_success: AtomicU64,
    fingerprint_not_found: AtomicU64,
    fingerprint_corrupted: AtomicU64,
    fingerprint_locked: AtomicU64,
    fingerprint_unsupported: AtomicU64,
    fingerprint_too_short: AtomicU64,
    fingerprint_other: AtomicU64,

    // AcoustID API metrics
    acoustid_success: AtomicU64,
    acoustid_no_matches: AtomicU64,
    acoustid_network_error: AtomicU64,
    acoustid_timeout: AtomicU64,
    acoustid_rate_limited: AtomicU64,
    acoustid_parse_error: AtomicU64,
    acoustid_other_error: AtomicU64,

    // MusicBrainz API metrics
    musicbrainz_success: AtomicU64,
    musicbrainz_not_found: AtomicU64,
    musicbrainz_network_error: AtomicU64,
    musicbrainz_timeout: AtomicU64,
    musicbrainz_rate_limited: AtomicU64,
    musicbrainz_parse_error: AtomicU64,
    musicbrainz_other_error: AtomicU64,

    // Circuit breaker metrics
    circuit_breaker_open_count: AtomicU64,
    circuit_breaker_half_open_count: AtomicU64,
    circuit_breaker_fast_fail_count: AtomicU64,

    // Performance metrics (microseconds)
    total_fingerprint_time_us: AtomicU64,
    total_acoustid_time_us: AtomicU64,
    total_musicbrainz_time_us: AtomicU64,
}

impl Default for MetricsInner {
    fn default() -> Self {
        Self {
            fingerprint_success: AtomicU64::new(0),
            fingerprint_not_found: AtomicU64::new(0),
            fingerprint_corrupted: AtomicU64::new(0),
            fingerprint_locked: AtomicU64::new(0),
            fingerprint_unsupported: AtomicU64::new(0),
            fingerprint_too_short: AtomicU64::new(0),
            fingerprint_other: AtomicU64::new(0),

            acoustid_success: AtomicU64::new(0),
            acoustid_no_matches: AtomicU64::new(0),
            acoustid_network_error: AtomicU64::new(0),
            acoustid_timeout: AtomicU64::new(0),
            acoustid_rate_limited: AtomicU64::new(0),
            acoustid_parse_error: AtomicU64::new(0),
            acoustid_other_error: AtomicU64::new(0),

            musicbrainz_success: AtomicU64::new(0),
            musicbrainz_not_found: AtomicU64::new(0),
            musicbrainz_network_error: AtomicU64::new(0),
            musicbrainz_timeout: AtomicU64::new(0),
            musicbrainz_rate_limited: AtomicU64::new(0),
            musicbrainz_parse_error: AtomicU64::new(0),
            musicbrainz_other_error: AtomicU64::new(0),

            circuit_breaker_open_count: AtomicU64::new(0),
            circuit_breaker_half_open_count: AtomicU64::new(0),
            circuit_breaker_fast_fail_count: AtomicU64::new(0),

            total_fingerprint_time_us: AtomicU64::new(0),
            total_acoustid_time_us: AtomicU64::new(0),
            total_musicbrainz_time_us: AtomicU64::new(0),
        }
    }
}

impl EnrichmentMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner::default()),
        }
    }

    /// Record a successful fingerprint generation
    pub fn record_fingerprint_success(&self, duration: Duration) {
        self.inner
            .fingerprint_success
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .total_fingerprint_time_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);

        tracing::debug!(
            target: "enrichment::metrics",
            fingerprint_duration_ms = duration.as_millis(),
            "Fingerprint generated successfully"
        );
    }

    /// Record a fingerprint error
    pub fn record_fingerprint_error(&self, error: &EnrichmentError) {
        let counter = match error {
            EnrichmentError::FingerprintError(msg) => {
                if msg.contains("not found") || msg.contains("No such file") {
                    &self.inner.fingerprint_not_found
                } else if msg.contains("corrupted") || msg.contains("invalid") {
                    &self.inner.fingerprint_corrupted
                } else if msg.contains("locked") || msg.contains("in use") {
                    &self.inner.fingerprint_locked
                } else if msg.contains("unsupported") || msg.contains("format") {
                    &self.inner.fingerprint_unsupported
                } else if msg.contains("too short") || msg.contains("duration") {
                    &self.inner.fingerprint_too_short
                } else {
                    &self.inner.fingerprint_other
                }
            }
            _ => &self.inner.fingerprint_other,
        };

        counter.fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            target: "enrichment::metrics",
            error = %error,
            error_category = classify_fingerprint_error(error),
            "Fingerprint generation failed"
        );
    }

    /// Record a successful AcoustID lookup
    pub fn record_acoustid_success(&self, duration: Duration) {
        self.inner.acoustid_success.fetch_add(1, Ordering::Relaxed);
        self.inner
            .total_acoustid_time_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);

        tracing::debug!(
            target: "enrichment::metrics",
            acoustid_duration_ms = duration.as_millis(),
            "AcoustID lookup succeeded"
        );
    }

    /// Record an AcoustID lookup error
    pub fn record_acoustid_error(&self, error: &EnrichmentError) {
        let counter = match error {
            EnrichmentError::NoMatches => &self.inner.acoustid_no_matches,
            EnrichmentError::Network(_) => &self.inner.acoustid_network_error,
            EnrichmentError::RateLimited => &self.inner.acoustid_rate_limited,
            EnrichmentError::Parse(_) | EnrichmentError::InvalidResponse(_) => {
                &self.inner.acoustid_parse_error
            }
            EnrichmentError::ApiError(msg) if msg.contains("timeout") => {
                &self.inner.acoustid_timeout
            }
            _ => &self.inner.acoustid_other_error,
        };

        counter.fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            target: "enrichment::metrics",
            error = %error,
            error_category = classify_acoustid_error(error),
            "AcoustID lookup failed"
        );
    }

    /// Record a successful MusicBrainz lookup
    pub fn record_musicbrainz_success(&self, duration: Duration) {
        self.inner
            .musicbrainz_success
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .total_musicbrainz_time_us
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);

        tracing::debug!(
            target: "enrichment::metrics",
            musicbrainz_duration_ms = duration.as_millis(),
            "MusicBrainz lookup succeeded"
        );
    }

    /// Record a MusicBrainz lookup error
    pub fn record_musicbrainz_error(&self, error: &EnrichmentError) {
        let counter = match error {
            EnrichmentError::NoMatches => &self.inner.musicbrainz_not_found,
            EnrichmentError::Network(_) => &self.inner.musicbrainz_network_error,
            EnrichmentError::RateLimited => &self.inner.musicbrainz_rate_limited,
            EnrichmentError::Parse(_) | EnrichmentError::InvalidResponse(_) => {
                &self.inner.musicbrainz_parse_error
            }
            EnrichmentError::ApiError(msg) if msg.contains("timeout") => {
                &self.inner.musicbrainz_timeout
            }
            _ => &self.inner.musicbrainz_other_error,
        };

        counter.fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            target: "enrichment::metrics",
            error = %error,
            error_category = classify_musicbrainz_error(error),
            "MusicBrainz lookup failed"
        );
    }

    /// Record circuit breaker state transition to Open
    pub fn record_circuit_breaker_opened(&self, service: &str) {
        self.inner
            .circuit_breaker_open_count
            .fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            target: "enrichment::metrics",
            service = service,
            "Circuit breaker opened"
        );
    }

    /// Record circuit breaker state transition to HalfOpen
    pub fn record_circuit_breaker_half_open(&self, service: &str) {
        self.inner
            .circuit_breaker_half_open_count
            .fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            target: "enrichment::metrics",
            service = service,
            "Circuit breaker half-open, testing recovery"
        );
    }

    /// Record circuit breaker fast-fail
    pub fn record_circuit_breaker_fast_fail(&self, service: &str) {
        self.inner
            .circuit_breaker_fast_fail_count
            .fetch_add(1, Ordering::Relaxed);

        tracing::debug!(
            target: "enrichment::metrics",
            service = service,
            "Circuit breaker fast-fail"
        );
    }

    /// Get current snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            fingerprint: FingerprintMetrics {
                success: self.inner.fingerprint_success.load(Ordering::Relaxed),
                not_found: self.inner.fingerprint_not_found.load(Ordering::Relaxed),
                corrupted: self.inner.fingerprint_corrupted.load(Ordering::Relaxed),
                locked: self.inner.fingerprint_locked.load(Ordering::Relaxed),
                unsupported: self.inner.fingerprint_unsupported.load(Ordering::Relaxed),
                too_short: self.inner.fingerprint_too_short.load(Ordering::Relaxed),
                other: self.inner.fingerprint_other.load(Ordering::Relaxed),
                total_time_us: self.inner.total_fingerprint_time_us.load(Ordering::Relaxed),
            },
            acoustid: ApiMetrics {
                success: self.inner.acoustid_success.load(Ordering::Relaxed),
                no_matches: self.inner.acoustid_no_matches.load(Ordering::Relaxed),
                network_error: self.inner.acoustid_network_error.load(Ordering::Relaxed),
                timeout: self.inner.acoustid_timeout.load(Ordering::Relaxed),
                rate_limited: self.inner.acoustid_rate_limited.load(Ordering::Relaxed),
                parse_error: self.inner.acoustid_parse_error.load(Ordering::Relaxed),
                other_error: self.inner.acoustid_other_error.load(Ordering::Relaxed),
                total_time_us: self.inner.total_acoustid_time_us.load(Ordering::Relaxed),
            },
            musicbrainz: ApiMetrics {
                success: self.inner.musicbrainz_success.load(Ordering::Relaxed),
                no_matches: self.inner.musicbrainz_not_found.load(Ordering::Relaxed),
                network_error: self.inner.musicbrainz_network_error.load(Ordering::Relaxed),
                timeout: self.inner.musicbrainz_timeout.load(Ordering::Relaxed),
                rate_limited: self.inner.musicbrainz_rate_limited.load(Ordering::Relaxed),
                parse_error: self.inner.musicbrainz_parse_error.load(Ordering::Relaxed),
                other_error: self.inner.musicbrainz_other_error.load(Ordering::Relaxed),
                total_time_us: self.inner.total_musicbrainz_time_us.load(Ordering::Relaxed),
            },
            circuit_breaker: CircuitBreakerMetrics {
                open_count: self
                    .inner
                    .circuit_breaker_open_count
                    .load(Ordering::Relaxed),
                half_open_count: self
                    .inner
                    .circuit_breaker_half_open_count
                    .load(Ordering::Relaxed),
                fast_fail_count: self
                    .inner
                    .circuit_breaker_fast_fail_count
                    .load(Ordering::Relaxed),
            },
        }
    }

    /// Reset all metrics to zero (useful for testing)
    pub fn reset(&self) {
        self.inner.fingerprint_success.store(0, Ordering::Relaxed);
        self.inner.fingerprint_not_found.store(0, Ordering::Relaxed);
        self.inner.fingerprint_corrupted.store(0, Ordering::Relaxed);
        self.inner.fingerprint_locked.store(0, Ordering::Relaxed);
        self.inner
            .fingerprint_unsupported
            .store(0, Ordering::Relaxed);
        self.inner.fingerprint_too_short.store(0, Ordering::Relaxed);
        self.inner.fingerprint_other.store(0, Ordering::Relaxed);

        self.inner.acoustid_success.store(0, Ordering::Relaxed);
        self.inner.acoustid_no_matches.store(0, Ordering::Relaxed);
        self.inner
            .acoustid_network_error
            .store(0, Ordering::Relaxed);
        self.inner.acoustid_timeout.store(0, Ordering::Relaxed);
        self.inner.acoustid_rate_limited.store(0, Ordering::Relaxed);
        self.inner.acoustid_parse_error.store(0, Ordering::Relaxed);
        self.inner.acoustid_other_error.store(0, Ordering::Relaxed);

        self.inner.musicbrainz_success.store(0, Ordering::Relaxed);
        self.inner.musicbrainz_not_found.store(0, Ordering::Relaxed);
        self.inner
            .musicbrainz_network_error
            .store(0, Ordering::Relaxed);
        self.inner.musicbrainz_timeout.store(0, Ordering::Relaxed);
        self.inner
            .musicbrainz_rate_limited
            .store(0, Ordering::Relaxed);
        self.inner
            .musicbrainz_parse_error
            .store(0, Ordering::Relaxed);
        self.inner
            .musicbrainz_other_error
            .store(0, Ordering::Relaxed);

        self.inner
            .circuit_breaker_open_count
            .store(0, Ordering::Relaxed);
        self.inner
            .circuit_breaker_half_open_count
            .store(0, Ordering::Relaxed);
        self.inner
            .circuit_breaker_fast_fail_count
            .store(0, Ordering::Relaxed);

        self.inner
            .total_fingerprint_time_us
            .store(0, Ordering::Relaxed);
        self.inner
            .total_acoustid_time_us
            .store(0, Ordering::Relaxed);
        self.inner
            .total_musicbrainz_time_us
            .store(0, Ordering::Relaxed);
    }
}

impl Default for EnrichmentMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of enrichment metrics at a point in time
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub fingerprint: FingerprintMetrics,
    pub acoustid: ApiMetrics,
    pub musicbrainz: ApiMetrics,
    pub circuit_breaker: CircuitBreakerMetrics,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FingerprintMetrics {
    pub success: u64,
    pub not_found: u64,
    pub corrupted: u64,
    pub locked: u64,
    pub unsupported: u64,
    pub too_short: u64,
    pub other: u64,
    pub total_time_us: u64,
}

impl FingerprintMetrics {
    pub fn total_attempts(&self) -> u64 {
        self.success
            + self.not_found
            + self.corrupted
            + self.locked
            + self.unsupported
            + self.too_short
            + self.other
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_attempts();
        if total == 0 {
            0.0
        } else {
            (self.success as f64) / (total as f64)
        }
    }

    pub fn avg_duration_ms(&self) -> f64 {
        if self.success == 0 {
            0.0
        } else {
            (self.total_time_us as f64) / (self.success as f64) / 1000.0
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiMetrics {
    pub success: u64,
    pub no_matches: u64,
    pub network_error: u64,
    pub timeout: u64,
    pub rate_limited: u64,
    pub parse_error: u64,
    pub other_error: u64,
    pub total_time_us: u64,
}

impl ApiMetrics {
    pub fn total_attempts(&self) -> u64 {
        self.success
            + self.no_matches
            + self.network_error
            + self.timeout
            + self.rate_limited
            + self.parse_error
            + self.other_error
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_attempts();
        if total == 0 {
            0.0
        } else {
            (self.success as f64) / (total as f64)
        }
    }

    pub fn avg_duration_ms(&self) -> f64 {
        if self.success == 0 {
            0.0
        } else {
            (self.total_time_us as f64) / (self.success as f64) / 1000.0
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CircuitBreakerMetrics {
    pub open_count: u64,
    pub half_open_count: u64,
    pub fast_fail_count: u64,
}

// Helper functions for error classification

fn classify_fingerprint_error(error: &EnrichmentError) -> &'static str {
    match error {
        EnrichmentError::FingerprintError(msg) => {
            if msg.contains("not found") || msg.contains("No such file") {
                "not_found"
            } else if msg.contains("corrupted") || msg.contains("invalid") {
                "corrupted"
            } else if msg.contains("locked") || msg.contains("in use") {
                "locked"
            } else if msg.contains("unsupported") || msg.contains("format") {
                "unsupported_format"
            } else if msg.contains("too short") || msg.contains("duration") {
                "too_short"
            } else {
                "other"
            }
        }
        _ => "other",
    }
}

fn classify_acoustid_error(error: &EnrichmentError) -> &'static str {
    match error {
        EnrichmentError::NoMatches => "no_matches",
        EnrichmentError::Network(_) => "network_error",
        EnrichmentError::RateLimited => "rate_limited",
        EnrichmentError::Parse(_) | EnrichmentError::InvalidResponse(_) => "parse_error",
        EnrichmentError::ApiError(msg) if msg.contains("timeout") => "timeout",
        _ => "other",
    }
}

fn classify_musicbrainz_error(error: &EnrichmentError) -> &'static str {
    match error {
        EnrichmentError::NoMatches => "not_found",
        EnrichmentError::Network(_) => "network_error",
        EnrichmentError::RateLimited => "rate_limited",
        EnrichmentError::Parse(_) | EnrichmentError::InvalidResponse(_) => "parse_error",
        EnrichmentError::ApiError(msg) if msg.contains("timeout") => "timeout",
        _ => "other",
    }
}

/// Timer helper for measuring operation duration
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_metrics() {
        let metrics = EnrichmentMetrics::new();

        metrics.record_fingerprint_success(Duration::from_millis(100));
        metrics.record_fingerprint_success(Duration::from_millis(200));
        metrics.record_fingerprint_error(&EnrichmentError::FingerprintError(
            "file not found".to_string(),
        ));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.fingerprint.success, 2);
        assert_eq!(snapshot.fingerprint.not_found, 1);
        assert_eq!(snapshot.fingerprint.total_attempts(), 3);
        assert!((snapshot.fingerprint.success_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_acoustid_metrics() {
        let metrics = EnrichmentMetrics::new();

        metrics.record_acoustid_success(Duration::from_millis(500));
        metrics.record_acoustid_error(&EnrichmentError::NoMatches);
        metrics.record_acoustid_error(&EnrichmentError::RateLimited);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.acoustid.success, 1);
        assert_eq!(snapshot.acoustid.no_matches, 1);
        assert_eq!(snapshot.acoustid.rate_limited, 1);
        assert_eq!(snapshot.acoustid.total_attempts(), 3);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = EnrichmentMetrics::new();

        metrics.record_fingerprint_success(Duration::from_millis(100));
        metrics.record_acoustid_success(Duration::from_millis(200));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.fingerprint.success, 1);
        assert_eq!(snapshot.acoustid.success, 1);

        metrics.reset();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.fingerprint.success, 0);
        assert_eq!(snapshot.acoustid.success, 0);
    }

    #[test]
    fn test_error_classification() {
        assert_eq!(
            classify_fingerprint_error(&EnrichmentError::FingerprintError(
                "file not found".to_string()
            )),
            "not_found"
        );

        assert_eq!(
            classify_acoustid_error(&EnrichmentError::RateLimited),
            "rate_limited"
        );

        assert_eq!(
            classify_musicbrainz_error(&EnrichmentError::NoMatches),
            "not_found"
        );
    }

    #[test]
    fn test_avg_duration() {
        let metrics = EnrichmentMetrics::new();

        metrics.record_fingerprint_success(Duration::from_millis(100));
        metrics.record_fingerprint_success(Duration::from_millis(300));

        let snapshot = metrics.snapshot();
        assert!((snapshot.fingerprint.avg_duration_ms() - 200.0).abs() < 1.0);
    }
}
