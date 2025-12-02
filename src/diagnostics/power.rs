//! Power Plan Diagnostics
//!
//! Windows power plans affect CPU frequency scaling and latency.
//! For audio work, "High Performance" or "Ultimate Performance" is recommended.

use super::{DiagnosticCheck, CheckStatus};

/// Power plan information
#[derive(Debug, Clone)]
pub struct PowerInfo {
    /// Active power scheme GUID
    pub scheme_guid: String,
    /// Friendly name of the power scheme
    pub scheme_name: String,
    /// Whether this is considered optimal for audio
    pub is_high_performance: bool,
}

impl PowerInfo {
    /// Query the active power plan
    #[cfg(windows)]
    pub fn query() -> Option<Self> {
        // Use powercfg command to get active scheme
        // This is simpler than using the PowerReadFriendlyName API
        
        let output = std::process::Command::new("powercfg")
            .args(["/getactivescheme"])
            .output()
            .ok()?;
        
        if !output.status.success() {
            return None;
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Output format: "Power Scheme GUID: <guid>  (<name>)"
        // Example: "Power Scheme GUID: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (High performance)"
        
        let line = stdout.lines().next()?;
        
        // Extract GUID
        let guid_start = line.find("GUID: ")? + 6;
        let guid_end = line[guid_start..].find(' ').map(|i| guid_start + i).unwrap_or(line.len());
        let guid = line[guid_start..guid_end].trim().to_string();
        
        // Extract name (in parentheses)
        let name = if let (Some(start), Some(end)) = (line.rfind('('), line.rfind(')')) {
            line[start + 1..end].to_string()
        } else {
            "Unknown".to_string()
        };
        
        let is_high_performance = name.to_lowercase().contains("high performance")
            || name.to_lowercase().contains("ultimate")
            || guid == "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"  // High Performance GUID
            || guid == "e9a42b02-d5df-448d-aa00-03f14749eb61"; // Ultimate Performance GUID
        
        Some(PowerInfo {
            scheme_guid: guid,
            scheme_name: name,
            is_high_performance,
        })
    }
    
    #[cfg(not(windows))]
    pub fn query() -> Option<Self> {
        None
    }
    
    /// Convert to a diagnostic check
    pub fn to_check(&self) -> DiagnosticCheck {
        let (status, recommendation) = if self.is_high_performance {
            (CheckStatus::Pass, None)
        } else if self.scheme_name.to_lowercase().contains("balanced") {
            (CheckStatus::Warning, Some(
                "Consider switching to 'High Performance' power plan for lower latency.\n\
                 Run: powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c".to_string()
            ))
        } else {
            (CheckStatus::Fail, Some(
                "Power saving mode may cause CPU throttling and audio glitches.\n\
                 Switch to 'High Performance': powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c".to_string()
            ))
        };
        
        DiagnosticCheck {
            name: "Power Plan".to_string(),
            category: "System".to_string(),
            status,
            value: self.scheme_name.clone(),
            recommendation,
        }
    }
}

/// List all available power schemes
#[cfg(windows)]
pub fn list_power_schemes() -> Vec<(String, String)> {
    let output = match std::process::Command::new("powercfg")
        .args(["/list"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    
    if !output.status.success() {
        return Vec::new();
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut schemes = Vec::new();
    
    for line in stdout.lines() {
        if line.contains("GUID:") {
            // Extract GUID and name similar to query()
            if let Some(guid_start) = line.find("GUID: ") {
                let guid_start = guid_start + 6;
                let rest = &line[guid_start..];
                
                if let Some(guid_end) = rest.find(' ') {
                    let guid = rest[..guid_end].trim().to_string();
                    
                    if let (Some(start), Some(end)) = (rest.rfind('('), rest.rfind(')')) {
                        let name = rest[start + 1..end].to_string();
                        schemes.push((guid, name));
                    }
                }
            }
        }
    }
    
    schemes
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(windows)]
    fn test_query_power_info() {
        let info = PowerInfo::query();
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert!(!info.scheme_guid.is_empty());
        assert!(!info.scheme_name.is_empty());
        
        println!("Power Plan: {} ({})", info.scheme_name, info.scheme_guid);
        println!("High Performance: {}", info.is_high_performance);
    }
    
    #[test]
    #[cfg(windows)]
    fn test_list_power_schemes() {
        let schemes = list_power_schemes();
        assert!(!schemes.is_empty());
        
        for (guid, name) in &schemes {
            println!("  {} - {}", name, guid);
        }
    }
}
