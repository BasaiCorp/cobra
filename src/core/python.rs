use crate::{Result, CobraError};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PythonEnvironment {
    pub python_path: PathBuf,
    pub version: String,
    pub site_packages: PathBuf,
}

impl PythonEnvironment {
    pub async fn detect() -> Result<Self> {
        // Try to find Python executable
        let python_cmd = if cfg!(windows) { "python" } else { "python3" };
        
        let output = Command::new(python_cmd)
            .arg("--version")
            .output()
            .map_err(|e| CobraError::PythonEnv(format!("Failed to execute python: {}", e)))?;
        
        if !output.status.success() {
            return Err(CobraError::PythonEnv("Python not found".to_string()));
        }
        
        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        
        // Get site-packages directory
        let site_output = Command::new(python_cmd)
            .arg("-c")
            .arg("import site; print(site.getsitepackages()[0])")
            .output()
            .map_err(|e| CobraError::PythonEnv(format!("Failed to get site-packages: {}", e)))?;
        
        let site_packages = PathBuf::from(
            String::from_utf8_lossy(&site_output.stdout).trim()
        );
        
        // Get Python executable path
        let path_output = Command::new(python_cmd)
            .arg("-c")
            .arg("import sys; print(sys.executable)")
            .output()
            .map_err(|e| CobraError::PythonEnv(format!("Failed to get python path: {}", e)))?;
        
        let python_path = PathBuf::from(
            String::from_utf8_lossy(&path_output.stdout).trim()
        );
        
        Ok(Self {
            python_path,
            version,
            site_packages,
        })
    }
}
