//! Pre-flight health checks for external dependencies
//!
//! Validates that required tools and services are available before
//! starting operations like batch enrichment. Provides actionable
//! guidance when issues are detected.
//!
//! # Architecture
//! - Async health checks with timeout protection
//! - 5-minute TTL caching to avoid repeated checks
//! - Clear status categories for UI display
//! - Detailed error messages with remediation steps

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::enrichment::fingerprint;

// ============================================================================
// Health Status Types
// ============================================================================

/// Overall health status for external dependencies
#[derive(Debug, Clone)]
pub struct HealthCheckReport {
    /// When this report was generated
    #[allow(dead_code)] // Used internally for cache invalidation
    pub timestamp: Instant,

    /// Chromaprint/fpcalc availability
    pub fpcalc_status: DependencyStatus,

    /// AcoustID API availability
    pub acoustid_status: ApiStatus,

    /// MusicBrainz API availability
    pub musicbrainz_status: ApiStatus,

    /// Network connectivity
    pub network_status: NetworkStatus,

    /// API key configuration
    pub api_key_configured: bool,
}

impl HealthCheckReport {
    /// Check if the report is stale (older than TTL)
    pub fn is_stale(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }

    /// Check if all systems are operational
    pub fn is_healthy(&self) -> bool {
        self.fpcalc_status.is_available()
            && self.acoustid_status.is_available()
            && self.musicbrainz_status.is_available()
            && self.network_status.is_connected()
    }

    /// Get list of issues preventing enrichment
    pub fn blocking_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if !self.fpcalc_status.is_available() {
            issues.push(format!("fpcalc: {}", self.fpcalc_status.message()));
        }

        if !self.api_key_configured {
            issues.push("AcoustID API key not configured".to_string());
        }

        if !self.network_status.is_connected() {
            issues.push(format!("Network: {}", self.network_status.message()));
        }

        if !self.acoustid_status.is_available() {
            issues.push(format!("AcoustID API: {}", self.acoustid_status.message()));
        }

        if !self.musicbrainz_status.is_available() {
            issues.push(format!(
                "MusicBrainz API: {}",
                self.musicbrainz_status.message()
            ));
        }

        issues
    }
}

/// Status of a local dependency (like fpcalc)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyStatus {
    Available { version: String },
    NotFound,
    Error { message: String },
}

impl DependencyStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, DependencyStatus::Available { .. })
    }

    pub fn message(&self) -> &str {
        match self {
            DependencyStatus::Available { version } => version,
            DependencyStatus::NotFound => "Not installed",
            DependencyStatus::Error { message } => message,
        }
    }
}

/// Status of an external API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiStatus {
    Available,
    Unreachable,
    Timeout,
    Error { message: String },
    NotChecked,
}

impl ApiStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, ApiStatus::Available)
    }

    pub fn message(&self) -> &str {
        match self {
            ApiStatus::Available => "Available",
            ApiStatus::Unreachable => "Cannot reach API (check internet connection)",
            ApiStatus::Timeout => "Request timed out (slow network or service issues)",
            ApiStatus::Error { message } => message,
            ApiStatus::NotChecked => "Not checked",
        }
    }
}

/// Network connectivity status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkStatus {
    Connected,
    Disconnected,
    Error { message: String },
}

impl NetworkStatus {
    pub fn is_connected(&self) -> bool {
        matches!(self, NetworkStatus::Connected)
    }

    pub fn message(&self) -> &str {
        match self {
            NetworkStatus::Connected => "Connected",
            NetworkStatus::Disconnected => "No internet connection",
            NetworkStatus::Error { message } => message,
        }
    }
}

// ============================================================================
// Health Check Runner
// ============================================================================

/// Health check service with caching
pub struct HealthChecker {
    /// Cached report (None if never checked)
    cache: Arc<RwLock<Option<HealthCheckReport>>>,

    /// How long to cache results before re-checking
    cache_ttl: Duration,
}

