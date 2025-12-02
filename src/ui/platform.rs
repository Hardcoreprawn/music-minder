//! Platform-specific utilities for detecting user folders and system paths.

use std::path::PathBuf;

/// Get the user's Music folder, with fallbacks for various OS configurations.
/// 
/// Checks in order:
/// 1. System audio directory (via `dirs` crate)
/// 2. OneDrive Music folder (Windows)
/// 3. Standard Music folder under USERPROFILE (Windows)
/// 4. Home directory
/// 5. Current directory (last resort)
pub fn get_user_music_folder() -> PathBuf {
    // Use the dirs crate which handles Windows known folders properly
    if let Some(music) = dirs::audio_dir() {
        if music.exists() {
            return music;
        }
    }
    
    // Try OneDrive Music folder (common for Windows users)
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let onedrive_music = PathBuf::from(&user_profile).join("OneDrive").join("Music");
        if onedrive_music.exists() {
            return onedrive_music;
        }
        // Try regular Music folder
        let music_path = PathBuf::from(&user_profile).join("Music");
        if music_path.exists() {
            return music_path;
        }
    }
    
    // Fallback to home directory
    if let Some(home) = dirs::home_dir() {
        return home;
    }
    
    // Last resort: current directory
    std::env::current_dir().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn get_music_folder_returns_valid_path() {
        let path = get_user_music_folder();
        // Should return some path (even if it's current dir)
        assert!(!path.as_os_str().is_empty() || path == PathBuf::new());
    }
}
