use crate::{Result, CobraError};
use crate::core::{config::CobraConfig, package_manager::LocalPackageManager};
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

pub async fn execute(packages: Vec<String>) -> Result<()> {
    if packages.is_empty() {
        return Err(CobraError::InvalidInput("No packages specified for uninstall".to_string()));
    }

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
    
    println!("Uninstalling packages...");
    
    let mut uninstalled_count = 0;
    let mut not_found_count = 0;
    
    for package_name in &packages {
        match uninstall_single_package(&package_manager, package_name).await {
            Ok(was_installed) => {
                if was_installed {
                    println!("  {} {}", "✓".green(), format!("Uninstalled {}", package_name).cyan());
                    uninstalled_count += 1;
                } else {
                    println!("  {} {}", "!".yellow(), format!("{} was not installed", package_name).dimmed());
                    not_found_count += 1;
                }
            }
            Err(e) => {
                println!("  {} Failed to uninstall {}: {}", "✗".red(), package_name.cyan(), e);
                return Err(e);
            }
        }
    }
    
    // Update .pth file after uninstallation
    if uninstalled_count > 0 {
        if let Err(e) = package_manager.create_pth_file().await {
            println!("Warning: Failed to update Python path file: {}", e);
        }
    }
    
    // Summary
    println!("{}", "─".repeat(50));
    if uninstalled_count > 0 {
        println!("Successfully uninstalled {} packages", uninstalled_count.to_string().green().bold());
    }
    if not_found_count > 0 {
        println!("{} packages were not installed", not_found_count.to_string().yellow());
    }
    
    if uninstalled_count > 0 {
        println!("\nNote: Packages removed from system but still listed in cobra.toml");
        println!("Run 'cobra remove {}' to remove from configuration", packages.join(" "));
    }
    
    Ok(())
}

async fn uninstall_single_package(
    package_manager: &LocalPackageManager, 
    package_name: &str
) -> Result<bool> {
    // Check if package is installed
    let installed_packages = package_manager.list_installed().await?;
    let package = installed_packages.iter().find(|p| p.name == package_name);
    
    if let Some(pkg) = package {
        // Remove package directory
        if pkg.install_path.exists() {
            fs::remove_dir_all(&pkg.install_path).await?;
        }
        
        // Remove dist-info directory if it exists
        let dist_info_path = pkg.install_path.parent()
            .unwrap()
            .join(format!("{}-{}.dist-info", pkg.name, pkg.version));
        
        if dist_info_path.exists() {
            fs::remove_dir_all(&dist_info_path).await?;
        }
        
        // Remove from registry
        package_manager.unregister_package(package_name).await?;
        
        Ok(true)
    } else {
        Ok(false)
    }
}
