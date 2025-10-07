use crate::{Result, CobraError, Dependency};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CobraConfig {
    pub project: ProjectInfo,
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub dev_dependencies: Vec<Dependency>,
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
    #[serde(default = "default_python_version")]
    pub python_version: String,
    #[serde(default = "default_parallel_downloads")]
    pub parallel_downloads: usize,
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
}

impl Default for CobraToolConfig {
    fn default() -> Self {
        Self {
            python_version: default_python_version(),
            parallel_downloads: default_parallel_downloads(),
            cache_enabled: default_cache_enabled(),
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
        // Remove if already exists
        self.dependencies.retain(|d| d.name != name);
        
        self.dependencies.push(Dependency {
            name: name.to_string(),
            version_spec: version.to_string(),
        });
    }

    pub fn remove_dependency(&mut self, name: &str) -> bool {
        let len_before = self.dependencies.len();
        self.dependencies.retain(|d| d.name != name);
        len_before != self.dependencies.len()
    }

    pub fn get_dependency(&self, name: &str) -> Option<&Dependency> {
        self.dependencies.iter().find(|d| d.name == name)
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
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            tool: ToolConfig::default(),
        }
    }
}
