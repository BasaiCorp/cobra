use crate::{Result, CobraError, Package};
use std::path::{Path, PathBuf};
use tokio::fs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageRegistry {
    pub packages: HashMap<String, InstalledPackage>,
}

pub struct LocalPackageManager {
    install_dir: PathBuf,
    registry_path: PathBuf,
}

impl LocalPackageManager {
    pub fn new(install_dir: PathBuf) -> Self {
        let registry_path = install_dir.join("cobra-registry.json");
        Self {
            install_dir,
            registry_path,
        }
    }

    /// Ensure the installation directory exists
    pub async fn ensure_install_dir(&self) -> Result<()> {
        if !self.install_dir.exists() {
            fs::create_dir_all(&self.install_dir).await?;
            println!("📁 Created installation directory: {}", self.install_dir.display());
        }
        Ok(())
    }

    /// Load the package registry
    pub async fn load_registry(&self) -> Result<PackageRegistry> {
        if !self.registry_path.exists() {
            return Ok(PackageRegistry::default());
        }

        let contents = fs::read_to_string(&self.registry_path).await?;
        let registry: PackageRegistry = serde_json::from_str(&contents)
            .map_err(|e| CobraError::Config(format!("Failed to parse registry: {}", e)))?;
        Ok(registry)
    }

    /// Save the package registry
    pub async fn save_registry(&self, registry: &PackageRegistry) -> Result<()> {
        let contents = serde_json::to_string_pretty(registry)
            .map_err(|e| CobraError::Config(format!("Failed to serialize registry: {}", e)))?;
        fs::write(&self.registry_path, contents).await?;
        Ok(())
    }

    /// Check if a package is already installed with the correct version
    pub async fn is_package_installed(&self, name: &str, version: &str) -> Result<bool> {
        let registry = self.load_registry().await?;
        
        if let Some(installed) = registry.packages.get(name) {
            // Check if the installed version satisfies the requirement
            if self.version_satisfies(&installed.version, version) {
                // Also verify the package directory still exists
                if installed.install_path.exists() {
                    return Ok(true);
                } else {
                    // Package directory was deleted, remove from registry
                    let mut updated_registry = registry;
                    updated_registry.packages.remove(name);
                    self.save_registry(&updated_registry).await?;
                }
            }
        }
        
        Ok(false)
    }

    /// Register a newly installed package
    pub async fn register_package(&self, package: &Package) -> Result<()> {
        let mut registry = self.load_registry().await?;
        
        let installed_package = InstalledPackage {
            name: package.name.clone(),
            version: package.version.clone(),
            install_path: self.install_dir.join(&package.name),
            installed_at: chrono::Utc::now(),
        };
        
        registry.packages.insert(package.name.clone(), installed_package);
        self.save_registry(&registry).await?;
        Ok(())
    }

    /// Get list of installed packages
    pub async fn list_installed(&self) -> Result<Vec<InstalledPackage>> {
        let registry = self.load_registry().await?;
        Ok(registry.packages.values().cloned().collect())
    }

    /// Remove a package from registry
    pub async fn unregister_package(&self, name: &str) -> Result<bool> {
        let mut registry = self.load_registry().await?;
        let removed = registry.packages.remove(name).is_some();
        if removed {
            self.save_registry(&registry).await?;
        }
        Ok(removed)
    }

    /// Simple version satisfaction check (can be enhanced later)
    fn version_satisfies(&self, installed: &str, required: &str) -> bool {
        if required == "*" {
            return true;
        }
        
        // Handle exact version matches
        if required.starts_with("==") {
            return installed == &required[2..];
        }
        
        // For now, just check exact match for other cases
        // TODO: Implement proper semantic versioning
        installed == required
    }

    /// Get the installation directory
    pub fn get_install_dir(&self) -> &Path {
        &self.install_dir
    }

    /// Create a .pth file to make packages discoverable by Python
    pub async fn create_pth_file(&self) -> Result<()> {
        // Get user site-packages directory
        let output = std::process::Command::new("python3")
            .arg("-c")
            .arg("import site; print(site.getusersitepackages())")
            .output()
            .map_err(|e| CobraError::PythonEnv(format!("Failed to get user site-packages: {}", e)))?;

        if !output.status.success() {
            return Err(CobraError::PythonEnv("Failed to get user site-packages".to_string()));
        }

        let user_site_packages = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let user_site_path = PathBuf::from(&user_site_packages);

        // Ensure user site-packages exists
        fs::create_dir_all(&user_site_path).await?;

        // Create .pth file pointing to our installation directory
        let pth_file = user_site_path.join("cobra-packages.pth");
        let install_dir_str = self.install_dir.to_string_lossy().to_string();
        
        fs::write(&pth_file, format!("{}\n", install_dir_str)).await?;
        
        println!("📝 Created Python path file: {}", pth_file.display());
        println!("🔗 Packages are now available to Python globally!");
        
        Ok(())
    }

    /// Remove the .pth file
    pub async fn remove_pth_file(&self) -> Result<()> {
        let output = std::process::Command::new("python3")
            .arg("-c")
            .arg("import site; print(site.getusersitepackages())")
            .output()
            .map_err(|e| CobraError::PythonEnv(format!("Failed to get user site-packages: {}", e)))?;

        if !output.status.success() {
            return Ok(()); // Silently fail if we can't get site-packages
        }

        let user_site_packages = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let pth_file = PathBuf::from(&user_site_packages).join("cobra-packages.pth");

        if pth_file.exists() {
            fs::remove_file(&pth_file).await?;
            println!("🗑️  Removed Python path file: {}", pth_file.display());
        }

        Ok(())
    }
}
