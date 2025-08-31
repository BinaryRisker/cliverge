//! Tool management functionality

use crate::{ConfigManager, ToolConfig, ToolError, VersionChecker, VersionCheckStrategy, VersionInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tracing::{debug, warn};

#[cfg(windows)]
use std::os::windows::process::CommandExt;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolStatus {
    Unknown,
    NotInstalled,
    Installed { version: String },
    Error(String),
}

impl Default for ToolStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub config: ToolConfig,
    pub status: ToolStatus,
    pub version_info: Option<VersionInfo>,
    pub user_config: HashMap<String, serde_json::Value>,
}

#[derive(Clone)]
pub struct ToolManager {
    config_manager: Arc<Mutex<ConfigManager>>,
    version_checker: VersionChecker,
    status_cache: Arc<Mutex<HashMap<String, ToolStatus>>>,
}

impl ToolManager {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self {
            config_manager,
            version_checker: VersionChecker::new(),
            status_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get all available tools with full status checking (blocking)
    pub async fn get_all_tools(&self) -> Result<Vec<ToolInfo>, ToolError> {
        let tools_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager.get_tools_config().clone()
        };
        let mut tool_infos = Vec::new();

        for tool_config in &tools_config.tools {
            let status = self.get_tool_status(&tool_config.id).await.unwrap_or_default();
            let version_info = self.get_cached_version_info(&tool_config.id).await;

            tool_infos.push(ToolInfo {
                config: tool_config.clone(),
                status,
                version_info,
                user_config: HashMap::new(), // TODO: Load user configuration
            });
        }

        Ok(tool_infos)
    }

    /// Get all tool configs immediately without status checking (non-blocking)
    pub fn get_all_tools_configs(&self) -> Result<Vec<ToolInfo>, ToolError> {
        let tools_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager.get_tools_config().clone()
        };
        let mut tool_infos = Vec::new();

        for tool_config in &tools_config.tools {
            tool_infos.push(ToolInfo {
                config: tool_config.clone(),
                status: ToolStatus::Unknown,  // Initial status, will be updated by background tasks
                version_info: None,
                user_config: HashMap::new(), // TODO: Load user configuration
            });
        }

