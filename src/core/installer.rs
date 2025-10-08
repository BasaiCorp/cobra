use crate::{Result, CobraError, Package, constants::*};
use crate::core::cache::MultiLevelCache;
use crate::core::package_manager::LocalPackageManager;
use crate::registry::client::RegistryClient;
use crate::utils::progress::ProgressTracker;
use crate::utils::hash::verify_package_hash;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tokio::sync::Semaphore;
use tokio::io::AsyncWriteExt;
use tokio::fs;
use futures::stream::StreamExt;
use zip::ZipArchive;
use memmap2::MmapOptions;
use std::io::Cursor;
use rayon::prelude::*;

pub struct Installer {
    client: Arc<RegistryClient>,
    cache: Option<Arc<MultiLevelCache>>,
    progress: Arc<ProgressTracker>,
    package_manager: Arc<LocalPackageManager>,
}

impl Installer {
    pub fn new(
        client: Arc<RegistryClient>,
        cache: Option<Arc<MultiLevelCache>>,
        progress: Arc<ProgressTracker>,
        package_manager: Arc<LocalPackageManager>,
    ) -> Self {
        Self {
            client,
            cache,
            progress,
            package_manager,
        }
    }

    /// Install packages in parallel with streaming downloads
    pub async fn install_parallel(&self, packages: Vec<Package>) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        // Ensure installation directory exists
        self.package_manager.ensure_install_dir().await?;

        // Filter out already installed packages
        let mut packages_to_install = Vec::new();
        let mut skipped_count = 0;

        for package in packages {
            if self.package_manager.is_package_installed(&package.name, &package.version).await? {
                println!("⏭️  Skipping {} {} (already installed)", package.name, package.version);
                skipped_count += 1;
            } else {
                packages_to_install.push(package);
            }
        }

        if packages_to_install.is_empty() {
            println!("✅ All {} packages are already installed!", skipped_count);
            return Ok(());
        }

        if skipped_count > 0 {
            println!("📦 Installing {} new packages ({} already installed)", 
                packages_to_install.len(), skipped_count);
        }

        // Semaphore to limit concurrent operations
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_INSTALLS));
        
        let tasks: Vec<_> = packages_to_install.into_iter().map(|pkg| {
            let sem = Arc::clone(&semaphore);
            let client = Arc::clone(&self.client);
            let cache = self.cache.clone();
            let progress = Arc::clone(&self.progress);
            let package_manager = Arc::clone(&self.package_manager);
            
            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                Self::install_single(pkg, client, cache, progress, package_manager).await
            })
        }).collect();

        // Wait for all installations to complete
        let results = futures::future::join_all(tasks).await;
        
        for result in results {
            result.map_err(|e| CobraError::InstallationFailed(e.to_string()))??;
        }

        // Create .pth file to make packages discoverable by Python
        self.package_manager.create_pth_file().await?;

        Ok(())
    }

    async fn install_single(
        package: Package,
        client: Arc<RegistryClient>,
        cache: Option<Arc<MultiLevelCache>>,
        progress: Arc<ProgressTracker>,
        package_manager: Arc<LocalPackageManager>,
    ) -> Result<()> {
        // Check cache first
        let cache_key = format!("package:{}:{}", package.name, package.version);
        
        let package_data = if let Some(cache) = &cache {
            if let Some(data) = cache.get(&cache_key).await {
                data
            } else {
                // Download package
                let data = Self::download_package(&package, &client, &progress).await?;
                let _ = cache.put(cache_key, data.clone()).await;
                data
            }
        } else {
            Self::download_package(&package, &client, &progress).await?
        };

        // Extract package (skip hash verification for now)
        let temp_path = std::env::temp_dir().join(format!("{}.whl", package.name));
        fs::write(&temp_path, &package_data).await?;
        Self::extract_package_mmap(&temp_path, &package.name, &package_manager).await?;
        fs::remove_file(&temp_path).await?;

        // Register the installed package
        package_manager.register_package(&package).await?;

        Ok(())
    }

    async fn download_package(
        package: &Package,
        client: &RegistryClient,
        progress: &ProgressTracker,
    ) -> Result<bytes::Bytes> {
        let size = package.size.unwrap_or(0);
        let pb = progress.add_download(&package.name, size).await;

        let response = client.download_package(&package.download_url).await?;
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| CobraError::Network(e))?;
            buffer.extend_from_slice(&chunk);
            pb.inc(chunk.len() as u64);
        }

        pb.finish_with_message(format!("✓ {}", package.name));
        Ok(bytes::Bytes::from(buffer))
    }

    async fn extract_package_mmap(archive_path: &Path, _package_name: &str, package_manager: &LocalPackageManager) -> Result<()> {
        // Use the package manager's installation directory
        let site_packages = package_manager.get_install_dir();
        
        // Ensure the site-packages directory exists
        fs::create_dir_all(&site_packages).await?;

        // Use memory-mapped file for faster extraction
        let file = std::fs::File::open(archive_path)
            .map_err(|e| CobraError::Archive(format!("Failed to open archive: {}", e)))?;
        
        let mmap = unsafe { 
            MmapOptions::new().map(&file)
                .map_err(|e| CobraError::Archive(format!("Failed to mmap file: {}", e)))?
        };

        let cursor = Cursor::new(&mmap[..]);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| CobraError::Archive(format!("Failed to read archive: {}", e)))?;

        // Extract files in parallel using rayon
        let indices: Vec<usize> = (0..archive.len()).collect();
        
        // Note: We need to extract sequentially due to ZipArchive borrowing rules
        // But we can still optimize with buffering
        for i in indices {
            let mut file = archive.by_index(i)
                .map_err(|e| CobraError::Archive(format!("Failed to read file: {}", e)))?;
            
            if file.is_file() {
                let outpath = site_packages.join(file.name());
                
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }
}
