use crate::{Result, CobraError};
use crate::core::{config::CobraConfig, resolver::DependencyResolver, installer::Installer, cache::MultiLevelCache};
use crate::registry::client::RegistryClient;
use crate::utils::progress::ProgressTracker;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

pub async fn execute(package: Option<String>) -> Result<()> {
    let config_path = Path::new("cobra.toml");
    if !config_path.exists() {
        return Err(CobraError::Config(
            "cobra.toml not found. Run 'cobra init' first.".to_string()
        ));
    }
    
    let config = CobraConfig::load(config_path).await?;
    
    match package {
        Some(pkg_name) => {
            println!("{} Updating {}...", "⚡".bright_yellow(), pkg_name.cyan());
            update_single_package(&config, &pkg_name).await?;
        }
        None => {
            println!("{} Updating all packages...", "⚡".bright_yellow());
            update_all_packages(&config).await?;
        }
    }
    
    Ok(())
}

async fn update_single_package(config: &CobraConfig, package_name: &str) -> Result<()> {
    let cache = Arc::new(MultiLevelCache::new().await?);
    let client = Arc::new(RegistryClient::new());
    let progress = Arc::new(ProgressTracker::new());
    
    // Find the package in dependencies
    let version_spec = config.dependencies.get(package_name)
        .ok_or_else(|| CobraError::PackageNotFound(package_name.to_string()))?;
    
    println!("{} Checking for updates...", "🔍".bright_blue());
    
    let dep = crate::Dependency {
        name: package_name.to_string(),
        version_spec: version_spec.clone(),
    };
    
    let resolver = DependencyResolver::new(client.clone(), Some(cache.clone()));
    let resolved = resolver.resolve(&[dep]).await?;
    
    let installer = Installer::new(client, Some(cache), progress);
    installer.install_parallel(resolved).await?;
    
    println!("{} {} updated successfully", "✓".green(), package_name.cyan());
    Ok(())
}

async fn update_all_packages(config: &CobraConfig) -> Result<()> {
    let cache = Arc::new(MultiLevelCache::new().await?);
    let client = Arc::new(RegistryClient::new());
    let progress = Arc::new(ProgressTracker::new());
    
    println!("{} Resolving latest versions...", "🔍".bright_blue());
    
    let dependencies_list = config.get_dependencies_list();
    let resolver = DependencyResolver::new(client.clone(), Some(cache.clone()));
    let resolved = resolver.resolve(&dependencies_list).await?;
    
    println!("{} Installing {} packages...", "📦".bright_blue(), resolved.len());
    
    let installer = Installer::new(client, Some(cache), progress);
    installer.install_parallel(resolved).await?;
    
    println!("{} All packages updated successfully", "✓".green().bold());
    Ok(())
}
