//! SDK for developing CLIverge plugins

use serde::{Deserialize, Serialize};

// Re-export async-trait for convenience
pub use async_trait::async_trait;

/// Errors that can occur when working with CLI tools
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Installation failed: {0}")]
    InstallationFailed(String),
    #[error("Update failed: {0}")]
    UpdateFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}

/// Output from executing a command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Configuration for tool installation
#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub sources: Vec<String>,
    pub verify: Option<bool>,
    pub post_install: Option<Vec<String>>,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            sources: vec![],
            verify: Some(true),
            post_install: None,
        }
    }
}

/// Status of a CLI tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolStatus {
    Loading,
    NotInstalled,
    Installed { version: String },
    Error(String),
}

impl Default for ToolStatus {
    fn default() -> Self {
        ToolStatus::Loading
    }
}

/// Core trait that all CLI tool adapters must implement
#[async_trait]
pub trait CliTool: Send + Sync {
    /// Unique identifier for the tool
    fn id(&self) -> &str;
    
    /// Human-readable name of the tool
    fn name(&self) -> &str;
    
    /// Version of the adapter (not the tool itself)
    fn version(&self) -> &str;
    
    /// Check if the tool is installed on the system
    async fn detect(&self) -> Result<bool, ToolError>;
    
    /// Install the tool
    async fn install(&self, config: &InstallConfig) -> Result<(), ToolError>;
    
    /// Update the tool to a specific version
    async fn update(&self, to_version: &str) -> Result<(), ToolError>;
    
    /// Uninstall the tool
    async fn uninstall(&self) -> Result<(), ToolError>;
    
    /// Execute a command with the tool
    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError>;
    
    /// Get help text for the tool
    fn help(&self) -> String;
    
    /// Get configuration schema for the tool (JSON Schema)
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }
    
    /// Get current status of the tool
    async fn status(&self) -> Result<ToolStatus, ToolError>;
}

/// Helper functions for common operations
pub mod helpers {
    use super::{CommandOutput, ToolError};
    use std::process::Stdio;
    use tokio::process::Command;
    
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    
    /// Execute a command and return the output
    pub async fn execute_command(
        program: &str,
        args: &[String],
        stdin: Option<&str>,
    ) -> Result<CommandOutput, ToolError> {
        let mut cmd;
        
        // On Windows, use cmd /c to properly handle npm-installed commands
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            
            cmd = Command::new("cmd");
            let mut cmd_args = vec!["/c".to_string(), program.to_string()];
            cmd_args.extend_from_slice(args);
            cmd.args(&cmd_args);
            
            // Use CREATE_NO_WINDOW to hide command prompt windows
            // This is specifically for GUI applications
            let exe_name = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_default();
            
            // Only use CREATE_NO_WINDOW for GUI applications
            if exe_name.contains("gui") {
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            cmd = Command::new(program);
            cmd.args(args);
        }
        
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }
        
        let mut child = cmd.spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to start {}: {}", program, e)))?;
        
        // Write stdin if provided
        if let Some(input) = stdin {
            if let Some(mut stdin_handle) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin_handle.write_all(input.as_bytes()).await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write stdin: {}", e)))?;
                stdin_handle.shutdown().await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to close stdin: {}", e)))?;
            }
        }
        
        let output = child.wait_with_output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to wait for {}: {}", program, e)))?;
        
        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
    
    /// Check if a command exists on the system
    pub fn command_exists(command: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // Use cmd /c where to avoid PowerShell alias issues
            // Use CREATE_NO_WINDOW only for existence checking to reduce popup
            std::process::Command::new("cmd")
                .args(["/c", "where", command])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW flag for this specific check
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .map(|output| output.status.success() && !output.stdout.is_empty())
                .unwrap_or(false)
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("which")
                .arg(command)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
    }
    
    /// Download a file from a URL
    pub async fn download_file(url: &str, path: &str) -> Result<(), ToolError> {
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;
        
        let response = reqwest::get(url).await
            .map_err(|e| ToolError::Other(format!("Failed to download {}: {}", url, e)))?;
            
        let bytes = response.bytes().await
            .map_err(|e| ToolError::Other(format!("Failed to read response: {}", e)))?;
            
        let mut file = File::create(path).await
            .map_err(|e| ToolError::Io(e))?;
            
        file.write_all(&bytes).await
            .map_err(|e| ToolError::Io(e))?;
            
        Ok(())
    }
}
