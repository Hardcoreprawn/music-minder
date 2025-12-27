//! Configuration system using TOML files.
//!
//! Config is stored in the OS-standard config directory:
//! - Windows: %APPDATA%\music-minder\config.toml
//! - macOS: ~/Library/Application Support/music-minder/config.toml
//! - Linux: ~/.config/music-minder/config.toml
//!
//! The config file is human-readable and editable. Settings are
//! loaded at startup and saved when changed through the UI.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// API credentials (keep separate for potential future encryption)
    pub credentials: Credentials,

    /// Appearance settings
    pub appearance: AppearanceConfig,

    /// Audio settings
    pub audio: AudioConfig,

    /// Library settings
    pub library: LibraryConfig,
}

/// API credentials
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Credentials {
    /// AcoustID API key for fingerprint lookups
    pub acoustid_api_key: Option<String>,
}

/// Appearance/theme settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Theme name (only "dark" supported currently)
    pub theme: String,

    /// Whether the sidebar is collapsed
    pub sidebar_collapsed: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            sidebar_collapsed: false,
        }
    }
}

/// Audio playback settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Selected output device name (empty = system default)
    pub output_device: String,

    /// Visualization mode: "spectrum", "waveform", "vu_meter", "off"
    pub visualization_mode: String,

    /// Last volume level (0.0 - 1.0)
    pub volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            output_device: String::new(),
            visualization_mode: "spectrum".to_string(),
            volume: 1.0,
        }
    }
}

/// Library management settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryConfig {
    /// Library scan paths
    pub paths: Vec<PathBuf>,

    /// Last scanned path (for quick rescan)
    pub last_scan_path: Option<PathBuf>,

    /// Whether to watch for file changes
    pub watch_for_changes: bool,

    /// Auto-queue tracks from same album when starting playback
    pub auto_queue: bool,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            last_scan_path: None,
            watch_for_changes: true,
            auto_queue: true,
        }
    }
}

// ============================================================================
// Config File Operations
// ============================================================================

/// Get the config directory path
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("music-minder"))
}

/// Get the full path to the config file
pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Load configuration from disk
///
/// Returns default config if file doesn't exist or can't be parsed.
/// Logs warnings but doesn't fail - we always return a usable config.
pub fn load() -> Config {
    let Some(path) = config_path() else {
        tracing::warn!("Could not determine config directory, using defaults");
        return Config::default();
    };

    if !path.exists() {
        tracing::info!("No config file found at {:?}, using defaults", path);
        return Config::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                tracing::info!("Loaded config from {:?}", path);
                config
            }
            Err(e) => {
                tracing::error!("Failed to parse config file {:?}: {}", path, e);
                tracing::warn!("Using default configuration");
                Config::default()
            }
        },
        Err(e) => {
            tracing::error!("Failed to read config file {:?}: {}", path, e);
            Config::default()
        }
    }
}

/// Save configuration to disk
///
/// Creates the config directory if it doesn't exist.
pub fn save(config: &Config) -> Result<(), ConfigError> {
    let dir = config_dir().ok_or(ConfigError::NoConfigDir)?;
    let path = dir.join("config.toml");

    // Ensure directory exists
    std::fs::create_dir_all(&dir).map_err(|e| ConfigError::CreateDir(dir.clone(), e))?;

    // Serialize to pretty TOML
    let contents = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;

    // Write atomically (write to temp, then rename)
    let temp_path = path.with_extension("toml.tmp");
    std::fs::write(&temp_path, &contents).map_err(|e| ConfigError::Write(temp_path.clone(), e))?;
    std::fs::rename(&temp_path, &path)
        .map_err(|e| ConfigError::Rename(temp_path, path.clone(), e))?;

    tracing::info!("Saved config to {:?}", path);
    Ok(())
}

/// Save configuration asynchronously (for use in Iced tasks)
pub async fn save_async(config: Config) -> Result<(), ConfigError> {
    // Config save is quick, but we move to blocking thread to not block UI
    tokio::task::spawn_blocking(move || save(&config))
        .await
        .map_err(|e| ConfigError::TaskJoin(e.to_string()))?
}

// ============================================================================
// Error Types
// ============================================================================

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Could not determine config directory")]
    NoConfigDir,

    #[error("Failed to create config directory {0}: {1}")]
    CreateDir(PathBuf, std::io::Error),

    #[error("Failed to serialize config: {0}")]
    Serialize(toml::ser::Error),

    #[error("Failed to write config to {0}: {1}")]
    Write(PathBuf, std::io::Error),

    #[error("Failed to rename temp file {0} to {1}: {2}")]
    Rename(PathBuf, PathBuf, std::io::Error),

    #[error("Task join error: {0}")]
    TaskJoin(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_serializes() {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(toml.contains("[credentials]"));
        assert!(toml.contains("[appearance]"));
        assert!(toml.contains("[audio]"));
        assert!(toml.contains("[library]"));
    }

    #[test]
    fn test_config_roundtrip() {
        let mut config = Config::default();
        config.credentials.acoustid_api_key = Some("test-key-123".to_string());
        config.audio.volume = 0.75;
        config.library.paths.push(PathBuf::from("/music"));

        let toml = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml).unwrap();

        assert_eq!(
            parsed.credentials.acoustid_api_key,
            Some("test-key-123".to_string())
        );
        assert_eq!(parsed.audio.volume, 0.75);
        assert_eq!(parsed.library.paths, vec![PathBuf::from("/music")]);
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        // Config with only some fields
        let toml = r#"
[credentials]
acoustid_api_key = "my-key"
"#;
        let config: Config = toml::from_str(toml).unwrap();

        // Specified field is set
        assert_eq!(
            config.credentials.acoustid_api_key,
            Some("my-key".to_string())
        );

        // Other fields use defaults
        assert_eq!(config.appearance.theme, "dark");
        assert_eq!(config.audio.volume, 1.0);
        assert!(config.library.paths.is_empty());
    }
}
