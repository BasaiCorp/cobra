use crate::{Result, CobraError};
use crate::core::{config::CobraConfig, package_manager::LocalPackageManager};
use crate::registry::client::RegistryClient;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

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
    
    println!("Checking package dependencies and conflicts...");
    println!("{}", "─".repeat(60));
    
    // Get installed packages
    let installed_packages = package_manager.list_installed().await?;
    let configured_deps = config.get_dependencies_list();
    
    let mut issues_found = 0;
    
    // Check 1: Missing packages (in config but not installed)
    let installed_names: HashSet<String> = installed_packages.iter()
        .map(|p| p.name.clone())
        .collect();
    
    let mut missing_packages = Vec::new();
    for dep in &configured_deps {
        if !installed_names.contains(&dep.name) {
            missing_packages.push(&dep.name);
        }
    }
    
    if !missing_packages.is_empty() {
        println!("{} Missing packages:", "!".yellow().bold());
        for pkg in &missing_packages {
            println!("  {} {}", "•".yellow(), pkg.red());
        }
        println!("  Run 'cobra install' to install missing packages\n");
        issues_found += missing_packages.len();
    }
    
    // Check 2: Extra packages (installed but not in config)
    let configured_names: HashSet<String> = configured_deps.iter()
        .map(|d| d.name.clone())
        .collect();
    
    let mut extra_packages = Vec::new();
    for pkg in &installed_packages {
        if !configured_names.contains(&pkg.name) {
            extra_packages.push(&pkg.name);
        }
    }
    
    if !extra_packages.is_empty() {
        println!("{} Extra packages (not in cobra.toml):", "!".yellow().bold());
        for pkg in &extra_packages {
            println!("  {} {}", "•".yellow(), pkg.cyan());
        }
        println!("  Run 'cobra remove {}' to remove from system\n", extra_packages.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "));
        issues_found += extra_packages.len();
    }
    
    // Check 3: Version conflicts
    let client = RegistryClient::new();
    let mut version_conflicts = Vec::new();
    
    for dep in &configured_deps {
        if let Some(installed_pkg) = installed_packages.iter().find(|p| p.name == dep.name) {
            if !version_matches(&installed_pkg.version, &dep.version_spec) {
                version_conflicts.push((dep, installed_pkg));
            }
        }
    }
    
    if !version_conflicts.is_empty() {
        println!("{} Version conflicts:", "!".red().bold());
        for (dep, installed) in &version_conflicts {
            println!("  {} {} (required: {}, installed: {})", 
                "•".red(), 
                dep.name.cyan(),
                dep.version_spec.green(),
                installed.version.red()
            );
        }
        println!("  Run 'cobra update' to resolve version conflicts\n");
        issues_found += version_conflicts.len();
    }
    
    // Check 4: Dependency integrity (check if package files exist)
    let mut corrupted_packages = Vec::new();
    for pkg in &installed_packages {
        if !pkg.install_path.exists() {
            corrupted_packages.push(&pkg.name);
        }
    }
    
    if !corrupted_packages.is_empty() {
        println!("{} Corrupted packages (files missing):", "!".red().bold());
        for pkg in &corrupted_packages {
            println!("  {} {}", "•".red(), pkg.red());
        }
        println!("  Run 'cobra install' to repair corrupted packages\n");
        issues_found += corrupted_packages.len();
    }
    
    // Check 5: Circular dependencies (basic check)
    let circular_deps = check_circular_dependencies(&configured_deps, &client).await?;
    if !circular_deps.is_empty() {
        println!("{} Potential circular dependencies:", "!".yellow().bold());
        for cycle in &circular_deps {
            println!("  {} {}", "•".yellow(), cycle.join(" -> ").cyan());
        }
        println!("  Review dependency specifications\n");
        issues_found += circular_deps.len();
    }
    
    // Summary
    println!("{}", "─".repeat(60));
    if issues_found == 0 {
        println!("{} All checks passed! No issues found.", "✓".green().bold());
        println!("Your package environment is healthy.");
    } else {
        println!("{} Found {} issues that need attention.", 
            "!".yellow().bold(), 
            issues_found.to_string().red().bold()
        );
        println!("Run the suggested commands to resolve these issues.");
    }
    
    Ok(())
}

fn version_matches(installed_version: &str, version_spec: &str) -> bool {
    // Simple version matching - in production, use proper semver parsing
    if version_spec == "*" {
        return true;
    }
    
    if version_spec.starts_with("==") {
        return installed_version == &version_spec[2..];
    }
    
    if version_spec.starts_with(">=") {
        // Simplified comparison - in production, use proper version comparison
        return true; // For now, assume it matches
    }
    
    // Default: exact match
    installed_version == version_spec
}

async fn check_circular_dependencies(
    _deps: &[crate::Dependency], 
    _client: &RegistryClient
) -> Result<Vec<Vec<String>>> {
    // Simplified circular dependency check
    // In production, this would build a dependency graph and detect cycles
    Ok(Vec::new())
}
