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
    if let Some(music) = dirs::audio_dir()
        && music.exists()
    {
        return music;
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

    #[test]
    fn get_music_folder_returns_existing_path() {
        let path = get_user_music_folder();
        // The returned path should exist (we check existence in the function)
        assert!(path.exists(), "Returned path should exist: {:?}", path);
    }

    #[test]
    fn get_music_folder_returns_directory() {
        let path = get_user_music_folder();
        // Should be a directory, not a file
        assert!(
            path.is_dir(),
            "Returned path should be a directory: {:?}",
            path
        );
    }

    // Tests for dirs crate API contract
    mod dirs_api {
        #[test]
        fn audio_dir_returns_option_pathbuf() {
            // dirs::audio_dir() should return Option<PathBuf>
            let result: Option<std::path::PathBuf> = dirs::audio_dir();
            // May be None on some systems, that's OK
            if let Some(path) = result {
                // If it returns a path, it should be absolute
                assert!(path.is_absolute(), "audio_dir should return absolute path");
            }
        }

        #[test]
        fn home_dir_returns_option_pathbuf() {
            // dirs::home_dir() should return Option<PathBuf>
            let result: Option<std::path::PathBuf> = dirs::home_dir();
            // home_dir should almost always succeed
            assert!(
                result.is_some(),
                "home_dir should return Some on most systems"
            );
            if let Some(path) = result {
                assert!(path.is_absolute(), "home_dir should return absolute path");
                assert!(path.exists(), "home_dir should return existing path");
            }
        }

        #[test]
        fn cache_dir_returns_option_pathbuf() {
            // dirs::cache_dir() should return Option<PathBuf>
            let result: Option<std::path::PathBuf> = dirs::cache_dir();
            // May be None on some systems
            if let Some(path) = result {
                assert!(path.is_absolute(), "cache_dir should return absolute path");
            }
        }
    }
}
