//! Report Generation
//!
//! Formats diagnostic reports for display.

use super::*;

impl DiagnosticReport {
    /// Print the report to stdout
    pub fn print(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║            MUSIC MINDER - SYSTEM DIAGNOSTICS                 ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        
        // Overall rating
        println!("Overall Rating: {} {}", 
            self.overall_rating.emoji(), 
            self.overall_rating.as_str());
        println!("{}", self.overall_rating.description());
        println!();
        
        // Group checks by category
        let mut categories: std::collections::HashMap<&str, Vec<&DiagnosticCheck>> = 
            std::collections::HashMap::new();
        
        for check in &self.checks {
            categories.entry(&check.category).or_default().push(check);
        }
        
        // Print each category
        for category in ["System", "CPU", "Memory", "Audio"] {
            if let Some(checks) = categories.get(category) {
                println!("┌─ {} ─────────────────────────────────────────────────", category);
                for check in checks {
                    println!("│ {} {} : {}", 
                        check.status.emoji(), 
                        check.name, 
                        check.value);
                    if let Some(ref rec) = check.recommendation {
                        for line in rec.lines() {
                            println!("│     └─ {}", line);
                        }
                    }
                }
                println!("└────────────────────────────────────────────────────────────");
                println!();
            }
        }
        
        // Audio devices detail
        if !self.audio_devices.is_empty() {
            println!("┌─ Audio Devices ────────────────────────────────────────────");
            for device in &self.audio_devices {
                let type_str = match device.device_type {
                    AudioDeviceType::Output => "OUT",
                    AudioDeviceType::Input => "IN ",
                };
                let default_str = if device.is_default { " ★" } else { "" };
                println!("│ [{}]{} {}", type_str, default_str, device.name);
                
                if !device.sample_rates.is_empty() {
                    let rates: Vec<String> = device.sample_rates.iter()
                        .map(|r| format!("{}kHz", r / 1000))
                        .collect();
                    println!("│       Rates: {}", rates.join(", "));
                }
            }
            println!("└────────────────────────────────────────────────────────────");
            println!();
        }
        
        // Summary of issues
        let issues = self.issues();
        if !issues.is_empty() {
            println!("┌─ Issues Found ─────────────────────────────────────────────");
            for issue in issues {
                let emoji = issue.status.emoji();
                println!("│ {} {}: {}", emoji, issue.name, issue.value);
            }
            println!("└────────────────────────────────────────────────────────────");
            println!();
        }
        
        println!("Report generated: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    
    /// Generate a compact one-line summary
    pub fn summary(&self) -> String {
        let issues = self.issues();
        if issues.is_empty() {
            format!("{} {} - No issues detected", 
                self.overall_rating.emoji(), 
                self.overall_rating.as_str())
        } else {
            format!("{} {} - {} issue(s) found", 
                self.overall_rating.emoji(), 
                self.overall_rating.as_str(),
                issues.len())
        }
    }
    
    /// Export report as JSON
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");
        
        json.push_str(&format!("  \"timestamp\": \"{}\",\n", self.timestamp.to_rfc3339()));
        json.push_str(&format!("  \"rating\": \"{}\",\n", self.overall_rating.as_str()));
        
        // Checks
        json.push_str("  \"checks\": [\n");
        for (i, check) in self.checks.iter().enumerate() {
            let status = match check.status {
                CheckStatus::Pass => "pass",
                CheckStatus::Warning => "warning",
                CheckStatus::Fail => "fail",
                CheckStatus::Info => "info",
            };
            json.push_str(&format!(
                "    {{\"name\": \"{}\", \"category\": \"{}\", \"status\": \"{}\", \"value\": \"{}\"}}",
                check.name, check.category, status, check.value.replace('"', "\\\"")
            ));
            if i < self.checks.len() - 1 {
                json.push(',');
            }
            json.push('\n');
        }
        json.push_str("  ],\n");
        
        // Audio devices
        json.push_str("  \"audio_devices\": [\n");
        for (i, device) in self.audio_devices.iter().enumerate() {
            let dev_type = match device.device_type {
                AudioDeviceType::Output => "output",
                AudioDeviceType::Input => "input",
            };
            json.push_str(&format!(
                "    {{\"name\": \"{}\", \"type\": \"{}\", \"default\": {}}}",
                device.name.replace('"', "\\\""), dev_type, device.is_default
            ));
            if i < self.audio_devices.len() - 1 {
                json.push(',');
            }
            json.push('\n');
        }
        json.push_str("  ]\n");
        
        json.push('}');
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_report_generation() {
        let report = DiagnosticReport::generate();
        
        // Should have some checks
        assert!(!report.checks.is_empty());
        
        // Print for visual inspection
        report.print();
    }
    
    #[test]
    fn test_report_json() {
        let report = DiagnosticReport::generate();
        let json = report.to_json();
        
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"rating\""));
        assert!(json.contains("\"checks\""));
        
        println!("{}", json);
    }
}
