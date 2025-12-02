//! Audio fingerprint generation using Chromaprint/fpcalc
//!
//! This module shells out to the `fpcalc` command-line tool (part of Chromaprint)
//! to generate audio fingerprints. This approach is more reliable than Rust bindings
//! and works on all platforms where fpcalc is installed.
//!
//! Install fpcalc:
//! - Windows: `winget install AcoustID.Chromaprint` or download from https://acoustid.org/chromaprint
//! - macOS: `brew install chromaprint`
//! - Linux: `apt install libchromaprint-tools` or equivalent

use std::path::Path;
use std::process::Command;

use crate::enrichment::domain::{AudioFingerprint, EnrichmentError};

/// Common installation paths for fpcalc on Windows
#[cfg(windows)]
const FPCALC_PATHS: &[&str] = &[
    "fpcalc", // In PATH
    r"C:\Program Files\Chromaprint\fpcalc.exe",
    r"C:\Program Files\MusicBrainz Picard\fpcalc.exe",
    r"C:\Program Files (x86)\Chromaprint\fpcalc.exe",
    r"C:\Program Files (x86)\MusicBrainz Picard\fpcalc.exe",
];

#[cfg(not(windows))]
const FPCALC_PATHS: &[&str] = &[
    "fpcalc", // In PATH
    "/usr/bin/fpcalc",
    "/usr/local/bin/fpcalc",
    "/opt/homebrew/bin/fpcalc",
];

/// Find the fpcalc executable, checking common installation paths
fn find_fpcalc() -> Option<&'static str> {
    FPCALC_PATHS
        .iter()
        .find(|&path| {
            Command::new(path)
                .arg("-version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|v| v as _)
}

/// Generate an audio fingerprint for the given file
pub fn generate_fingerprint(path: &Path) -> Result<AudioFingerprint, EnrichmentError> {
    let fpcalc = find_fpcalc().ok_or_else(|| {
        EnrichmentError::FingerprintError(
            "fpcalc not found. Please install Chromaprint: https://acoustid.org/chromaprint"
                .to_string(),
        )
    })?;

    let output = Command::new(fpcalc)
        .arg("-json")
        .arg(path)
        .output()
        .map_err(|e| EnrichmentError::FingerprintError(format!("Failed to run fpcalc: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EnrichmentError::FingerprintError(format!(
            "fpcalc failed: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_fpcalc_json(&stdout)
}

/// Parse the JSON output from fpcalc
fn parse_fpcalc_json(json: &str) -> Result<AudioFingerprint, EnrichmentError> {
    let parsed: FpcalcOutput = serde_json::from_str(json).map_err(|e| {
        EnrichmentError::FingerprintError(format!("Failed to parse fpcalc output: {}", e))
    })?;

    Ok(AudioFingerprint {
        fingerprint: parsed.fingerprint,
        duration_secs: parsed.duration.round() as u32,
    })
}

/// fpcalc JSON output structure
#[derive(serde::Deserialize)]
struct FpcalcOutput {
    fingerprint: String,
    duration: f64,
}

/// Check if fpcalc is available on the system
pub fn is_fpcalc_available() -> bool {
    find_fpcalc().is_some()
}

/// Get fpcalc version string (for diagnostics)
pub fn get_fpcalc_version() -> Option<String> {
    let fpcalc = find_fpcalc()?;
    Command::new(fpcalc)
        .arg("-version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fpcalc_json() {
        let json = r#"{"duration": 180.5, "fingerprint": "AQADtNIyRUkkZUqS"}"#;

        let result = parse_fpcalc_json(json).unwrap();

        assert_eq!(result.fingerprint, "AQADtNIyRUkkZUqS");
        assert_eq!(result.duration_secs, 181); // Rounded
    }

    #[test]
    fn test_parse_fpcalc_json_error() {
        let json = r#"{"error": "invalid"}"#;

        let result = parse_fpcalc_json(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_is_fpcalc_available() {
        // This test just ensures the function doesn't panic
        let _ = is_fpcalc_available();
    }

    #[test]
    fn test_fingerprint_nonexistent_file() {
        let result = generate_fingerprint(Path::new("/nonexistent/file.mp3"));

        // Should fail (either fpcalc not found or file not found)
        assert!(result.is_err());
    }
}
