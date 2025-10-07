use crate::{Result, CobraError};
use crate::core::config::CobraConfig;
use colored::Colorize;
use std::path::Path;

pub async fn execute(packages: Vec<String>) -> Result<()> {
    if packages.is_empty() {
        return Err(CobraError::Config("No packages specified".to_string()));
    }
    
    let config_path = Path::new("cobra.toml");
    if !config_path.exists() {
        return Err(CobraError::Config(
            "cobra.toml not found. Run 'cobra init' first.".to_string()
        ));
    }
    
    println!("{} Removing packages...", "⚡".bright_yellow());
    
    let mut config = CobraConfig::load(config_path).await?;
    
    for package in &packages {
        if config.remove_dependency(package) {
            println!("{} Removed {}", "✓".green(), package.cyan());
        } else {
            println!("{} Package {} not found in dependencies", 
                "⚠".yellow(), 
                package.cyan()
            );
        }
    }
    
    config.save(config_path).await?;
    
    println!("\n{} Packages removed from cobra.toml", "✓".green());
    println!("{} Run {} to update your environment", 
        "💡".bright_yellow(),
        "cobra install".cyan()
    );
    
    Ok(())
}
