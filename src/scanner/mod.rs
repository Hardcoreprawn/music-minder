use futures::stream::Stream;
use std::path::PathBuf;
use tokio::sync::mpsc;
use walkdir::WalkDir;

/// Scans the given root directory recursively for audio files.
///
/// Supported extensions: mp3, flac, ogg, wav, m4a (case-insensitive).
/// Returns a Stream of PathBufs.
pub fn scan(root: PathBuf) -> impl Stream<Item = PathBuf> {
    let (tx, rx) = mpsc::channel(100);

    // Spawn a blocking task to perform the synchronous file system traversal
    tokio::task::spawn_blocking(move || {
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext = ext.to_lowercase();
                    match ext.as_str() {
                        "mp3" | "flac" | "ogg" | "wav" | "m4a" => {
                            // Send the path to the channel. If the receiver is dropped,
                            // blocking_send will return an error, and we stop scanning.
                            if tx.blocking_send(path.to_path_buf()).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Convert the mpsc Receiver into a Stream
    futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|path| (path, rx))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_scan_audio_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create dummy files in root
        File::create(root.join("song.mp3")).unwrap();
        File::create(root.join("music.flac")).unwrap();
        File::create(root.join("notes.txt")).unwrap(); // Should be ignored
        File::create(root.join("image.png")).unwrap(); // Should be ignored
        File::create(root.join("UPPERCASE.OGG")).unwrap(); // Should be found (case-insensitive)

        // Create subdirectory
        let subdir = root.join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        File::create(subdir.join("track.wav")).unwrap();
        File::create(subdir.join("ignore.doc")).unwrap(); // Should be ignored

        // Collect results
        let paths: Vec<PathBuf> = scan(root.to_path_buf()).collect().await;

        // Verify count
        assert_eq!(paths.len(), 4);

        // Verify contents (checking file names)
        let file_names: Vec<String> = paths
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
            .collect();

        assert!(file_names.contains(&"song.mp3".to_string()));
        assert!(file_names.contains(&"music.flac".to_string()));
        assert!(file_names.contains(&"track.wav".to_string()));
        assert!(file_names.contains(&"UPPERCASE.OGG".to_string()));
        
        assert!(!file_names.contains(&"notes.txt".to_string()));
        assert!(!file_names.contains(&"image.png".to_string()));
    }
}