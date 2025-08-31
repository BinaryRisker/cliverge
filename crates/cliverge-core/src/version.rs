//! Version checking functionality for CLI tools

use crate::{ToolConfig, ToolError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, error, warn};

#[cfg(windows)]
use std::os::windows::process::CommandExt;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    pub check_method: String,
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionCheckStrategy {
    Auto,           // Automatically select best method
    SelfCheck,      // Use tool's own update check
    PackageManager, // Use package manager
    LocalDatabase,  // Use local version database
}

impl Default for VersionCheckStrategy {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Clone)]
pub struct VersionChecker {
    local_db: Option<VersionDatabase>,
}

impl VersionChecker {
    pub fn new() -> Self {
        let local_db = VersionDatabase::load().ok();
        Self { local_db }
    }

    /// Create a command configured to run hidden on Windows
    #[cfg(windows)]
    fn create_hidden_command(command: &str, args: &[String]) -> Command {
        let mut powershell_args = vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
        ];
        
        // Construct the full command as a string
        let full_command = format!("{} {}", command, args.join(" "));
        powershell_args.push(full_command);
        
        let mut cmd = Command::new("powershell.exe");
        cmd.args(&powershell_args);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd
    }

    /// Create a command configured to run normally on non-Windows
    #[cfg(not(windows))]
    fn create_hidden_command(command: &str, args: &[String]) -> Command {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd
    }

    /// Main entry point for version checking
    pub async fn check_version(
        &self,
        tool_config: &ToolConfig,
        strategy: VersionCheckStrategy,
    ) -> Result<VersionInfo, ToolError> {
        debug!("Checking version for tool: {} with strategy: {:?}", tool_config.id, strategy);

        let result = match strategy {
            VersionCheckStrategy::SelfCheck => self.check_via_self_update(tool_config).await,
            VersionCheckStrategy::PackageManager => self.check_via_package_manager(tool_config).await,
            VersionCheckStrategy::LocalDatabase => self.check_via_local_database(tool_config).await,
            VersionCheckStrategy::Auto => self.auto_check(tool_config).await,
        };

        match result {
            Ok(mut version_info) => {
                version_info.last_checked = chrono::Utc::now();
                Ok(version_info)
            }
            Err(e) => {
                error!("Version check failed for {}: {}", tool_config.id, e);
                Err(e)
            }
        }
    }

    /// Get current version of installed tool
    pub async fn get_current_version(&self, tool_config: &ToolConfig) -> Result<String, ToolError> {
        debug!("Getting current version for: {}", tool_config.id);

        let mut cmd = Self::create_hidden_command(&tool_config.command, &tool_config.version_check);
        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute version check: {}", e)))?;

        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            let version = Self::parse_version_string(&version_str);
            debug!("Current version for {}: {}", tool_config.id, version);
            Ok(version)
        } else {
            let error_str = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::ExecutionFailed(format!("Version check command failed: {}", error_str)))
        }
    }

    /// Auto-select best checking method
    async fn auto_check(&self, tool_config: &ToolConfig) -> Result<VersionInfo, ToolError> {
        debug!("Auto-selecting version check method for: {}", tool_config.id);

        // Priority 1: Tool's own update check
        if let Some(update_check) = &tool_config.update_check {
            if !update_check.is_empty() {
                if let Ok(result) = self.check_via_self_update(tool_config).await {
                    debug!("Auto-check succeeded with self-update method");
                    return Ok(result);
                }
                warn!("Self-update check failed for {}, trying package manager", tool_config.id);
            }
        }

        // Priority 2: Package manager
        if let Ok(result) = self.check_via_package_manager(tool_config).await {
            debug!("Auto-check succeeded with package manager method");
            return Ok(result);
        }
        warn!("Package manager check failed for {}, trying local database", tool_config.id);

        // Priority 3: Local database
        self.check_via_local_database(tool_config).await
    }

    /// Use tool's own update checking mechanism
    async fn check_via_self_update(&self, tool_config: &ToolConfig) -> Result<VersionInfo, ToolError> {
        debug!("Checking version via self-update for: {}", tool_config.id);

        let current = self.get_current_version(tool_config).await.ok();

        let latest = if let Some(update_cmd) = &tool_config.update_check {
            if !update_cmd.is_empty() {
                let mut cmd = if cfg!(windows) {
                    // On Windows, use hidden PowerShell execution
                    Self::create_hidden_command(&update_cmd[0], &update_cmd[1..])
                } else {
                    let mut cmd = Command::new(&update_cmd[0]);
                    cmd.args(&update_cmd[1..]);
                    cmd
                };
                
                let output = cmd.output()
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Update check failed: {}", e)))?;

                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    Some(Self::parse_latest_version_from_output(&output_str))
                } else {
                    warn!("Update check command failed for {}", tool_config.id);
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let update_available = Self::compare_versions(&current, &latest);

        Ok(VersionInfo {
            current,
            latest,
            update_available,
            check_method: "self-check".to_string(),
            last_checked: chrono::Utc::now(),
        })
    }

    /// Check version via package manager
    async fn check_via_package_manager(&self, tool_config: &ToolConfig) -> Result<VersionInfo, ToolError> {
        debug!("Checking version via package manager for: {}", tool_config.id);

        let current = self.get_current_version(tool_config).await.ok();
        let platform = std::env::consts::OS;

        let install_config = tool_config
            .install
            .get(platform)
            .ok_or_else(|| ToolError::NotSupported(format!("Platform {} not supported", platform)))?;

        let latest = match install_config.method.as_str() {
            "npm" => self.check_npm_version(tool_config).await.ok(),
            "brew" => self.check_brew_version(tool_config).await.ok(),
            "pip" => self.check_pip_version(tool_config).await.ok(),
            _ => {
                warn!("Unsupported package manager: {}", install_config.method);
                None
            }
        };

        let update_available = Self::compare_versions(&current, &latest);

        Ok(VersionInfo {
            current,
            latest,
            update_available,
            check_method: format!("package-manager-{}", install_config.method),
            last_checked: chrono::Utc::now(),
        })
    }

    /// Check version via local database
    async fn check_via_local_database(&self, tool_config: &ToolConfig) -> Result<VersionInfo, ToolError> {
        debug!("Checking version via local database for: {}", tool_config.id);

        let current = self.get_current_version(tool_config).await.ok();
        let latest = if let Some(db) = &self.local_db {
            db.get_latest_version(&tool_config.id).map(|s| s.to_string())
        } else {
            None
        };

        let update_available = Self::compare_versions(&current, &latest);

        Ok(VersionInfo {
            current,
            latest,
            update_available,
            check_method: "local-database".to_string(),
            last_checked: chrono::Utc::now(),
        })
    }

    /// Check NPM package version
    async fn check_npm_version(&self, tool_config: &ToolConfig) -> Result<String, ToolError> {
        let package_name = Self::extract_package_name_from_config(tool_config, "npm")?;

        let output = Command::new("npm")
            .args(&["view", &package_name, "version"])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("NPM command failed: {}", e)))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(version)
        } else {
            Err(ToolError::NotFound(format!("NPM package {} not found", package_name)))
        }
    }

    /// Check Homebrew formula version
    async fn check_brew_version(&self, tool_config: &ToolConfig) -> Result<String, ToolError> {
        let formula_name = Self::extract_package_name_from_config(tool_config, "brew")?;

        let output = Command::new("brew")
            .args(&["info", &formula_name, "--json=v1"])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Brew command failed: {}", e)))?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let json: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| ToolError::ParseError(format!("Failed to parse brew JSON: {}", e)))?;

            if let Some(versions) = json.as_array() {
                if let Some(first_formula) = versions.first() {
                    if let Some(versions_obj) = first_formula.get("versions") {
                        if let Some(stable) = versions_obj.get("stable") {
                            if let Some(version) = stable.as_str() {
                                return Ok(version.to_string());
                            }
                        }
                    }
                }
            }
        }

        Err(ToolError::NotFound(format!("Brew formula {} not found", formula_name)))
    }

    /// Check pip package version
    async fn check_pip_version(&self, tool_config: &ToolConfig) -> Result<String, ToolError> {
        let package_name = Self::extract_package_name_from_config(tool_config, "pip")?;

        let output = Command::new("pip")
            .args(&["index", "versions", &package_name])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Pip command failed: {}", e)))?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Parse pip output to extract latest version
            // Format: "package (x.y.z)"
            if let Some(version) = Self::parse_pip_version_output(&output_str) {
                Ok(version)
            } else {
                Err(ToolError::ParseError("Failed to parse pip version output".to_string()))
            }
        } else {
            Err(ToolError::NotFound(format!("Pip package {} not found", package_name)))
        }
    }

    /// Extract package name from tool config
    fn extract_package_name_from_config(tool_config: &ToolConfig, method: &str) -> Result<String, ToolError> {
        let platform = std::env::consts::OS;
        let install_config = tool_config
            .install
            .get(platform)
            .ok_or_else(|| ToolError::NotSupported(format!("Platform {} not supported", platform)))?;

        if install_config.method != method {
            return Err(ToolError::NotSupported(format!("Method {} not supported", method)));
        }

        // Try to get package name from config first
        if let Some(package_name) = &install_config.package_name {
            return Ok(package_name.clone());
        }

        // Fallback: extract from install command
        if let Some(command) = &install_config.command {
            if let Some(package_name) = command.last() {
                return Ok(package_name.clone());
            }
        }

        Err(ToolError::ConfigError(format!("No package name found for {}", tool_config.id)))
    }

    /// Parse version string from command output
    fn parse_version_string(output: &str) -> String {
        // Common version patterns
        let patterns = [
            regex::Regex::new(r"v?(\d+\.\d+\.\d+(?:\.\d+)?)").unwrap(),
            regex::Regex::new(r"version\s+v?(\d+\.\d+\.\d+(?:\.\d+)?)").unwrap(),
            regex::Regex::new(r"(\d+\.\d+\.\d+(?:\.\d+)?)").unwrap(),
        ];

        for pattern in &patterns {
            if let Some(captures) = pattern.captures(output) {
                if let Some(version) = captures.get(1) {
                    return version.as_str().to_string();
                }
            }
        }

        "unknown".to_string()
    }

    /// Parse latest version from update check output
    fn parse_latest_version_from_output(output: &str) -> String {
        // Look for common update messages
        if output.contains("up to date") || output.contains("already latest") {
            return "current".to_string();
        }

        // Look for "update available" messages with version numbers
        let patterns = [
            regex::Regex::new(r"update.*available.*v?(\d+\.\d+\.\d+(?:\.\d+)?)").unwrap(),
            regex::Regex::new(r"new.*version.*v?(\d+\.\d+\.\d+(?:\.\d+)?)").unwrap(),
            regex::Regex::new(r"latest.*v?(\d+\.\d+\.\d+(?:\.\d+)?)").unwrap(),
        ];

        for pattern in &patterns {
            if let Some(captures) = pattern.captures(output) {
                if let Some(version) = captures.get(1) {
                    return version.as_str().to_string();
                }
            }
        }

        Self::parse_version_string(output)
    }

    /// Parse pip version output
    fn parse_pip_version_output(output: &str) -> Option<String> {
        // pip index versions output format varies, try to parse common formats
        let lines: Vec<&str> = output.lines().collect();
        for line in &lines {
            if line.contains("Available versions:") {
                if let Some(version_line) = lines.iter().find(|l| l.trim().starts_with(char::is_numeric)) {
                    return Some(version_line.trim().split_whitespace().next()?.to_string());
                }
            }
        }
        None
    }

    /// Compare versions and determine if update is available
    fn compare_versions(current: &Option<String>, latest: &Option<String>) -> bool {
        match (current, latest) {
            (Some(current), Some(latest)) => {
                if latest == "current" {
                    false
                } else {
                    Self::is_version_newer(latest, current)
                }
            }
            _ => false,
        }
    }

    /// Simple semantic version comparison
    fn is_version_newer(new_version: &str, current_version: &str) -> bool {
        let parse_version = |v: &str| -> Vec<u32> {
            v.trim_start_matches('v')
                .split('.')
                .filter_map(|part| part.parse::<u32>().ok())
                .collect()
        };

        let new_parts = parse_version(new_version);
        let current_parts = parse_version(current_version);

        for (new_part, current_part) in new_parts.iter().zip(current_parts.iter()) {
            match new_part.cmp(current_part) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => continue,
            }
        }

        new_parts.len() > current_parts.len()
    }
}

