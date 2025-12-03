//! File hash computation for change detection.
//!
//! Provides efficient partial hashing of large files by only reading
//! the first and last 1MB, which is sufficient for detecting changes.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Compute a partial hash of a file (first 1MB + last 1MB).
///
/// This is fast for large files while still detecting most changes.
/// The file size is included in the hash, so files of different sizes
/// will have different hashes even if their sampled content matches.
///
/// # Arguments
///
/// * `path` - Path to the file to hash
///
/// # Returns
///
/// SHA256 hash as a lowercase hex string (64 characters)
///
/// # Errors
///
/// Returns an IO error if the file cannot be read.
pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    let mut hasher = Sha256::new();

    // Hash file size first (so different sized files have different hashes)
    hasher.update(file_size.to_le_bytes());

    const CHUNK_SIZE: u64 = 1024 * 1024; // 1MB

    if file_size <= CHUNK_SIZE * 2 {
        // Small file - hash the whole thing
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        hasher.update(&buffer);
    } else {
        // Large file - hash first and last 1MB
        let mut buffer = vec![0u8; CHUNK_SIZE as usize];

        // First chunk
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);

        // Last chunk
        file.seek(SeekFrom::End(-(CHUNK_SIZE as i64)))?;
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_compute_file_hash_small_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
        drop(file);

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex

        // Same content should give same hash
        let hash2 = compute_file_hash(&file_path).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_compute_file_hash_different_content() {
        let dir = tempdir().unwrap();

        let file1_path = dir.path().join("test1.txt");
        let file2_path = dir.path().join("test2.txt");

        std::fs::write(&file1_path, b"Content A").unwrap();
        std::fs::write(&file2_path, b"Content B").unwrap();

        let hash1 = compute_file_hash(&file1_path).unwrap();
        let hash2 = compute_file_hash(&file2_path).unwrap();

        assert_ne!(hash1, hash2);
    }
}