        Ok(tool_infos)
    }

    /// Get specific tool by ID
    pub async fn get_tool(&self, tool_id: &str) -> Result<ToolInfo, ToolError> {
        let tool_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone()
        };

        let status = self.get_tool_status(tool_id).await.unwrap_or_default();
        let version_info = self.get_cached_version_info(tool_id).await;

        Ok(ToolInfo {
            config: tool_config,
            status,
            version_info,
            user_config: HashMap::new(), // TODO: Load user configuration
        })
    }

    /// Check tool installation status
    pub async fn check_tool_status(&self, tool_id: &str) -> Result<ToolStatus, ToolError> {
        debug!("Checking status for tool: {}", tool_id);

        let tool_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone()
        };

        let status = if self.is_tool_installed(&tool_config).await {
            match self.get_tool_version(&tool_config).await {
                Ok(version) => ToolStatus::Installed { version },
                Err(e) => {
                    warn!("Failed to get version for {}: {}", tool_id, e);
                    ToolStatus::Error("Version check failed".to_string())
                }
            }
        } else {
            ToolStatus::NotInstalled
        };

        // Cache the status
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.insert(tool_id.to_string(), status.clone());
        }

        Ok(status)
    }

    /// Install a tool
    pub async fn install_tool(&self, tool_id: &str) -> Result<(), ToolError> {
        debug!("Installing tool: {}", tool_id);

        let (tool_config, install_config) = {
            let config_manager = self.config_manager.lock().unwrap();
            let tool_config = config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone();

            let platform = std::env::consts::OS;
            let install_config = tool_config
                .install
                .get(platform)
                .ok_or_else(|| ToolError::NotSupported(format!("Platform {} not supported for {}", platform, tool_id)))?
                .clone();

            (tool_config, install_config)
        };

        // Check if already installed
        if self.is_tool_installed(&tool_config).await {
            debug!("Tool {} is already installed", tool_id);
            return Ok(());
        }

        match install_config.method.as_str() {
            "npm" | "brew" | "pip" => {
                if let Some(command) = &install_config.command {
                    self.execute_install_command(command).await?;
                } else {
                    return Err(ToolError::ConfigError("Install command not specified".to_string()));
                }
            }
            "script" => {
                if let Some(url) = &install_config.url {
                    self.execute_install_script(url).await?;
                } else {
                    return Err(ToolError::ConfigError("Install script URL not specified".to_string()));
                }
            }
            _ => {
                return Err(ToolError::NotSupported(format!("Install method {} not supported", install_config.method)));
            }
        }

        // Verify installation
        if !self.is_tool_installed(&tool_config).await {
            return Err(ToolError::InstallationFailed(format!("Installation verification failed for {}", tool_id)));
        }

        // Clear status cache to force refresh
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.remove(tool_id);
        }

        debug!("Tool {} installed successfully", tool_id);
        Ok(())
    }

    /// Uninstall a tool
    pub async fn uninstall_tool(&self, tool_id: &str) -> Result<(), ToolError> {
        debug!("Uninstalling tool: {}", tool_id);

        let (tool_config, install_config) = {
            let config_manager = self.config_manager.lock().unwrap();
            let tool_config = config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone();

            let platform = std::env::consts::OS;
            let install_config = tool_config
                .install
                .get(platform)
                .ok_or_else(|| ToolError::NotSupported(format!("Platform {} not supported for {}", platform, tool_id)))?
                .clone();

            (tool_config, install_config)
        };

        if !self.is_tool_installed(&tool_config).await {
            debug!("Tool {} is not installed", tool_id);
            return Ok(());
        }

        // Generate uninstall command based on install method
        let uninstall_command = match install_config.method.as_str() {
            "npm" => {
                if let Some(package_name) = &install_config.package_name {
                    vec!["npm".to_string(), "uninstall".to_string(), "-g".to_string(), package_name.clone()]
                } else {
                    return Err(ToolError::ConfigError("NPM package name not specified".to_string()));
                }
            }
            "brew" => {
                if let Some(package_name) = &install_config.package_name {
                    vec!["brew".to_string(), "uninstall".to_string(), package_name.clone()]
                } else {
                    return Err(ToolError::ConfigError("Brew formula name not specified".to_string()));
                }
            }
            "pip" => {
                if let Some(package_name) = &install_config.package_name {
                    vec!["pip".to_string(), "uninstall".to_string(), "-y".to_string(), package_name.clone()]
                } else {
                    return Err(ToolError::ConfigError("Pip package name not specified".to_string()));
                }
            }
            _ => {
                return Err(ToolError::NotSupported(format!("Uninstall method {} not supported", install_config.method)));
            }
        };

        self.execute_install_command(&uninstall_command).await?;

        // Clear status cache to force refresh
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.remove(tool_id);
        }

        debug!("Tool {} uninstalled successfully", tool_id);
        Ok(())
    }

    /// Execute a tool command
    pub async fn execute_tool(&self, tool_id: &str, args: &[String]) -> Result<std::process::Output, ToolError> {
        debug!("Executing tool: {} with args: {:?}", tool_id, args);

        let tool_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone()
        };

        if !self.is_tool_installed(&tool_config).await {
            return Err(ToolError::NotFound(format!("Tool {} is not installed", tool_id)));
        }

        let mut cmd = Self::create_hidden_command(&tool_config.command, args);
        
        let output = cmd.output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute {}: {}", tool_id, e)))?;

        Ok(output)
    }

    /// Check for version updates
    pub async fn check_version_updates(&self, tool_id: &str, strategy: VersionCheckStrategy) -> Result<VersionInfo, ToolError> {
        let tool_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone()
        };

        self.version_checker.check_version(&tool_config, strategy).await
    }

    /// Update tool to latest version
    pub async fn update_tool(&self, tool_id: &str) -> Result<(), ToolError> {
        debug!("Updating tool: {}", tool_id);

        let (tool_config, install_config) = {
            let config_manager = self.config_manager.lock().unwrap();
            let tool_config = config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone();

            let platform = std::env::consts::OS;
            let install_config = tool_config
                .install
                .get(platform)
                .ok_or_else(|| ToolError::NotSupported(format!("Platform {} not supported for {}", platform, tool_id)))?
                .clone();

            (tool_config, install_config)
        };

        if !self.is_tool_installed(&tool_config).await {
            return Err(ToolError::NotFound(format!("Tool {} is not installed", tool_id)));
        }

        // Try self-update first if available
        if let Some(update_cmd) = &tool_config.update_check {
            let self_update_cmd: Vec<String> = update_cmd.iter()
                .map(|s| if s == "--check-only" { "--update".to_string() } else { s.clone() })
                .collect();

            match self.execute_install_command(&self_update_cmd).await {
                Ok(_) => {
                    debug!("Tool {} updated via self-update", tool_id);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Self-update failed for {}: {}, trying package manager", tool_id, e);
                }
            }
        }

        // Fallback to package manager update
        let update_command = match install_config.method.as_str() {
            "npm" => {
                if let Some(package_name) = &install_config.package_name {
                    vec!["npm".to_string(), "update".to_string(), "-g".to_string(), package_name.clone()]
                } else {
                    return Err(ToolError::ConfigError("NPM package name not specified".to_string()));
                }
            }
            "brew" => {
                if let Some(package_name) = &install_config.package_name {
                    vec!["brew".to_string(), "upgrade".to_string(), package_name.clone()]
                } else {
                    return Err(ToolError::ConfigError("Brew formula name not specified".to_string()));
                }
            }
            "pip" => {
                if let Some(package_name) = &install_config.package_name {
                    vec!["pip".to_string(), "install".to_string(), "--upgrade".to_string(), package_name.clone()]
                } else {
                    return Err(ToolError::ConfigError("Pip package name not specified".to_string()));
                }
            }
            _ => {
                return Err(ToolError::NotSupported(format!("Update method {} not supported", install_config.method)));
            }
        };

        self.execute_install_command(&update_command).await?;

        // Clear status cache to force refresh
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.remove(tool_id);
        }

        debug!("Tool {} updated successfully", tool_id);
        Ok(())
    }

    /// Refresh all tool statuses
    pub async fn refresh_all_status(&self) -> Result<(), ToolError> {
        debug!("Refreshing all tool statuses");

        let tools_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager.get_tools_config().clone()
        };

        for tool_config in &tools_config.tools {
            if let Err(e) = self.check_tool_status(&tool_config.id).await {
                warn!("Failed to check status for {}: {}", tool_config.id, e);
            }
        }

        Ok(())
    }

    /// Update cached status for a specific tool
    pub fn update_cached_status(&self, tool_id: &str, status: ToolStatus) {
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.insert(tool_id.to_string(), status);
        }
    }

    /// Get cached status for a specific tool without performing new checks
    pub fn get_cached_status(&self, tool_id: &str) -> Option<ToolStatus> {
        if let Ok(cache) = self.status_cache.lock() {
            cache.get(tool_id).cloned()
        } else {
            None
        }
    }

    // Private helper methods

    /// Create a command configured for hidden execution on Windows
    fn create_hidden_command(command: &str, args: &[String]) -> Command {
        if cfg!(windows) {
            let mut powershell_args = vec![
                "-NoProfile".to_string(),
                "-WindowStyle".to_string(),
                "Hidden".to_string(),
                "-Command".to_string(),
            ];
            
            // Construct the full command as a string
            let full_command = if args.is_empty() {
                command.to_string()
            } else {
                format!("{} {}", command, args.join(" "))
            };
            powershell_args.push(full_command);
            
            let mut cmd = Command::new("powershell.exe");
            cmd.args(&powershell_args);
            
            // Use CREATE_NO_WINDOW flag to prevent console window
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            cmd
        } else {
            let mut cmd = Command::new(command);
            cmd.args(args);
            cmd
        }
    }

    async fn is_tool_installed(&self, tool_config: &ToolConfig) -> bool {
        let mut cmd = Self::create_hidden_command(&tool_config.command, &tool_config.version_check);
        
        match cmd.output().await {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    async fn get_tool_version(&self, tool_config: &ToolConfig) -> Result<String, ToolError> {
        self.version_checker.get_current_version(tool_config).await
    }

    async fn get_tool_status(&self, tool_id: &str) -> Result<ToolStatus, ToolError> {
        // Check cache first
        if let Ok(cache) = self.status_cache.lock() {
            if let Some(cached_status) = cache.get(tool_id) {
                return Ok(cached_status.clone());
            }
        }

        // If not cached, perform status check without using the public method to avoid recursion
        let tool_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone()
        };

        let status = if self.is_tool_installed(&tool_config).await {
            match self.get_tool_version(&tool_config).await {
                Ok(version) => ToolStatus::Installed { version },
                Err(e) => {
                    warn!("Failed to get version for {}: {}", tool_id, e);
                    ToolStatus::Error("Version check failed".to_string())
                }
            }
        } else {
            ToolStatus::NotInstalled
        };

        // Cache the status
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.insert(tool_id.to_string(), status.clone());
        }

        Ok(status)
    }

    async fn get_cached_version_info(&self, _tool_id: &str) -> Option<VersionInfo> {
        // TODO: Implement version info caching
        None
    }

    async fn execute_install_command(&self, command: &[String]) -> Result<(), ToolError> {
        if command.is_empty() {
            return Err(ToolError::ConfigError("Empty install command".to_string()));
        }

        let mut cmd = Self::create_hidden_command(&command[0], &command[1..]);
        
        let output = cmd.output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute install command: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::InstallationFailed(format!("Install command failed: {}", error_msg)));
        }

        Ok(())
    }

    async fn execute_install_script(&self, script_url: &str) -> Result<(), ToolError> {
        // For now, we'll use a simple approach with curl + sh
        // In a production environment, you'd want more secure script handling
        let install_command = format!("curl -fsSL {} | sh", script_url);
        
        let output = Command::new("sh")
            .args(["-c", &install_command])
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute install script: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::InstallationFailed(format!("Install script failed: {}", error_msg)));
        }

        Ok(())
    }
}
