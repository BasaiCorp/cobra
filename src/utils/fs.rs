use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use crate::{Result, CobraError};

/// Atomic write operation - write to temp file then rename
pub async fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent()
        .ok_or_else(|| CobraError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Parent directory not found"
        )))?;
    
    fs::create_dir_all(parent).await?;
    
    // Write to temporary file first
    let temp_path = parent.join(format!(".{}.tmp", 
        path.file_name().unwrap().to_string_lossy()));
    
    let mut file = fs::File::create(&temp_path).await?;
    file.write_all(contents).await?;
    file.sync_all().await?;
    
    // Atomic rename
    fs::rename(temp_path, path).await?;
    Ok(())
}

/// Fast directory copy with parallel file operations
pub async fn copy_dir_parallel(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).await?;
    
    let mut entries = fs::read_dir(src).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if entry.file_type().await?.is_dir() {
            Box::pin(copy_dir_parallel(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }
    
    Ok(())
}

/// Get cache directory for Cobra
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| CobraError::Io(
            std::io::Error::new(std::io::ErrorKind::NotFound, "Cache directory not found")
        ))?
        .join("cobra");
    
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Get config directory for Cobra
pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| CobraError::Io(
            std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found")
        ))?
        .join("cobra");
    
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

/// Calculate directory size
pub async fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    let mut entries = fs::read_dir(path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        if metadata.is_file() {
            total += metadata.len();
        } else if metadata.is_dir() {
            total += Box::pin(dir_size(&entry.path())).await?;
        }
    }
    
    Ok(total)
}
