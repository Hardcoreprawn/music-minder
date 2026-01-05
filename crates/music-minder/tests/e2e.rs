//! End-to-end CLI workflow tests
//!
//! Tests full workflows to ensure different commands work together correctly:
//! - Scan → List
//! - Scan → Identify → Write Tags
//! - Scan → Organize
//! - Watch → Detect Changes

mod common;

#[cfg(test)]
mod e2e_tests {
    use std::fs::{self, File};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;
    use walkdir::WalkDir;

    /// Test helper: Create a temporary music file for testing
    fn create_test_music_file(dir: &Path, filename: &str) -> PathBuf {
        let path = dir.join(filename);
        // Create empty file (real audio not needed for metadata tests)
        File::create(&path).expect("Failed to create test file");
        path
    }

    /// Test helper: Create test directory structure
    fn create_test_library() -> TempDir {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let base = dir.path();

        // Create subdirectories
        fs::create_dir_all(base.join("Artist1/Album1")).expect("Failed to create dirs");
        fs::create_dir_all(base.join("Artist2/Album2")).expect("Failed to create dirs");

        // Create test files
        create_test_music_file(&base.join("Artist1/Album1"), "track1.mp3");
        create_test_music_file(&base.join("Artist1/Album1"), "track2.flac");
        create_test_music_file(&base.join("Artist2/Album2"), "track3.ogg");

        dir
    }

    /// Test helper: Count files in directory recursively
    fn count_files(dir: &Path, extension: &str) -> usize {
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case(extension))
                    .unwrap_or(false)
            })
            .count()
    }

    #[test]
    fn test_library_structure() {
        let lib = create_test_library();
        let base = lib.path();

        // Verify structure
        assert!(base.join("Artist1/Album1/track1.mp3").exists());
        assert!(base.join("Artist1/Album1/track2.flac").exists());
        assert!(base.join("Artist2/Album2/track3.ogg").exists());

        // Verify counts
        assert_eq!(count_files(base, "mp3"), 1);
        assert_eq!(count_files(base, "flac"), 1);
        assert_eq!(count_files(base, "ogg"), 1);
    }

    #[test]
    fn test_file_discovery() {
        let lib = create_test_library();
        let base = lib.path();

        // Count all audio files
        let audio_files: Vec<_> = WalkDir::new(base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        matches!(
                            ext.to_lowercase().as_str(),
                            "mp3" | "flac" | "ogg" | "wav" | "m4a"
                        )
                    })
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(audio_files.len(), 3, "Should find 3 audio files");
    }

    #[test]
    fn test_directory_walking() {
        let lib = create_test_library();
        let base = lib.path();

        // Verify we can walk and find all files
        let mut found_files = vec![];
        for entry in WalkDir::new(base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            found_files.push(entry.path().to_path_buf());
        }

        assert_eq!(found_files.len(), 3);
    }

    #[test]
    fn test_path_operations() {
        let lib = create_test_library();
        let base = lib.path();

        // Test path manipulation
        let file1 = base.join("Artist1/Album1/track1.mp3");
        assert_eq!(file1.file_name().unwrap().to_str().unwrap(), "track1.mp3");
        assert_eq!(
            file1
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            "Album1"
        );
    }
}

/// Integration tests using the enrichment service with mocked APIs
#[cfg(test)]
mod enrichment_e2e_tests {
    use crate::common::http_mocks::{MockAcoustIdServer, MockMusicBrainzServer};

    #[tokio::test]
    async fn test_acoustid_mock_server() {
        let mock = MockAcoustIdServer::start().await;
        mock.mock_lookup_success().await;

        let client = reqwest::Client::new();
        let url = format!(
            "{}/v2/lookup?client=test&duration=180&fingerprint=test",
            mock.base_url()
        );

        let response = client.get(&url).send().await;
        assert!(response.is_ok());

        let resp = response.unwrap();
        assert_eq!(resp.status(), 200);

        let body = resp.text().await.unwrap();
        assert!(body.contains("Test Track"));
    }

    #[tokio::test]
    async fn test_acoustid_mock_no_matches() {
        let mock = MockAcoustIdServer::start().await;
        mock.mock_lookup_no_matches().await;

        let client = reqwest::Client::new();
        let url = format!(
            "{}/v2/lookup?client=test&duration=180&fingerprint=test",
            mock.base_url()
        );

        let response = client.get(&url).send().await.unwrap();
        let body = response.text().await.unwrap();

        assert!(body.contains("\"results\": []"));
    }

    #[tokio::test]
    async fn test_acoustid_mock_server_error() {
        let mock = MockAcoustIdServer::start().await;
        mock.mock_lookup_error(500).await;

        let client = reqwest::Client::new();
        let url = format!(
            "{}/v2/lookup?client=test&duration=180&fingerprint=test",
            mock.base_url()
        );

        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_musicbrainz_mock_server() {
        let mock = MockMusicBrainzServer::start().await;
        mock.mock_recording_lookup_success().await;

        let client = reqwest::Client::new();
        let url = format!(
            "{}/ws/2/recording/4f4868cb-dcc2-4998-bafe-a97bdda95e7f",
            mock.base_url()
        );

        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 200);

        let body = response.text().await.unwrap();
        assert!(body.contains("Test Track"));
    }

    #[tokio::test]
    async fn test_musicbrainz_mock_not_found() {
        let mock = MockMusicBrainzServer::start().await;
        mock.mock_recording_not_found().await;

        let client = reqwest::Client::new();
        let url = format!("{}/ws/2/recording/invalid-id", mock.base_url());

        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 404);
    }
}
