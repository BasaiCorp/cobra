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
    
    println!("{} Adding packages...", "⚡".bright_yellow());
    
    let mut config = CobraConfig::load(config_path).await?;
    
    for package in &packages {
        let (name, version) = parse_package_spec(package)?;
        config.add_dependency(&name, &version);
        println!("{} Added {} {}", "✓".green(), name.cyan(), version.dimmed());
    }
    
    config.save(config_path).await?;
    
    println!("\n{} Run {} to install the new packages", 
        "💡".bright_yellow(),
        "cobra install".cyan()
    );
    
    Ok(())
}

fn parse_package_spec(spec: &str) -> Result<(String, String)> {
    if let Some((name, version)) = spec.split_once('@') {
        Ok((name.to_string(), version.to_string()))
    } else if let Some((name, version)) = spec.split_once("==") {
        Ok((name.to_string(), format!("=={}", version)))
    } else {
        // No version specified, use latest
        Ok((spec.to_string(), "*".to_string()))
    }
}