impl HealthChecker {
    /// Create a new health checker with 5-minute cache TTL
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a health checker with custom cache TTL (for testing)
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            cache_ttl: ttl,
        }
    }

    /// Run health checks (uses cache if fresh)
    pub async fn check(&self, api_key: Option<&str>) -> HealthCheckReport {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(report) = cache.as_ref()
                && !report.is_stale(self.cache_ttl)
            {
                tracing::debug!("Using cached health check report");
                return report.clone();
            }
        }

        // Cache is stale or missing, run new checks
        tracing::info!("Running fresh health checks");
        let report = Self::run_checks(api_key).await;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            *cache = Some(report.clone());
        }

        report
    }

    /// Force a fresh check, bypassing cache
    pub async fn check_fresh(&self, api_key: Option<&str>) -> HealthCheckReport {
        tracing::info!("Running forced health checks (bypassing cache)");
        let report = Self::run_checks(api_key).await;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            *cache = Some(report.clone());
        }

        report
    }

    /// Clear cached results
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
    }

    /// Run all health checks (no caching)
    async fn run_checks(api_key: Option<&str>) -> HealthCheckReport {
        // Run checks in parallel for speed
        let (fpcalc_status, network_status, acoustid_status, musicbrainz_status) = tokio::join!(
            Self::check_fpcalc(),
            Self::check_network(),
            Self::check_acoustid_api(api_key),
            Self::check_musicbrainz_api(),
        );

        HealthCheckReport {
            timestamp: Instant::now(),
            fpcalc_status,
            acoustid_status,
            musicbrainz_status,
            network_status,
            api_key_configured: api_key.is_some(),
        }
    }

    /// Check if fpcalc is available
    async fn check_fpcalc() -> DependencyStatus {
        // Run in blocking thread to avoid blocking async runtime
        tokio::task::spawn_blocking(|| {
            if !fingerprint::is_fpcalc_available() {
                return DependencyStatus::NotFound;
            }

            match fingerprint::get_fpcalc_version() {
                Some(version) => DependencyStatus::Available { version },
                None => DependencyStatus::Error {
                    message: "Found but cannot determine version".to_string(),
                },
            }
        })
        .await
        .unwrap_or_else(|e| DependencyStatus::Error {
            message: format!("Check failed: {}", e),
        })
    }

    /// Check network connectivity (simple DNS resolution)
    async fn check_network() -> NetworkStatus {
        match tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::lookup_host("api.acoustid.org:443"),
        )
        .await
        {
            Ok(Ok(_)) => NetworkStatus::Connected,
            Ok(Err(e)) => NetworkStatus::Error {
                message: format!("DNS resolution failed: {}", e),
            },
            Err(_) => NetworkStatus::Disconnected,
        }
    }

    /// Check AcoustID API availability
    async fn check_acoustid_api(api_key: Option<&str>) -> ApiStatus {
        // If no API key, we can't test the API
        let Some(_key) = api_key else {
            return ApiStatus::NotChecked;
        };

        // Simple HEAD request to check if API is reachable
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return ApiStatus::Error {
                    message: format!("Failed to create HTTP client: {}", e),
                };
            }
        };

        // Use the public API docs page as a liveness check (doesn't require auth)
        match client.get("https://api.acoustid.org/").send().await {
            Ok(resp) if resp.status().is_success() => ApiStatus::Available,
            Ok(resp) => ApiStatus::Error {
                message: format!("API returned status {}", resp.status()),
            },
            Err(e) if e.is_timeout() => ApiStatus::Timeout,
            Err(e) if e.is_connect() => ApiStatus::Unreachable,
            Err(e) => ApiStatus::Error {
                message: format!("Request failed: {}", e),
            },
        }
    }

    /// Check MusicBrainz API availability
    async fn check_musicbrainz_api() -> ApiStatus {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("MusicMinder/0.1 (mailto:dev@example.com)")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return ApiStatus::Error {
                    message: format!("Failed to create HTTP client: {}", e),
                };
            }
        };

        // Simple GET to check if API is reachable
        match client.get("https://musicbrainz.org/ws/2/").send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_client_error() => {
                // 4xx is also OK - means API is up, we just didn't provide valid query
                ApiStatus::Available
            }
            Ok(resp) => ApiStatus::Error {
                message: format!("API returned status {}", resp.status()),
            },
            Err(e) if e.is_timeout() => ApiStatus::Timeout,
            Err(e) if e.is_connect() => ApiStatus::Unreachable,
            Err(e) => ApiStatus::Error {
                message: format!("Request failed: {}", e),
            },
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get actionable guidance for a health check issue
pub fn remediation_for_issue(issue: &str) -> Option<&'static str> {
    if issue.contains("fpcalc") && issue.contains("Not installed") {
        return Some(
            "Install Chromaprint:\n\
             • Windows: winget install AcoustID.Chromaprint\n\
             • macOS: brew install chromaprint\n\
             • Linux: apt install libchromaprint-tools",
        );
    }

    if issue.contains("API key not configured") {
        return Some(
            "Get a free AcoustID API key:\n\
             1. Visit https://acoustid.org/register\n\
             2. Register an account\n\
             3. Create an application to get your API key\n\
             4. Enter the key in Settings → Enrichment",
        );
    }

    if issue.contains("Network") {
        return Some("Check your internet connection and firewall settings");
    }

    if issue.contains("API") && issue.contains("Unreachable") {
        return Some(
            "The API service may be down. Try again later or check https://status.acoustid.org/",
        );
    }

    if issue.contains("Timeout") {
        return Some(
            "Your network may be slow or the service is experiencing high load. Try again.",
        );
    }

    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_status() {
        let available = DependencyStatus::Available {
            version: "1.5.0".to_string(),
        };
        assert!(available.is_available());
        assert_eq!(available.message(), "1.5.0");

        let not_found = DependencyStatus::NotFound;
        assert!(!not_found.is_available());
        assert_eq!(not_found.message(), "Not installed");
    }

    #[test]
    fn test_api_status() {
        assert!(ApiStatus::Available.is_available());
        assert!(!ApiStatus::Unreachable.is_available());
        assert_eq!(
            ApiStatus::Timeout.message(),
            "Request timed out (slow network or service issues)"
        );
    }

    #[test]
    fn test_network_status() {
        assert!(NetworkStatus::Connected.is_connected());
        assert!(!NetworkStatus::Disconnected.is_connected());
    }

    #[test]
    fn test_report_is_healthy() {
        let report = HealthCheckReport {
            timestamp: Instant::now(),
            fpcalc_status: DependencyStatus::Available {
                version: "1.5.0".to_string(),
            },
            acoustid_status: ApiStatus::Available,
            musicbrainz_status: ApiStatus::Available,
            network_status: NetworkStatus::Connected,
            api_key_configured: true,
        };

        assert!(report.is_healthy());
        assert!(report.blocking_issues().is_empty());
    }

    #[test]
    fn test_report_with_issues() {
        let report = HealthCheckReport {
            timestamp: Instant::now(),
            fpcalc_status: DependencyStatus::NotFound,
            acoustid_status: ApiStatus::Available,
            musicbrainz_status: ApiStatus::Available,
            network_status: NetworkStatus::Connected,
            api_key_configured: false,
        };

        assert!(!report.is_healthy());
        let issues = report.blocking_issues();
        assert_eq!(issues.len(), 2);
        assert!(issues[0].contains("fpcalc"));
        assert!(issues[1].contains("API key"));
    }

    #[test]
    fn test_report_staleness() {
        let mut report = HealthCheckReport {
            timestamp: Instant::now(),
            fpcalc_status: DependencyStatus::Available {
                version: "1.5.0".to_string(),
            },
            acoustid_status: ApiStatus::Available,
            musicbrainz_status: ApiStatus::Available,
            network_status: NetworkStatus::Connected,
            api_key_configured: true,
        };

        // Fresh report
        assert!(!report.is_stale(Duration::from_secs(300)));

        // Make it old
        report.timestamp = Instant::now() - Duration::from_secs(400);
        assert!(report.is_stale(Duration::from_secs(300)));
    }

    #[test]
    fn test_remediation_for_fpcalc() {
        let advice = remediation_for_issue("fpcalc: Not installed");
        assert!(advice.is_some());
        assert!(advice.unwrap().contains("winget install"));
    }

    #[test]
    fn test_remediation_for_api_key() {
        let advice = remediation_for_issue("AcoustID API key not configured");
        assert!(advice.is_some());
        assert!(advice.unwrap().contains("acoustid.org/register"));
    }

    #[tokio::test]
    async fn test_health_checker_caching() {
        let checker = HealthChecker::with_ttl(Duration::from_secs(1));

        // First check
        let report1 = checker.check(Some("test-key")).await;
        let timestamp1 = report1.timestamp;

        // Immediate second check should use cache
        let report2 = checker.check(Some("test-key")).await;
        assert_eq!(report2.timestamp, timestamp1);

        // Wait for cache to expire
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Third check should be fresh
        let report3 = checker.check(Some("test-key")).await;
        assert!(report3.timestamp > timestamp1);
    }

    #[tokio::test]
    async fn test_health_checker_force_refresh() {
        let checker = HealthChecker::new();

        // First check
        let report1 = checker.check(Some("test-key")).await;
        let timestamp1 = report1.timestamp;

        // Force refresh should bypass cache
        let report2 = checker.check_fresh(Some("test-key")).await;
        assert!(report2.timestamp > timestamp1);
    }
}
