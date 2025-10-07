use crate::{Result, CobraError, Package};
use reqwest::Client;

/// PackageCloud.io registry implementation (for custom/private packages)
pub struct PackageCloudRegistry {
    client: Client,
    base_url: String,
    api_token: Option<String>,
}

impl PackageCloudRegistry {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://packagecloud.io".to_string(),
            api_token: None,
        }
    }

    pub fn with_token(token: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://packagecloud.io".to_string(),
            api_token: Some(token),
        }
    }

    pub fn with_custom_url(url: String, token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: url,
            api_token: token,
        }
    }

    pub async fn get_package(&self, repo: &str, package_name: &str) -> Result<Package> {
        let url = format!("{}/api/v1/repos/{}/package/python/{}.json", 
            self.base_url, repo, package_name);

        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.api_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(CobraError::PackageNotFound(package_name.to_string()));
        }

        let json: serde_json::Value = response.json().await?;
        
        // Parse PackageCloud response format
        let name = json["name"].as_str()
            .ok_or_else(|| CobraError::PackageNotFound(package_name.to_string()))?
            .to_string();
        
        let version = json["version"].as_str()
            .ok_or_else(|| CobraError::PackageNotFound(package_name.to_string()))?
            .to_string();

        let download_url = json["download_url"].as_str()
            .ok_or_else(|| CobraError::PackageNotFound(package_name.to_string()))?
            .to_string();

        Ok(Package {
            name,
            version,
            dependencies: Vec::new(),
            download_url,
            hash: None,
            size: None,
        })
    }

    pub async fn list_packages(&self, repo: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/v1/repos/{}/packages.json", self.base_url, repo);

        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.api_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = response.json().await?;
        let mut packages = Vec::new();

        if let Some(array) = json.as_array() {
            for item in array {
                if let Some(name) = item["name"].as_str() {
                    packages.push(name.to_string());
                }
            }
        }

        Ok(packages)
    }
}

impl Default for PackageCloudRegistry {
    fn default() -> Self {
        Self::new()
    }
}