/// Local version database for offline version checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDatabase {
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub tools: HashMap<String, ToolVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolVersionInfo {
    pub latest_version: String,
    pub release_date: chrono::DateTime<chrono::Utc>,
    pub download_url: Option<String>,
    pub changelog_url: Option<String>,
}

impl VersionDatabase {
    pub fn load() -> Result<Self, ToolError> {
        let path = Self::get_database_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| ToolError::ConfigError(format!("Failed to read version database: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| ToolError::ConfigError(format!("Failed to parse version database: {}", e)))
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), ToolError> {
        let path = Self::get_database_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::ConfigError(format!("Failed to create config directory: {}", e)))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ToolError::ConfigError(format!("Failed to serialize version database: {}", e)))?;

        std::fs::write(&path, json)
            .map_err(|e| ToolError::ConfigError(format!("Failed to write version database: {}", e)))
    }

    pub fn get_latest_version(&self, tool_id: &str) -> Option<&str> {
        self.tools.get(tool_id).map(|info| info.latest_version.as_str())
    }

    pub fn update_tool_version(&mut self, tool_id: String, version_info: ToolVersionInfo) {
        self.tools.insert(tool_id, version_info);
        self.last_updated = chrono::Utc::now();
    }

    pub fn is_stale(&self, max_age_days: i64) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.last_updated);
        age.num_days() > max_age_days
    }

    fn get_database_path() -> Result<PathBuf, ToolError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| ToolError::ConfigError("Could not find config directory".to_string()))?;

        Ok(config_dir.join("cliverge").join("version_database.json"))
    }
}

impl Default for VersionDatabase {
    fn default() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            tools: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(VersionChecker::parse_version_string("v1.2.3"), "1.2.3");
        assert_eq!(VersionChecker::parse_version_string("version 2.1.0"), "2.1.0");
        assert_eq!(VersionChecker::parse_version_string("foo 1.0.0-beta"), "1.0.0");
        assert_eq!(VersionChecker::parse_version_string("no version here"), "unknown");
    }

    #[test]
    fn test_version_comparison() {
        assert!(VersionChecker::is_version_newer("1.2.3", "1.2.2"));
        assert!(VersionChecker::is_version_newer("1.3.0", "1.2.9"));
        assert!(VersionChecker::is_version_newer("2.0.0", "1.9.9"));
        assert!(!VersionChecker::is_version_newer("1.2.2", "1.2.3"));
        assert!(!VersionChecker::is_version_newer("1.2.3", "1.2.3"));
    }
}
