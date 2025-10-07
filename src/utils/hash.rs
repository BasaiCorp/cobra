use blake3::Hasher;
use sha2::{Sha256, Digest};
use std::path::Path;
use tokio::io::AsyncReadExt;
use crate::{Result, CobraError};

/// Verify package hash using BLAKE3 (faster) or SHA256
pub async fn verify_package_hash(path: &Path, expected_hash: &str) -> Result<bool> {
    let computed = compute_hash(path).await?;
    Ok(computed == expected_hash)
}

/// Compute BLAKE3 hash for a file (3x faster than SHA256)
pub async fn compute_hash(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; 65536]; // 64KB buffer for optimal performance

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Compute SHA256 hash (for compatibility with PyPI)
pub async fn compute_sha256(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 65536];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Parallel hash computation for multiple files
pub async fn compute_hashes_parallel(paths: Vec<&Path>) -> Result<Vec<String>> {
    let futures: Vec<_> = paths.into_iter()
        .map(|path| compute_hash(path))
        .collect();
    
    futures::future::try_join_all(futures).await
}
