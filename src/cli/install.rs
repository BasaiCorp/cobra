use crate::{Result, CobraError};
use crate::core::{config::CobraConfig, installer::Installer, resolver::DependencyResolver, cache::MultiLevelCache};
use crate::registry::client::RegistryClient;
use crate::utils::progress::ProgressTracker;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

pub async fn execute(no_cache: bool) -> Result<()> {
    let start = Instant::now();
    
    // Load configuration
    let config_path = Path::new("cobra.toml");
    if !config_path.exists() {
        return Err(CobraError::Config(
            "cobra.toml not found. Run 'cobra init' first.".to_string()
        ));
    }
    
    println!("{} Loading configuration...", "⚡".bright_yellow());
    let config = CobraConfig::load(config_path).await?;
    
    println!("{} Found {} dependencies", "✓".green(), config.dependencies.len());
    
    // Initialize components
    let cache = if no_cache {
        None
    } else {
        Some(Arc::new(MultiLevelCache::new().await?))
    };
    
    let client = Arc::new(RegistryClient::new());
    let progress = Arc::new(ProgressTracker::new());
    
    // Resolve dependencies
    println!("{} Resolving dependency graph...", "🔍".bright_blue());
    let resolver = DependencyResolver::new(client.clone(), cache.clone());
    let resolved = resolver.resolve(&config.dependencies).await?;
    
    let resolve_time = start.elapsed();
    println!("{} Resolved {} packages in {:.2}ms", 
        "✓".green(), 
        resolved.len(),
        resolve_time.as_secs_f64() * 1000.0
    );
    
    // Install packages in parallel
    println!("{} Installing packages...", "📦".bright_blue());
    let installer = Installer::new(client, cache, progress.clone());
    installer.install_parallel(resolved).await?;
    
    let total_time = start.elapsed();
    println!("\n{} Installation complete in {:.2}s", 
        "✓".green().bold(),
        total_time.as_secs_f64()
    );
    
    Ok(())
}
