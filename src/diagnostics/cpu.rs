//! CPU Diagnostics
//!
//! Measures CPU characteristics relevant to audio performance:
//! - Processor count and model
//! - Current CPU usage
//! - Frequency (actual vs max, detects throttling)

use super::{DiagnosticCheck, CheckStatus};

#[cfg(windows)]
use std::mem::MaybeUninit;

/// CPU information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// Processor name/model
    pub name: String,
    /// Number of logical processors
    pub logical_cores: u32,
    /// Number of physical cores
    pub physical_cores: u32,
    /// Maximum frequency in MHz
    pub max_frequency_mhz: u32,
    /// Current frequency in MHz (approximate)
    pub current_frequency_mhz: Option<u32>,
    /// Current CPU usage percentage (0-100)
    pub usage_percent: Option<f32>,
}

impl CpuInfo {
    /// Query CPU information
    #[cfg(windows)]
    pub fn query() -> Option<Self> {
        // Get processor count
        #[repr(C)]
        struct SystemInfo {
            w_processor_architecture: u16,
            w_reserved: u16,
            dw_page_size: u32,
            lp_minimum_application_address: *mut std::ffi::c_void,
            lp_maximum_application_address: *mut std::ffi::c_void,
            dw_active_processor_mask: usize,
            dw_number_of_processors: u32,
            dw_processor_type: u32,
            dw_allocation_granularity: u32,
            w_processor_level: u16,
            w_processor_revision: u16,
        }
        
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetSystemInfo(lpSystemInfo: *mut SystemInfo);
        }
        
        let mut sys_info: MaybeUninit<SystemInfo> = MaybeUninit::uninit();
        unsafe {
            GetSystemInfo(sys_info.as_mut_ptr());
        }
        let sys_info = unsafe { sys_info.assume_init() };
        let logical_cores = sys_info.dw_number_of_processors;
        
        // Get processor name from registry
        let name = get_processor_name().unwrap_or_else(|| "Unknown Processor".to_string());
        
        // Get frequency from registry
        let max_frequency_mhz = get_processor_frequency().unwrap_or(0);
        
        // Estimate physical cores (rough heuristic)
        let physical_cores = if name.contains("Intel") && logical_cores > 1 {
            // Intel typically has HT (2 threads per core) on consumer CPUs
            // This is a rough estimate
            logical_cores / 2
        } else if name.contains("AMD") && logical_cores > 1 {
            // AMD Ryzen has SMT (2 threads per core)
            logical_cores / 2
        } else {
            logical_cores
        };
        
        Some(CpuInfo {
            name,
            logical_cores,
            physical_cores,
            max_frequency_mhz,
            current_frequency_mhz: None, // Would need more complex measurement
            usage_percent: get_cpu_usage(),
        })
    }
    
    #[cfg(not(windows))]
    pub fn query() -> Option<Self> {
        None
    }
    
    /// Convert to diagnostic checks
    pub fn to_checks(&self) -> Vec<DiagnosticCheck> {
        let mut checks = Vec::new();
        
        // CPU info check
        checks.push(DiagnosticCheck {
            name: "Processor".to_string(),
            category: "CPU".to_string(),
            status: CheckStatus::Info,
            value: format!("{} ({} cores, {} threads)", 
                self.name, self.physical_cores, self.logical_cores),
            recommendation: None,
        });
        
        // Frequency check
        if self.max_frequency_mhz > 0 {
            let (status, recommendation) = if self.max_frequency_mhz >= 2000 {
                (CheckStatus::Pass, None)
            } else if self.max_frequency_mhz >= 1000 {
                (CheckStatus::Warning, Some(
                    "CPU frequency is relatively low. May struggle with complex audio processing.".to_string()
                ))
            } else {
                (CheckStatus::Fail, Some(
                    "CPU frequency is very low. Audio glitches likely under load.".to_string()
                ))
            };
            
            checks.push(DiagnosticCheck {
                name: "CPU Frequency".to_string(),
                category: "CPU".to_string(),
                status,
                value: format!("{} MHz", self.max_frequency_mhz),
                recommendation,
            });
        }
        
        // CPU usage check
        if let Some(usage) = self.usage_percent {
            let (status, recommendation) = if usage < 50.0 {
                (CheckStatus::Pass, None)
            } else if usage < 80.0 {
                (CheckStatus::Warning, Some(
                    "CPU usage is elevated. Close unnecessary applications for best audio performance.".to_string()
                ))
            } else {
                (CheckStatus::Fail, Some(
                    "CPU usage is very high! Audio glitches likely. Close other applications.".to_string()
                ))
            };
            
            checks.push(DiagnosticCheck {
                name: "CPU Usage".to_string(),
                category: "CPU".to_string(),
                status,
                value: format!("{:.1}%", usage),
                recommendation,
            });
        }
        
        // Core count check
        let (status, recommendation) = if self.logical_cores >= 4 {
            (CheckStatus::Pass, None)
        } else if self.logical_cores >= 2 {
            (CheckStatus::Warning, Some(
                "Limited CPU cores. Heavy audio processing may compete with system tasks.".to_string()
            ))
        } else {
            (CheckStatus::Fail, Some(
                "Single-core system. Audio processing will compete with all other tasks.".to_string()
            ))
        };
        
        checks.push(DiagnosticCheck {
            name: "CPU Cores".to_string(),
            category: "CPU".to_string(),
            status,
            value: format!("{} logical cores", self.logical_cores),
            recommendation,
        });
        
        checks
    }
}

