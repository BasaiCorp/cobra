use crate::{Result, CobraError, Dependency};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CobraConfig {
    pub project: ProjectInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub tool: ToolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ToolConfig {
    #[serde(default)]
    pub cobra: CobraToolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CobraToolConfig {
    #[serde(default = "default_python_version", rename = "python-version")]
    pub python_version: String,
    #[serde(default = "default_parallel_downloads", rename = "parallel-downloads")]
    pub parallel_downloads: usize,
    #[serde(default = "default_cache_enabled", rename = "cache-enabled")]
    pub cache_enabled: bool,
    #[serde(default = "default_install_dir", rename = "install-dir")]
    pub install_dir: String,
}

impl Default for CobraToolConfig {
    fn default() -> Self {
        Self {
            python_version: default_python_version(),
            parallel_downloads: default_parallel_downloads(),
            cache_enabled: default_cache_enabled(),
            install_dir: default_install_dir(),
        }
    }
}

fn default_python_version() -> String {
    "3.11".to_string()
}

fn default_parallel_downloads() -> usize {
    16
}

fn default_cache_enabled() -> bool {
    true
}

fn default_install_dir() -> String {
    ".cobra_packages".to_string()
}

impl CobraConfig {
    pub async fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).await?;
        let config: CobraConfig = toml::from_str(&contents)
            .map_err(|e| CobraError::Config(format!("Failed to parse cobra.toml: {}", e)))?;
        Ok(config)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| CobraError::Config(format!("Failed to serialize config: {}", e)))?;
        fs::write(path, contents).await?;
        Ok(())
    }

    pub fn add_dependency(&mut self, name: &str, version: &str) {
        self.dependencies.insert(name.to_string(), version.to_string());
    }

    pub fn remove_dependency(&mut self, name: &str) -> bool {
        self.dependencies.remove(name).is_some()
    }

    pub fn get_dependency(&self, name: &str) -> Option<String> {
        self.dependencies.get(name).cloned()
    }

    /// Convert HashMap dependencies to Vec<Dependency> for processing
    pub fn get_dependencies_list(&self) -> Vec<Dependency> {
        self.dependencies
            .iter()
            .map(|(name, version_spec)| Dependency {
                name: name.clone(),
                version_spec: version_spec.clone(),
            })
            .collect()
    }

    /// Get install directory path
    pub fn get_install_dir(&self) -> String {
        self.tool.cobra.install_dir.clone()
    }
}

impl Default for CobraConfig {
    fn default() -> Self {
        Self {
            project: ProjectInfo {
                name: "my-project".to_string(),
                version: "0.1.0".to_string(),
                description: String::new(),
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            tool: ToolConfig::default(),
        }
    }
}
