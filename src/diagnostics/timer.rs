//! Timer Resolution Diagnostics
//!
//! Windows has a system timer that by default ticks every 15.625ms (64 Hz).
//! For low-latency audio, we need higher resolution (1ms or better).
//!
//! Applications can request higher resolution via timeBeginPeriod(), and
//! the system uses the highest requested resolution.

use super::{CheckStatus, DiagnosticCheck};

#[cfg(windows)]
use std::mem::MaybeUninit;

/// Timer resolution information
#[derive(Debug, Clone)]
pub struct TimerInfo {
    /// Best (smallest) supported timer resolution in microseconds (~500us)
    pub best_resolution_us: u32,
    /// Worst (largest) supported timer resolution in microseconds (~15625us)  
    pub worst_resolution_us: u32,
    /// Current timer resolution in microseconds
    pub current_resolution_us: u32,
}

impl TimerInfo {
    /// Query the system timer resolution
    #[cfg(windows)]
    pub fn query() -> Option<Self> {
        // NtQueryTimerResolution is undocumented but stable
        // Returns values in 100-nanosecond units
        // NOTE: Windows naming is confusing:
        // - "MinimumResolution" = largest interval = worst resolution (~15.6ms)
        // - "MaximumResolution" = smallest interval = best resolution (~0.5ms)

        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn NtQueryTimerResolution(
                MinimumResolution: *mut u32,
                MaximumResolution: *mut u32,
                CurrentResolution: *mut u32,
            ) -> i32;
        }

        unsafe {
            let mut worst_res = MaybeUninit::uninit(); // "Minimum" = worst
            let mut best_res = MaybeUninit::uninit(); // "Maximum" = best
            let mut cur_res = MaybeUninit::uninit();

            let status = NtQueryTimerResolution(
                worst_res.as_mut_ptr(),
                best_res.as_mut_ptr(),
                cur_res.as_mut_ptr(),
            );

            if status == 0 {
                // Convert from 100ns units to microseconds
                Some(TimerInfo {
                    best_resolution_us: best_res.assume_init() / 10,
                    worst_resolution_us: worst_res.assume_init() / 10,
                    current_resolution_us: cur_res.assume_init() / 10,
                })
            } else {
                None
            }
        }
    }

    #[cfg(not(windows))]
    pub fn query() -> Option<Self> {
        // On non-Windows, we can't query this
        None
    }

    /// Convert to a diagnostic check
    pub fn to_check(&self) -> DiagnosticCheck {
        let current_ms = self.current_resolution_us as f64 / 1000.0;
        let best_ms = self.best_resolution_us as f64 / 1000.0;

        let (status, recommendation) = if current_ms <= 1.0 {
            (CheckStatus::Pass, None)
        } else if current_ms <= 2.0 {
            (CheckStatus::Warning, Some(
                "Timer resolution is slightly high. Some application may need to request higher resolution.".to_string()
            ))
        } else {
            (
                CheckStatus::Fail,
                Some(format!(
                    "Timer resolution is {:.1}ms (default). For audio work, {:.1}ms is available. \
                 Audio applications should request higher resolution automatically.",
                    current_ms, best_ms
                )),
            )
        };

        DiagnosticCheck {
            name: "Timer Resolution".to_string(),
            category: "System".to_string(),
            status,
            value: format!(
                "{:.2}ms (best: {:.2}ms, worst: {:.2}ms)",
                current_ms,
                best_ms,
                self.worst_resolution_us as f64 / 1000.0
            ),
            recommendation,
        }
    }
}

/// Request high timer resolution for the duration of this guard
#[cfg(windows)]
pub struct HighResolutionTimer {
    _private: (),
}

#[cfg(windows)]
impl HighResolutionTimer {
    /// Request 1ms timer resolution
    pub fn request() -> Option<Self> {
        #[link(name = "winmm")]
        unsafe extern "system" {
            fn timeBeginPeriod(uPeriod: u32) -> u32;
        }

        unsafe {
            if timeBeginPeriod(1) == 0 {
                Some(HighResolutionTimer { _private: () })
            } else {
                None
            }
        }
    }
}

#[cfg(windows)]
impl Drop for HighResolutionTimer {
    fn drop(&mut self) {
        #[link(name = "winmm")]
        unsafe extern "system" {
            fn timeEndPeriod(uPeriod: u32) -> u32;
        }

        unsafe {
            timeEndPeriod(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_query_timer_resolution() {
        let info = TimerInfo::query();
        assert!(info.is_some());

        let info = info.unwrap();
        // Best resolution should be around 500us (0.5ms)
        assert!(info.best_resolution_us > 0);
        assert!(info.best_resolution_us < 2000); // Less than 2ms

        // Worst resolution is typically 15.625ms
        assert!(info.worst_resolution_us >= 10000); // At least 10ms

        // Current should be between best and worst
        assert!(info.current_resolution_us >= info.best_resolution_us);
        assert!(info.current_resolution_us <= info.worst_resolution_us);

        println!(
            "Timer: current={:.2}ms, best={:.2}ms, worst={:.2}ms",
            info.current_resolution_us as f64 / 1000.0,
            info.best_resolution_us as f64 / 1000.0,
            info.worst_resolution_us as f64 / 1000.0
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_high_resolution_timer() {
        let before = TimerInfo::query().unwrap();

        {
            let _guard = HighResolutionTimer::request();
            let during = TimerInfo::query().unwrap();
            // Resolution should be at or near best
            assert!(during.current_resolution_us <= before.current_resolution_us);
        }

        // Note: Resolution may not immediately return to previous value
        // as other processes may have requested high resolution
    }
}
