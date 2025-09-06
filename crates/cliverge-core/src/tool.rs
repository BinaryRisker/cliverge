//! Tool management functionality

use crate::{ConfigManager, ToolConfig, ToolError, VersionChecker, VersionCheckStrategy, VersionInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tracing::{debug, warn};



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

        // Execute installation command
        if let Some(command) = &install_config.command {
            self.execute_install_command(command).await?;
        } else {
            // Fallback: construct command from method and package_name
            let command = self.construct_install_command(&install_config)?;
            self.execute_install_command(&command).await?;
        }

        // Clear status cache to force re-check
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.remove(tool_id);
        }

        debug!("Tool {} installed successfully", tool_id);
        Ok(())
    }

    /// Construct install command from method and package name
    fn construct_install_command(&self, install_config: &crate::InstallMethod) -> Result<Vec<String>, ToolError> {
        match install_config.method.as_str() {
            "npm" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["npm".to_string(), "install".to_string(), "-g".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("npm install requires package_name".to_string()))
                }
            }
            "brew" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["brew".to_string(), "install".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("brew install requires package_name".to_string()))
                }
            }
            "pip" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["pip".to_string(), "install".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("pip install requires package_name".to_string()))
                }
            }
            "apt" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["sudo".to_string(), "apt".to_string(), "install".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("apt install requires package_name".to_string()))
                }
            }
            "yum" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["sudo".to_string(), "yum".to_string(), "install".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("yum install requires package_name".to_string()))
                }
            }
            "dnf" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["sudo".to_string(), "dnf".to_string(), "install".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("dnf install requires package_name".to_string()))
                }
            }
            "pacman" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["sudo".to_string(), "pacman".to_string(), "-S".to_string(), "--noconfirm".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("pacman install requires package_name".to_string()))
                }
            }
            "winget" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["winget".to_string(), "install".to_string(), package_name.clone(), "--accept-source-agreements".to_string(), "--accept-package-agreements".to_string()])
                } else {
                    Err(ToolError::ConfigError("winget install requires package_name".to_string()))
                }
            }
            "choco" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["choco".to_string(), "install".to_string(), package_name.clone(), "-y".to_string()])
                } else {
                    Err(ToolError::ConfigError("choco install requires package_name".to_string()))
                }
            }
            "scoop" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["scoop".to_string(), "install".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("scoop install requires package_name".to_string()))
                }
            }
            "cargo" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["cargo".to_string(), "install".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("cargo install requires package_name".to_string()))
                }
            }
            "go" => {
                if let Some(package_name) = &install_config.package_name {
                    Ok(vec!["go".to_string(), "install".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("go install requires package_name".to_string()))
                }
            }
            method => Err(ToolError::NotSupported(format!("Install method '{}' not supported", method)))
        }
    }

    /// Execute install command with proper error handling
    async fn execute_install_command(&self, command: &[String]) -> Result<(), ToolError> {
        if command.is_empty() {
            return Err(ToolError::ConfigError("Empty install command".to_string()));
        }

        debug!("Executing install command: {:?}", command);
        
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(ToolError::ExecutionFailed(format!(
                "Install command failed with exit code {}: stdout: {}, stderr: {}", 
                output.status.code().unwrap_or(-1), 
                stdout, 
                stderr
            )));
        }

        Ok(())
    }

    /// Uninstall a tool
    pub async fn uninstall_tool(&self, tool_id: &str) -> Result<(), ToolError> {
        debug!("Uninstalling tool: {}", tool_id);

        let (tool_config, uninstall_config) = {
            let config_manager = self.config_manager.lock().unwrap();
            let tool_config = config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone();

            let platform = std::env::consts::OS;
            
            // 首先尝试使用专门的卸载配置
            let uninstall_config = if let Some(uninstall_configs) = &tool_config.uninstall {
                uninstall_configs.get(platform).cloned()
            } else {
                None
            };

            // 如果没有专门的卸载配置，回退到安装配置
            let uninstall_config = uninstall_config.or_else(|| {
                tool_config.install.get(platform).cloned()
            }).ok_or_else(|| ToolError::NotSupported(format!("Platform {} not supported for {}", platform, tool_id)))?;

            (tool_config, uninstall_config)
        };

        if !self.is_tool_installed(&tool_config).await {
            debug!("Tool {} is not installed", tool_id);
            return Ok(());
        }

        // Execute uninstall command
        if let Some(command) = &uninstall_config.command {
            self.execute_uninstall_command(command).await?;
        } else {
            // Fallback: construct command from method and package_name
            let command = self.construct_uninstall_command(&uninstall_config)?;
            self.execute_uninstall_command(&command).await?;
        }

        // Clear status cache to force re-check
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.remove(tool_id);
        }

        debug!("Tool {} uninstalled successfully", tool_id);
        Ok(())
    }

    /// Construct uninstall command from method and package name
    fn construct_uninstall_command(&self, uninstall_config: &crate::InstallMethod) -> Result<Vec<String>, ToolError> {
        match uninstall_config.method.as_str() {
            "npm" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["npm".to_string(), "uninstall".to_string(), "-g".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("npm uninstall requires package_name".to_string()))
                }
            }
            "brew" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["brew".to_string(), "uninstall".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("brew uninstall requires package_name".to_string()))
                }
            }
            "pip" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["pip".to_string(), "uninstall".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("pip uninstall requires package_name".to_string()))
                }
            }
            "apt" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["sudo".to_string(), "apt".to_string(), "remove".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("apt remove requires package_name".to_string()))
                }
            }
            "yum" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["sudo".to_string(), "yum".to_string(), "remove".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("yum remove requires package_name".to_string()))
                }
            }
            "dnf" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["sudo".to_string(), "dnf".to_string(), "remove".to_string(), "-y".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("dnf remove requires package_name".to_string()))
                }
            }
            "pacman" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["sudo".to_string(), "pacman".to_string(), "-R".to_string(), "--noconfirm".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("pacman remove requires package_name".to_string()))
                }
            }
            "winget" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["winget".to_string(), "uninstall".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("winget uninstall requires package_name".to_string()))
                }
            }
            "choco" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["choco".to_string(), "uninstall".to_string(), package_name.clone(), "-y".to_string()])
                } else {
                    Err(ToolError::ConfigError("choco uninstall requires package_name".to_string()))
                }
            }
            "scoop" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["scoop".to_string(), "uninstall".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("scoop uninstall requires package_name".to_string()))
                }
            }
            "cargo" => {
                if let Some(package_name) = &uninstall_config.package_name {
                    Ok(vec!["cargo".to_string(), "uninstall".to_string(), package_name.clone()])
                } else {
                    Err(ToolError::ConfigError("cargo uninstall requires package_name".to_string()))
                }
            }
            method => Err(ToolError::NotSupported(format!("Uninstall method '{}' not supported", method)))
        }
    }

    /// Execute uninstall command with proper error handling
    async fn execute_uninstall_command(&self, command: &[String]) -> Result<(), ToolError> {
        if command.is_empty() {
            return Err(ToolError::ConfigError("Empty uninstall command".to_string()));
        }

        debug!("Executing uninstall command: {:?}", command);
        
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(ToolError::ExecutionFailed(format!(
                "Uninstall command failed with exit code {}: stdout: {}, stderr: {}", 
                output.status.code().unwrap_or(-1), 
                stdout, 
                stderr
            )));
        }

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

    /// Check for version updates with upgrade information
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

    /// Check if updates are available for a tool
    pub async fn has_updates_available(&self, tool_id: &str) -> Result<bool, ToolError> {
        let version_info = self.check_version_updates(tool_id, VersionCheckStrategy::Auto).await?;
        
        // Compare current version with latest version
        if let (Some(current), Some(latest)) = (&version_info.current, &version_info.latest) {
            Ok(Self::compare_versions(current, latest) == std::cmp::Ordering::Less)
        } else {
            Ok(false)
        }
    }

    /// Get help information for a tool
    pub async fn get_tool_help(&self, tool_id: &str) -> Result<String, ToolError> {
        let tool_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager
                .get_tool_config(tool_id)
                .ok_or_else(|| ToolError::NotFound(format!("Tool {} not found", tool_id)))?
                .clone()
        };

        // Try to get help using common help commands
        let help_commands = vec![
            vec!["--help".to_string()],
            vec!["help".to_string()],
            vec!["-h".to_string()],
        ];

        for help_cmd in help_commands {
            match self.execute_tool_command(&tool_config.command, &help_cmd).await {
                Ok(output) => {
                    if output.status.success() {
                        let help_text = String::from_utf8_lossy(&output.stdout).to_string();
                        if !help_text.trim().is_empty() {
                            return Ok(help_text);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Err(ToolError::ExecutionFailed(format!("Could not get help information for {}", tool_id)))
    }

    /// Check all tools for available updates
    pub async fn check_all_updates(&self) -> Result<Vec<(String, bool)>, ToolError> {
        let tools_config = {
            let config_manager = self.config_manager.lock().unwrap();
            config_manager.get_tools_config().clone()
        };

        let mut update_results = Vec::new();
        
        for tool_config in &tools_config.tools {
            match self.has_updates_available(&tool_config.id).await {
                Ok(has_updates) => {
                    update_results.push((tool_config.id.clone(), has_updates));
                }
                Err(e) => {
                    warn!("Failed to check updates for {}: {}", tool_config.id, e);
                    update_results.push((tool_config.id.clone(), false));
                }
            }
        }

        Ok(update_results)
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
        if let Some(update_check_configs) = &tool_config.update_check {
            let platform = std::env::consts::OS;
            if let Some(update_cmd) = update_check_configs.get(platform) {
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
        let platform = std::env::consts::OS;
        if let Some(version_check_args) = tool_config.version_check.get(platform) {
            let mut cmd = Self::create_hidden_command(&tool_config.command, version_check_args);
            
            match cmd.output().await {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        } else {
            false
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

    /// Execute a tool command directly
    async fn execute_tool_command(&self, command: &str, args: &[String]) -> Result<std::process::Output, ToolError> {
        let mut cmd = Self::create_hidden_command(command, args);
        
        cmd.output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))
    }

    /// Compare two semantic version strings
    fn compare_versions(current: &str, latest: &str) -> std::cmp::Ordering {
        // Simple version comparison - in production you'd want a proper semver library
        let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
        let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();
        
        let max_len = std::cmp::max(current_parts.len(), latest_parts.len());
        
        for i in 0..max_len {
            let current_part = current_parts.get(i).unwrap_or(&0);
            let latest_part = latest_parts.get(i).unwrap_or(&0);
            
            match current_part.cmp(latest_part) {
                std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
                std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
                std::cmp::Ordering::Equal => continue,
            }
        }
        
        std::cmp::Ordering::Equal
    }
}

