use crate::{Result, CobraError};
use crate::core::{config::CobraConfig, package_manager::LocalPackageManager};
use crate::registry::client::RegistryClient;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

pub async fn execute(package_name: String) -> Result<()> {
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
    
    // Check if package is installed locally
    let installed_packages = package_manager.list_installed().await?;
    let local_package = installed_packages.iter().find(|p| p.name == package_name);
    
    // Get package info from PyPI
    let client = RegistryClient::new();
    let package_info = client.get_package_info(&package_name, "*").await?;
    
    // Display package information
    println!("{}", "Package Information".bold().underline());
    println!("{}", "─".repeat(50));
    
    println!("{}: {}", "Name".bold(), package_info.name.cyan());
    println!("{}: {}", "Version".bold(), package_info.version.green());
    
    if let Some(description) = &package_info.description {
        if !description.is_empty() {
            println!("{}: {}", "Description".bold(), description);
        }
    }
    
    if let Some(author) = &package_info.author {
        if !author.is_empty() {
            println!("{}: {}", "Author".bold(), author);
        }
    }
    
    if let Some(homepage) = &package_info.homepage {
        if !homepage.is_empty() {
            println!("{}: {}", "Homepage".bold(), homepage.blue().underline());
        }
    }
    
    if let Some(size) = package_info.size {
        let size_mb = size as f64 / 1024.0 / 1024.0;
        println!("{}: {:.2} MB", "Size".bold(), size_mb);
    }
    
    // Installation status
    println!("{}", "─".repeat(50));
    if let Some(local_pkg) = local_package {
        println!("{}: {} {}", 
            "Status".bold(), 
            "Installed".green().bold(),
            format!("({})", local_pkg.installed_at.format("%Y-%m-%d %H:%M:%S")).dimmed()
        );
        println!("{}: {}", "Install Path".bold(), local_pkg.install_path.display());
        
        if local_pkg.version != package_info.version {
            println!("{}: {} -> {}", 
                "Update Available".bold().yellow(),
                local_pkg.version.red(),
                package_info.version.green()
            );
        }
    } else {
        println!("{}: {}", "Status".bold(), "Not Installed".red());
        println!("Run 'cobra add {}' to add to your project", package_name.cyan());
    }
    
    // Dependencies (if available)
    if !package_info.dependencies.is_empty() {
        println!("{}", "─".repeat(50));
        println!("{}:", "Dependencies".bold());
        for dep in &package_info.dependencies {
            println!("  - {}", dep.name.cyan());
        }
    }
    
    Ok(())
}
