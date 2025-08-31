use cliverge_sdk::{async_trait, CliTool, ToolError, CommandOutput, InstallConfig, ToolStatus};
use cliverge_sdk::helpers::{execute_command, command_exists};

pub struct GeminiCliTool {
    name: String,
    version: String,
}

impl GeminiCliTool {
    pub fn new() -> Self {
        Self {
            name: "Gemini CLI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl CliTool for GeminiCliTool {
    fn id(&self) -> &str {
        "gemini-cli"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn detect(&self) -> Result<bool, ToolError> {
        Ok(command_exists("gemini"))
    }

    async fn install(&self, _config: &InstallConfig) -> Result<(), ToolError> {
        if self.detect().await? {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let result = execute_command("winget", &["install".to_string(), "Google.GeminiCLI".to_string()], None).await;
            if result.is_err() {
                println!("Installing Gemini CLI via Scoop...");
                execute_command("scoop", &["bucket".to_string(), "add".to_string(), "google".to_string(), "https://github.com/google/scoop-bucket".to_string()], None).await?;
                execute_command("scoop", &["install".to_string(), "gemini-cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "macos")]
        {
            execute_command("brew", &["tap".to_string(), "google/gemini".to_string()], None).await?;
            execute_command("brew", &["install".to_string(), "gemini-cli".to_string()], None).await?;
        }

        #[cfg(target_os = "linux")]
        {
            let install_script = "curl -fsSL https://dl.google.com/gemini/cli/install.sh | sudo sh";
            execute_command("sh", &["-c".to_string(), install_script.to_string()], None).await?;
        }

        Ok(())
    }

    async fn update(&self, _to_version: &str) -> Result<(), ToolError> {
        #[cfg(target_os = "macos")]
        {
            execute_command("brew", &["upgrade".to_string(), "gemini-cli".to_string()], None).await?;
        }

        #[cfg(target_os = "windows")]
        {
            let result = execute_command("winget", &["upgrade".to_string(), "Google.GeminiCLI".to_string()], None).await;
            if result.is_err() {
                execute_command("scoop", &["update".to_string(), "gemini-cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            let result = execute_command("gemini", &["self-update".to_string()], None).await;
            if result.is_err() {
                // Reinstall if self-update fails
                self.uninstall().await?;
                self.install(&InstallConfig::default()).await?;
            }
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), ToolError> {
        #[cfg(target_os = "macos")]
        {
            execute_command("brew", &["uninstall".to_string(), "gemini-cli".to_string()], None).await?;
        }

        #[cfg(target_os = "windows")]
        {
            let result = execute_command("winget", &["uninstall".to_string(), "Google.GeminiCLI".to_string()], None).await;
            if result.is_err() {
                execute_command("scoop", &["uninstall".to_string(), "gemini-cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            execute_command("sudo", &["rm".to_string(), "/usr/local/bin/gemini".to_string()], None).await?;
        }

        Ok(())
    }

    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError> {
        if !self.detect().await? {
            return Err(ToolError::NotFound("Gemini CLI not installed".to_string()));
        }

        execute_command("gemini", args, None).await
    }

    fn help(&self) -> String {
        format!(
            "{} - Google Gemini AI Assistant\n\n\
            Common Commands:\n\
            - gemini code \"<prompt>\" - Generate code\n\
            - gemini explain --file <file> - Explain code\n\
            - gemini review --path <path> - Review code\n\
            - gemini test --file <file> - Generate tests\n\n\
            Environment Variables:\n\
            - GEMINI_API_KEY: Your Google API key (required)\n\
            - GEMINI_MODEL: Model to use (default: gemini-pro)\n\n\
            For more information, run: gemini --help",
            self.name
        )
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "api_key": {
                    "type": "string",
                    "description": "Google Gemini API key",
                    "secret": true
                },
                "model": {
                    "type": "string",
                    "description": "Gemini model to use",
                    "default": "gemini-pro",
                    "enum": ["gemini-pro", "gemini-pro-vision"]
                }
            },
            "required": ["api_key"]
        }))
    }

    async fn status(&self) -> Result<ToolStatus, ToolError> {
        if !command_exists("gemini") {
            return Ok(ToolStatus::NotInstalled);
        }

        match execute_command("gemini", &["--version".to_string()], None).await {
            Ok(output) if output.exit_code == 0 => {
                let version = output.stdout.trim().split_whitespace()
                    .last()
                    .unwrap_or("unknown")
                    .to_string();
                Ok(ToolStatus::Installed { version })
            }
            _ => Ok(ToolStatus::Error("Cannot get version".to_string())),
        }
    }
}

impl Default for GeminiCliTool {
    fn default() -> Self {
        Self::new()
    }
}
