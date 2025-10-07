use crate::{Result, CobraError};
use colored::Colorize;
use std::path::Path;
use tokio::fs;

const DEFAULT_COBRA_TOML: &str = r#"[project]
name = "my-project"
version = "0.1.0"
description = "A Python project managed by Cobra"

[dependencies]
# requests = "^2.31.0"
# numpy = "^1.24.0"

[dev-dependencies]
# pytest = "^7.4.0"

[tool.cobra]
python-version = "3.11"
parallel-downloads = 16
cache-enabled = true
install-dir = ".cobra_packages"  # Local package directory
"#;

pub async fn execute(path: &str) -> Result<()> {
    let cobra_path = Path::new(path).join("cobra.toml");
    
    if cobra_path.exists() {
        return Err(CobraError::Config(
            "cobra.toml already exists in this directory".to_string()
        ));
    }
    
    println!("{} Initializing new Cobra project...", "⚡".bright_yellow());
    
    fs::write(&cobra_path, DEFAULT_COBRA_TOML).await?;
    
    println!("{} Created cobra.toml", "✓".green());
    println!("\nNext steps:");
    println!("  1. Edit cobra.toml to add your dependencies");
    println!("  2. Run {} to install packages", "cobra install".cyan());
    
    Ok(())
}
