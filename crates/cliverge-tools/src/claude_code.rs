use cliverge_sdk::{async_trait, CliTool, ToolError, CommandOutput, InstallConfig, ToolStatus};
use cliverge_sdk::helpers::{execute_command, command_exists};
use std::path::Path;

pub struct ClaudeCodeTool {
    name: String,
    version: String,
}

impl ClaudeCodeTool {
    pub fn new() -> Self {
        Self {
            name: "Claude Code CLI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl CliTool for ClaudeCodeTool {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn detect(&self) -> Result<bool, ToolError> {
        // Check if 'claude' command exists in PATH
        if command_exists("claude") {
            // Try to get version to confirm it's working
            match execute_command("claude", &["--version".to_string()], None).await {
                Ok(output) => Ok(output.exit_code == 0),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    async fn install(&self, _config: &InstallConfig) -> Result<(), ToolError> {
        // For demonstration, we'll implement a simple installation process
        // In a real implementation, this would handle different installation methods
        
        // Check if already installed
        if self.detect().await? {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            // Windows installation
            let download_url = "https://releases.anthropic.com/claude-cli/latest/claude-cli-windows-x64.zip";
            let install_dir = std::env::var("LOCALAPPDATA")
                .map_err(|_| ToolError::InstallationFailed("Cannot find LOCALAPPDATA".to_string()))?;
            let install_path = Path::new(&install_dir).join("Programs").join("claude-cli");
            
            tokio::fs::create_dir_all(&install_path).await
                .map_err(|e| ToolError::InstallationFailed(format!("Failed to create install directory: {}", e)))?;
            
            // This is a placeholder - in reality you'd download and extract
            println!("Installing Claude Code CLI to {:?}", install_path);
            println!("Note: This is a demonstration. Real installation would download from {}", download_url);
        }

        #[cfg(target_os = "macos")]
        {
            // macOS installation via Homebrew
            let result = execute_command("brew", &["tap".to_string(), "anthropic/claude".to_string()], None).await?;
            if result.exit_code != 0 {
                return Err(ToolError::InstallationFailed("Failed to add Anthropic tap".to_string()));
            }
            
            let result = execute_command("brew", &["install".to_string(), "claude-cli".to_string()], None).await?;
            if result.exit_code != 0 {
                return Err(ToolError::InstallationFailed("Failed to install via Homebrew".to_string()));
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Linux installation
            let install_script = "curl -fsSL https://releases.anthropic.com/claude-cli/install.sh | sh";
            let result = execute_command("sh", &["-c".to_string(), install_script.to_string()], None).await?;
            if result.exit_code != 0 {
                return Err(ToolError::InstallationFailed("Failed to run install script".to_string()));
            }
        }

        Ok(())
    }

    async fn update(&self, to_version: &str) -> Result<(), ToolError> {
        // Check current version
        let current_status = self.status().await?;
        
        match current_status {
            ToolStatus::Installed { version } => {
                if version == to_version {
                    return Ok(()); // Already at target version
                }
                
                // For simplicity, we'll reinstall to update
                println!("Updating Claude Code CLI from {} to {}", version, to_version);
                
                // Use system update mechanism
                #[cfg(target_os = "macos")]
                {
                    let result = execute_command("brew", &["upgrade".to_string(), "claude-cli".to_string()], None).await?;
                    if result.exit_code != 0 {
                        return Err(ToolError::UpdateFailed("Failed to update via Homebrew".to_string()));
                    }
                }
                
                #[cfg(target_os = "windows")]
                {
                    // On Windows, reinstall
                    self.uninstall().await?;
                    self.install(&InstallConfig::default()).await?;
                }
                
                #[cfg(target_os = "linux")]
                {
                    // On Linux, use self-update if available
                    let result = execute_command("claude", &["self-update".to_string()], None).await;
                    if result.is_err() || result.unwrap().exit_code != 0 {
                        // Fallback to reinstall
                        self.uninstall().await?;
                        self.install(&InstallConfig::default()).await?;
                    }
                }
            }
            _ => return Err(ToolError::UpdateFailed("Tool not installed".to_string())),
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), ToolError> {
        if !self.detect().await? {
            return Ok(()); // Already uninstalled
        }

        #[cfg(target_os = "macos")]
        {
            let result = execute_command("brew", &["uninstall".to_string(), "claude-cli".to_string()], None).await?;
            if result.exit_code != 0 {
                return Err(ToolError::InstallationFailed("Failed to uninstall via Homebrew".to_string()));
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Remove from Windows
            let install_dir = std::env::var("LOCALAPPDATA")
                .map_err(|_| ToolError::InstallationFailed("Cannot find LOCALAPPDATA".to_string()))?;
            let install_path = Path::new(&install_dir).join("Programs").join("claude-cli");
            
            if install_path.exists() {
                tokio::fs::remove_dir_all(&install_path).await
                    .map_err(|e| ToolError::InstallationFailed(format!("Failed to remove install directory: {}", e)))?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            // On Linux, try to remove via package manager or manual removal
            if Path::new("/usr/local/bin/claude").exists() {
                let result = execute_command("sudo", &["rm".to_string(), "/usr/local/bin/claude".to_string()], None).await?;
                if result.exit_code != 0 {
                    return Err(ToolError::InstallationFailed("Failed to remove binary".to_string()));
                }
            }
        }

        Ok(())
    }

    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError> {
        if !self.detect().await? {
            return Err(ToolError::NotFound("Claude Code CLI not installed".to_string()));
        }

        execute_command("claude", args, None).await
    }

    fn help(&self) -> String {
        format!(
            "{} - Anthropic Claude AI Code Assistant\n\n\
            Common Commands:\n\
            - claude generate \"<prompt>\" - Generate code from natural language\n\
            - claude explain --file <file> - Explain code in a file\n\
            - claude refactor --file <file> - Refactor existing code\n\
            - claude fix --file <file> - Fix bugs in code\n\
            - claude review --file <file> - Review code quality\n\
            - claude test generate --file <file> - Generate unit tests\n\n\
            Environment Variables:\n\
            - ANTHROPIC_API_KEY: Your Anthropic API key (required)\n\
            - CLAUDE_MODEL: Model to use (default: claude-3-sonnet)\n\
            - CLAUDE_MAX_TOKENS: Maximum tokens per request (default: 4000)\n\n\
            For more information, run: claude --help", 
            self.name
        )
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "api_key": {
                    "type": "string",
                    "description": "Anthropic API key",
                    "secret": true
                },
                "model": {
                    "type": "string",
                    "description": "Claude model to use",
                    "default": "claude-3-sonnet",
                    "enum": ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"]
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens per request",
                    "default": 4000,
                    "minimum": 100,
                    "maximum": 100000
                },
                "temperature": {
                    "type": "number",
                    "description": "Control randomness in responses",
                    "default": 0.7,
                    "minimum": 0.0,
                    "maximum": 1.0
                }
            },
            "required": ["api_key"]
        }))
    }

    async fn status(&self) -> Result<ToolStatus, ToolError> {
        if !command_exists("claude") {
            return Ok(ToolStatus::NotInstalled);
        }

        match execute_command("claude", &["--version".to_string()], None).await {
            Ok(output) => {
                if output.exit_code == 0 {
                    // Parse version from output (format: "1.0.83 (Claude Code)")
                    let version_str = output.stdout.trim();
                    let version = version_str.split_whitespace()
                        .next()  // Get first part (the version number)
                        .unwrap_or("unknown")
                        .to_string();
                    
                    Ok(ToolStatus::Installed { version })
                } else {
                    Ok(ToolStatus::Error("Command failed".to_string()))
                }
            }
            Err(e) => Ok(ToolStatus::Error(e.to_string())),
        }
    }
}

impl Default for ClaudeCodeTool {
    fn default() -> Self {
        Self::new()
    }
}