#[cfg(windows)]
fn get_processor_name() -> Option<String> {
    use std::ptr::null_mut;
    
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut isize,
        ) -> i32;
        
        fn RegQueryValueExW(
            hKey: isize,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;
        
        fn RegCloseKey(hKey: isize) -> i32;
    }
    
    const HKEY_LOCAL_MACHINE: isize = -2147483646i64 as isize;
    const KEY_READ: u32 = 0x20019;
    
    let subkey: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    
    let value_name: Vec<u16> = "ProcessorNameString"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    
    unsafe {
        let mut hkey: isize = 0;
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }
        
        let mut data = vec![0u8; 256];
        let mut data_len = data.len() as u32;
        let mut data_type = 0u32;
        
        let result = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            null_mut(),
            &mut data_type,
            data.as_mut_ptr(),
            &mut data_len,
        );
        
        RegCloseKey(hkey);
        
        if result == 0 {
            // Convert from UTF-16
            let wide: Vec<u16> = data[..data_len as usize]
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .take_while(|&c| c != 0)
                .collect();
            Some(String::from_utf16_lossy(&wide).trim().to_string())
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn get_processor_frequency() -> Option<u32> {
    use std::ptr::null_mut;
    
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut isize,
        ) -> i32;
        
        fn RegQueryValueExW(
            hKey: isize,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;
        
        fn RegCloseKey(hKey: isize) -> i32;
    }
    
    const HKEY_LOCAL_MACHINE: isize = -2147483646i64 as isize;
    const KEY_READ: u32 = 0x20019;
    
    let subkey: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    
    let value_name: Vec<u16> = "~MHz"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    
    unsafe {
        let mut hkey: isize = 0;
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }
        
        let mut data = [0u8; 4];
        let mut data_len = 4u32;
        let mut data_type = 0u32;
        
        let result = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            null_mut(),
            &mut data_type,
            data.as_mut_ptr(),
            &mut data_len,
        );
        
        RegCloseKey(hkey);
        
        if result == 0 {
            Some(u32::from_le_bytes(data))
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn get_cpu_usage() -> Option<f32> {
    // This is a simplified approach using GetSystemTimes
    // For accurate usage, we'd need to sample over time
    
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimes(
            lpIdleTime: *mut u64,
            lpKernelTime: *mut u64,
            lpUserTime: *mut u64,
        ) -> i32;
    }
    
    unsafe {
        let mut idle1 = 0u64;
        let mut kernel1 = 0u64;
        let mut user1 = 0u64;
        
        if GetSystemTimes(&mut idle1, &mut kernel1, &mut user1) == 0 {
            return None;
        }
        
        // Wait a short time
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let mut idle2 = 0u64;
        let mut kernel2 = 0u64;
        let mut user2 = 0u64;
        
        if GetSystemTimes(&mut idle2, &mut kernel2, &mut user2) == 0 {
            return None;
        }
        
        let idle_delta = idle2.saturating_sub(idle1);
        let kernel_delta = kernel2.saturating_sub(kernel1);
        let user_delta = user2.saturating_sub(user1);
        
        let total = kernel_delta + user_delta;
        if total == 0 {
            return Some(0.0);
        }
        
        // Kernel time includes idle time
        let busy = total.saturating_sub(idle_delta);
        Some((busy as f64 / total as f64 * 100.0) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(windows)]
    fn test_query_cpu_info() {
        let info = CpuInfo::query();
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert!(!info.name.is_empty());
        assert!(info.logical_cores >= 1);
        assert!(info.physical_cores >= 1);
        println!("CPU: {} ({} MHz)", info.name, info.max_frequency_mhz);
    }
    
    #[test]
    #[cfg(windows)]
    fn test_get_processor_name() {
        let name = get_processor_name();
        assert!(name.is_some());
        let name = name.unwrap();
        assert!(!name.is_empty());
        println!("Processor: {}", name);
    }
}
