use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use cliverge_sdk::ToolStatus;
use chrono::{DateTime, Utc};

/// Cached information about a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToolInfo {
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub latest_version: Option<String>,
    pub cached_at: DateTime<Utc>,
    pub last_checked: DateTime<Utc>,
}

/// Cache data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCache {
    pub version: u32,
    pub tools: HashMap<String, CachedToolInfo>,
    pub last_updated: DateTime<Utc>,
}

impl Default for ToolCache {
    fn default() -> Self {
        Self {
            version: 1,
            tools: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

impl ToolCache {
    /// Get the cache file path
    pub fn get_cache_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("cliverge");
        path.push("cache.json");
        path
    }

    /// Load cache from disk
    pub async fn load() -> Option<Self> {
        let path = Self::get_cache_path();
        
        if !path.exists() {
            return None;
        }

        match fs::read_to_string(&path).await {
            Ok(contents) => {
                match serde_json::from_str::<ToolCache>(&contents) {
                    Ok(cache) => {
                        // Check if cache is expired (24 hours)
                        let now = Utc::now();
                        let age = now.signed_duration_since(cache.last_updated);
                        
                        if age.num_hours() < 24 {
                            Some(cache)
                        } else {
                            None // Cache is expired
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse cache: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read cache file: {}", e);
                None
            }
        }
    }

    /// Save cache to disk
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::get_cache_path();
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json).await?;
        
        Ok(())
    }

    /// Update tool information in cache
    pub fn update_tool(&mut self, tool_info: CachedToolInfo) {
        self.tools.insert(tool_info.id.clone(), tool_info);
        self.last_updated = Utc::now();
    }

    /// Get cached tool info
    pub fn get_tool(&self, tool_id: &str) -> Option<&CachedToolInfo> {
        self.tools.get(tool_id)
    }

    /// Check if a specific tool's cache is stale (older than 1 hour)
    pub fn is_tool_stale(&self, tool_id: &str) -> bool {
        if let Some(tool) = self.tools.get(tool_id) {
            let age = Utc::now().signed_duration_since(tool.last_checked);
            age.num_hours() >= 1
        } else {
            true
        }
    }
}

/// Module for checking latest versions from package registries
pub mod version_checker {
    use super::*;
    
    /// Check latest version from npm registry
    pub async fn check_npm_version(package_name: &str) -> Option<String> {
        let url = format!("https://registry.npmjs.org/{}/latest", package_name);
        
        match reqwest::get(&url).await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            json["version"].as_str().map(|v| v.to_string())
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Check latest version from PyPI
    pub async fn check_pypi_version(package_name: &str) -> Option<String> {
        let url = format!("https://pypi.org/pypi/{}/json", package_name);
        
        match reqwest::get(&url).await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            json["info"]["version"].as_str().map(|v| v.to_string())
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Check latest version from GitHub releases
    pub async fn check_github_version(owner: &str, repo: &str) -> Option<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );
        
        let client = reqwest::Client::new();
        match client
            .get(&url)
            .header("User-Agent", "cliverge-gui")
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            json["tag_name"].as_str().map(|v| {
                                // Remove 'v' prefix if present
                                v.trim_start_matches('v').to_string()
                            })
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Get latest version for a specific tool
    pub async fn get_latest_version(tool_id: &str) -> Option<String> {
        match tool_id {
            "claude-code" => {
                // Claude might be distributed via npm or GitHub
                check_npm_version("@anthropic/claude-cli").await
            }
            "gemini-cli" => {
                check_npm_version("@google/gemini-cli").await
            }
            "qwen-code-cli" => {
                check_pypi_version("qwen-code").await
            }
            "openai-codex" => {
                check_npm_version("openai-cli").await
            }
            "opencode" => {
                check_github_version("opencode", "opencode-cli").await
            }
            "iflow" => {
                check_npm_version("iflow-cli").await
            }
            _ => None,
        }
    }
}
