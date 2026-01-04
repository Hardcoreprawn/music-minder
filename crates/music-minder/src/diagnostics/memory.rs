//! Memory Diagnostics
//!
//! Checks memory availability and pressure, which can affect audio performance
//! if the system is paging heavily.

use super::{CheckStatus, DiagnosticCheck};

#[cfg(windows)]
use std::mem::MaybeUninit;

/// Memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total physical memory in bytes
    pub total_physical: u64,
    /// Available physical memory in bytes
    pub available_physical: u64,
    /// Total page file size in bytes
    pub total_page_file: u64,
    /// Available page file in bytes
    pub available_page_file: u64,
    /// Memory load percentage (0-100)
    pub memory_load: u32,
}

impl MemoryInfo {
    /// Query memory information
    #[cfg(windows)]
    pub fn query() -> Option<Self> {
        #[repr(C)]
        #[allow(clippy::upper_case_acronyms)]
        struct MEMORYSTATUSEX {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
        }

        unsafe {
            let mut status = MaybeUninit::<MEMORYSTATUSEX>::uninit();
            (*status.as_mut_ptr()).dw_length = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

            if GlobalMemoryStatusEx(status.as_mut_ptr()) != 0 {
                let status = status.assume_init();
                Some(MemoryInfo {
                    total_physical: status.ull_total_phys,
                    available_physical: status.ull_avail_phys,
                    total_page_file: status.ull_total_page_file,
                    available_page_file: status.ull_avail_page_file,
                    memory_load: status.dw_memory_load,
                })
            } else {
                None
            }
        }
    }

    #[cfg(not(windows))]
    pub fn query() -> Option<Self> {
        None
    }

    /// Get total physical memory in GB
    pub fn total_gb(&self) -> f64 {
        self.total_physical as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Get available physical memory in GB
    pub fn available_gb(&self) -> f64 {
        self.available_physical as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Convert to diagnostic checks
    pub fn to_checks(&self) -> Vec<DiagnosticCheck> {
        let mut checks = Vec::new();

        // Total RAM check
        let total_gb = self.total_gb();
        let (status, recommendation) = if total_gb >= 8.0 {
            (CheckStatus::Pass, None)
        } else if total_gb >= 4.0 {
            (
                CheckStatus::Warning,
                Some("4-8GB RAM is adequate but may be limiting for large projects.".to_string()),
            )
        } else {
            (
                CheckStatus::Fail,
                Some(
                    "Less than 4GB RAM. System will page frequently, causing audio glitches."
                        .to_string(),
                ),
            )
        };

        checks.push(DiagnosticCheck {
            name: "Total RAM".to_string(),
            category: "Memory".to_string(),
            status,
            value: format!("{:.1} GB", total_gb),
            recommendation,
        });

        // Available RAM check
        let available_gb = self.available_gb();
        let (status, recommendation) = if available_gb >= 2.0 {
            (CheckStatus::Pass, None)
        } else if available_gb >= 1.0 {
            (
                CheckStatus::Warning,
                Some("Available RAM is getting low. Close unused applications.".to_string()),
            )
        } else {
            (CheckStatus::Fail, Some(
                "Very low available RAM! System is likely paging. Close applications immediately.".to_string()
            ))
        };

        checks.push(DiagnosticCheck {
            name: "Available RAM".to_string(),
            category: "Memory".to_string(),
            status,
            value: format!(
                "{:.1} GB ({:.1}% free)",
                available_gb,
                (1.0 - self.memory_load as f64 / 100.0) * 100.0
            ),
            recommendation,
        });

        // Memory pressure check
        let (status, recommendation) = if self.memory_load < 70 {
            (CheckStatus::Pass, None)
        } else if self.memory_load < 85 {
            (
                CheckStatus::Warning,
                Some("Memory pressure is elevated. Monitor for paging activity.".to_string()),
            )
        } else {
            (
                CheckStatus::Fail,
                Some("High memory pressure! System is likely swapping to disk.".to_string()),
            )
        };

        checks.push(DiagnosticCheck {
            name: "Memory Pressure".to_string(),
            category: "Memory".to_string(),
            status,
            value: format!("{}% in use", self.memory_load),
            recommendation,
        });

        checks
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_query_memory_info() {
        let info = MemoryInfo::query();
        assert!(info.is_some());

        let info = info.unwrap();
        assert!(info.total_physical > 0);
        assert!(info.available_physical > 0);
        assert!(info.available_physical <= info.total_physical);
        assert!(info.memory_load <= 100);

        println!(
            "RAM: {:.1} GB total, {:.1} GB available ({}% used)",
            info.total_gb(),
            info.available_gb(),
            info.memory_load
        );
    }
}
