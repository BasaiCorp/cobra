use crate::{Result, CobraError};
use crate::core::{config::CobraConfig, package_manager::LocalPackageManager};
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

pub async fn execute() -> Result<()> {
    let config_path = Path::new("cobra.toml");
    if !config_path.exists() {
        return Err(CobraError::Config(
            "No cobra.toml found. Run 'cobra init' to create one.".to_string()
        ));
    }

    let config = CobraConfig::load(config_path).await?;
    
    // Initialize package manager
    let install_dir = std::env::current_dir()?.join(config.get_install_dir());
    let package_manager = Arc::new(LocalPackageManager::new(install_dir));
    
    // Get installed packages
    let installed_packages = package_manager.list_installed().await?;
    
    if installed_packages.is_empty() {
        println!("No packages installed.");
        println!("Run 'cobra install' to install packages from cobra.toml");
        return Ok(());
    }
    
    println!("Installed packages:");
    println!("{}", "─".repeat(50));
    
    for package in &installed_packages {
        let name_colored = package.name.cyan();
        let version_colored = package.version.green();
        let install_time = package.installed_at.format("%Y-%m-%d %H:%M:%S");
        
        println!("{} {} (installed: {})", 
            name_colored, 
            version_colored,
            install_time.to_string().dimmed()
        );
    }
    
    println!("{}", "─".repeat(50));
    println!("Total: {} packages", installed_packages.len().to_string().bold());
    
    Ok(())
}
