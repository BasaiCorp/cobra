use crate::{Result, CobraError, Package, Dependency, constants::*};
use reqwest::{Client, ClientBuilder, Response};
use std::time::Duration;

/// High-performance HTTP client with connection pooling and HTTP/2
pub struct RegistryClient {
    client: Client,
    pypi_base_url: String,
}

impl RegistryClient {
    pub fn new() -> Self {
        let client = Self::create_optimized_client();
        Self {
            client,
            pypi_base_url: "https://pypi.org".to_string(),
        }
    }

    /// Create optimized HTTP client with aggressive performance settings
    fn create_optimized_client() -> Client {
        ClientBuilder::new()
            .pool_max_idle_per_host(32)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .http1_title_case_headers()
            .timeout(HTTP_TIMEOUT)
            .user_agent("cobra/1.0 (blazingly-fast-python-package-manager)")
            .gzip(true)
            .brotli(true)
            .build()
            .expect("Failed to create HTTP client")
    }

    /// Get package information from PyPI
    pub async fn get_package_info(&self, name: &str, version_spec: &str) -> Result<Package> {
        let url = if version_spec == "*" || version_spec.is_empty() {
            format!("{}/pypi/{}/json", self.pypi_base_url, name)
        } else {
            // For specific versions, strip operators like ==, >=, etc.
            let version = version_spec.trim_start_matches("==")
                .trim_start_matches(">=")
                .trim_start_matches("<=")
                .trim_start_matches("~=")
                .trim_start_matches("^")
                .trim();
            format!("{}/pypi/{}/{}/json", self.pypi_base_url, name, version)
        };

        let response = self.client.get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CobraError::PackageNotFound(name.to_string()));
        }

        let json: serde_json::Value = response.json().await?;
        
        // Parse package info
        let info = &json["info"];
        let version = info["version"].as_str()
            .ok_or_else(|| CobraError::PackageNotFound(format!("Invalid package data for {}", name)))?
            .to_string();

        // Get download URL for wheel file (prefer wheels over source)
        let urls = &json["urls"];
        let mut download_url = String::new();
        let mut size = None;
        let mut hash = None;

        if let Some(urls_array) = urls.as_array() {
            // Prefer wheel files
            for url_info in urls_array {
                if url_info["packagetype"].as_str() == Some("bdist_wheel") {
                    download_url = url_info["url"].as_str().unwrap_or("").to_string();
                    size = url_info["size"].as_u64();
                    if let Some(digests) = url_info["digests"].as_object() {
                        hash = digests.get("sha256")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                    break;
                }
            }

            // Fallback to source distribution
            if download_url.is_empty() {
                for url_info in urls_array {
                    if url_info["packagetype"].as_str() == Some("sdist") {
                        download_url = url_info["url"].as_str().unwrap_or("").to_string();
                        size = url_info["size"].as_u64();
                        if let Some(digests) = url_info["digests"].as_object() {
                            hash = digests.get("sha256")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        break;
                    }
                }
            }
        }

        if download_url.is_empty() {
            return Err(CobraError::PackageNotFound(
                format!("No download URL found for {}", name)
            ));
        }

        // Parse dependencies
        let mut dependencies = Vec::new();
        if let Some(requires_dist) = info["requires_dist"].as_array() {
            for dep in requires_dist {
                if let Some(dep_str) = dep.as_str() {
                    if let Some((dep_name, dep_version)) = parse_dependency(dep_str) {
                        dependencies.push(Dependency {
                            name: dep_name,
                            version_spec: dep_version,
                        });
                    }
                }
            }
        }

        // Extract additional metadata
        let description = info["summary"].as_str().map(|s| s.to_string());
        let author = info["author"].as_str().map(|s| s.to_string());
        let homepage = info["home_page"].as_str()
            .or_else(|| info["project_url"].as_str())
            .map(|s| s.to_string());

        Ok(Package {
            name: name.to_string(),
            version,
            dependencies,
            download_url,
            hash,
            size,
            description,
            author,
            homepage,
        })
    }

    /// Download package file
    pub async fn download_package(&self, url: &str) -> Result<Response> {
        let response = self.client.get(url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CobraError::InstallationFailed(
                format!("Failed to download: {}", response.status())
            ));
        }

        Ok(response)
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse dependency string like "requests (>=2.0.0)" into (name, version_spec)
fn parse_dependency(dep_str: &str) -> Option<(String, String)> {
    // Skip extras and environment markers
    let dep_str = dep_str.split(';').next()?.trim();
    let dep_str = dep_str.split('[').next()?.trim();

    if let Some(pos) = dep_str.find('(') {
        let name = dep_str[..pos].trim().to_string();
        let version = dep_str[pos+1..].trim_end_matches(')').trim().to_string();
        Some((name, version))
    } else if dep_str.contains("==") || dep_str.contains(">=") || dep_str.contains("<=") {
        // Handle inline version specs
        for op in &["==", ">=", "<=", "~=", "!="] {
            if let Some(pos) = dep_str.find(op) {
                let name = dep_str[..pos].trim().to_string();
                let version = dep_str[pos..].trim().to_string();
                return Some((name, version));
            }
        }
        None
    } else {
        Some((dep_str.to_string(), "*".to_string()))
    }
}
