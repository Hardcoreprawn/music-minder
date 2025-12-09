//! System Diagnostics Module
//!
//! Provides LatencyMon-style system analysis to identify hardware and driver
//! issues that could affect audio playback quality.
//!
//! ## Metrics Collected
//! - Timer resolution (system interrupt frequency)
//! - CPU information and frequency
//! - Memory availability and pressure
//! - Power plan (affects CPU throttling)
//! - Audio device enumeration
//! - Interrupt latency estimation
//!
//! ## Architecture Note
//! True DPC/ISR latency measurement requires kernel-mode access (ETW tracing
//! or a custom driver). This module provides user-mode approximations and
//! system configuration checks that correlate with audio performance.

mod audio;
mod cpu;
mod memory;
mod power;
mod report;
mod timer;

pub use audio::*;
pub use cpu::*;
pub use memory::*;
pub use power::*;
pub use timer::*;

/// Overall system readiness rating for audio work
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioReadiness {
    /// System is well-configured for low-latency audio
    Excellent,
    /// System should handle audio fine
    Good,
    /// Some issues detected, may have occasional glitches
    Fair,
    /// Significant issues detected, likely to have audio problems
    Poor,
}

impl AudioReadiness {
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioReadiness::Excellent => "Excellent",
            AudioReadiness::Good => "Good",
            AudioReadiness::Fair => "Fair",
            AudioReadiness::Poor => "Poor",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            AudioReadiness::Excellent => "🟢",
            AudioReadiness::Good => "🟡",
            AudioReadiness::Fair => "🟠",
            AudioReadiness::Poor => "🔴",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AudioReadiness::Excellent => "System is optimally configured for low-latency audio",
            AudioReadiness::Good => "System should handle audio playback without issues",
            AudioReadiness::Fair => {
                "Some configuration issues detected; occasional glitches possible"
            }
            AudioReadiness::Poor => "Significant issues detected; audio problems likely",
        }
    }
}

/// A single diagnostic check result
#[derive(Debug, Clone)]
pub struct DiagnosticCheck {
    pub name: String,
    pub category: String,
    pub status: CheckStatus,
    pub value: String,
    pub recommendation: Option<String>,
}

/// Status of a diagnostic check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
    Info,
}

impl CheckStatus {
    pub fn emoji(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "✓",
            CheckStatus::Warning => "⚠",
            CheckStatus::Fail => "✗",
            CheckStatus::Info => "ℹ",
        }
    }
}

/// Complete system diagnostic report
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub overall_rating: AudioReadiness,
    pub checks: Vec<DiagnosticCheck>,
    pub timer_info: Option<TimerInfo>,
    pub cpu_info: Option<CpuInfo>,
    pub memory_info: Option<MemoryInfo>,
    pub power_info: Option<PowerInfo>,
    pub audio_devices: Vec<AudioDeviceInfo>,
}

impl Default for DiagnosticReport {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            overall_rating: AudioReadiness::Fair,
            checks: vec![DiagnosticCheck {
                name: "Diagnostics".to_string(),
                category: "System".to_string(),
                status: CheckStatus::Warning,
                value: "Failed to generate report".to_string(),
                recommendation: Some("Try running diagnostics again".to_string()),
            }],
            timer_info: None,
            cpu_info: None,
            memory_info: None,
            power_info: None,
            audio_devices: vec![],
        }
    }
}

impl DiagnosticReport {
    /// Run all diagnostics and generate a report
    pub fn generate() -> Self {
        let mut checks = Vec::new();

        // Timer resolution
        let timer_info = TimerInfo::query();
        if let Some(ref info) = timer_info {
            checks.push(info.to_check());
        }

        // CPU info
        let cpu_info = CpuInfo::query();
        if let Some(ref info) = cpu_info {
            checks.extend(info.to_checks());
        }

        // Memory info
        let memory_info = MemoryInfo::query();
        if let Some(ref info) = memory_info {
            checks.extend(info.to_checks());
        }

        // Power plan
        let power_info = PowerInfo::query();
        if let Some(ref info) = power_info {
            checks.push(info.to_check());
        }

        // Audio devices
        let audio_devices = AudioDeviceInfo::enumerate();
        if !audio_devices.is_empty() {
            checks.push(DiagnosticCheck {
                name: "Audio Devices".to_string(),
                category: "Audio".to_string(),
                status: CheckStatus::Info,
                value: format!("{} device(s) found", audio_devices.len()),
                recommendation: None,
            });
        }

        // Calculate overall rating
        let overall_rating = Self::calculate_rating(&checks);

        DiagnosticReport {
            timestamp: chrono::Utc::now(),
            overall_rating,
            checks,
            timer_info,
            cpu_info,
            memory_info,
            power_info,
            audio_devices,
        }
    }

    fn calculate_rating(checks: &[DiagnosticCheck]) -> AudioReadiness {
        let fail_count = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        let warning_count = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warning)
            .count();

        if fail_count >= 2 {
            AudioReadiness::Poor
        } else if fail_count == 1 || warning_count >= 3 {
            AudioReadiness::Fair
        } else if warning_count >= 1 {
            AudioReadiness::Good
        } else {
            AudioReadiness::Excellent
        }
    }

    /// Get only checks with issues (warnings or failures)
    pub fn issues(&self) -> Vec<&DiagnosticCheck> {
        self.checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Warning | CheckStatus::Fail))
            .collect()
    }
}
