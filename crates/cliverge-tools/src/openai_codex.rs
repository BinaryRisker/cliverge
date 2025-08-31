use cliverge_sdk::{async_trait, CliTool, ToolError, CommandOutput, InstallConfig, ToolStatus};
use cliverge_sdk::helpers::{execute_command, command_exists};

pub struct OpenAiCodexTool {
    name: String,
    version: String,
}

impl OpenAiCodexTool {
    pub fn new() -> Self {
        Self {
            name: "OpenAI CodeX CLI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl CliTool for OpenAiCodexTool {
    fn id(&self) -> &str { "openai-codex" }
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn detect(&self) -> Result<bool, ToolError> {
        Ok(command_exists("codex"))
    }

    async fn install(&self, _config: &InstallConfig) -> Result<(), ToolError> {
        if self.detect().await? { return Ok(()); }

        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["bucket".to_string(), "add".to_string(), "openai".to_string(), "https://github.com/openai/scoop-bucket".to_string()], None).await?;
          execute_command("scoop", &["install".to_string(), "openai-codex".to_string()], None).await?; }

        #[cfg(target_os = "macos")]
        { execute_command("brew", &["tap".to_string(), "openai/homebrew-codex".to_string()], None).await?;
          execute_command("brew", &["install".to_string(), "openai-codex".to_string()], None).await?; }

        #[cfg(target_os = "linux")]
        { execute_command("pip", &["install".to_string(), "openai-codex-cli".to_string()], None).await
            .or_else(|_| execute_command("snap", &["install".to_string(), "openai-codex".to_string()], None))?; }

        Ok(())
    }

    async fn update(&self, _to_version: &str) -> Result<(), ToolError> {
        execute_command("codex", &["self-update".to_string()], None).await.map(|_| ()).map_err(|_| ToolError::UpdateFailed("Update failed".to_string()))
    }

    async fn uninstall(&self) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["uninstall".to_string(), "openai-codex".to_string()], None).await?; }
        #[cfg(target_os = "macos")]
        { execute_command("brew", &["uninstall".to_string(), "openai-codex".to_string()], None).await?; }
        #[cfg(target_os = "linux")]
        { execute_command("pip", &["uninstall".to_string(), "openai-codex-cli".to_string()], None).await
            .or_else(|_| execute_command("snap", &["remove".to_string(), "openai-codex".to_string()], None))?; }
        Ok(())
    }

    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError> {
        if !self.detect().await? { return Err(ToolError::NotFound("OpenAI CodeX CLI not installed".to_string())); }
        execute_command("codex", args, None).await
    }

    fn help(&self) -> String {
        format!("{} - OpenAI CodeX Code Generator\n\nCommands:\n- codex generate \"Create React component\" - Generate code\n- codex complete --file main.py --line 25 - Complete code\n- codex explain --file algorithm.cpp - Explain code\n- codex translate --from python --to rust - Translate code", self.name)
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "api_key": {"type": "string", "description": "OpenAI API key", "secret": true},
                "model": {"type": "string", "description": "Model to use", "default": "gpt-4", "enum": ["gpt-4", "gpt-3.5-turbo", "codex"]}
            },
            "required": ["api_key"]
        }))
    }

    async fn status(&self) -> Result<ToolStatus, ToolError> {
        if !command_exists("codex") { return Ok(ToolStatus::NotInstalled); }
        match execute_command("codex", &["--version".to_string()], None).await {
            Ok(output) if output.exit_code == 0 => Ok(ToolStatus::Installed { version: "1.0.0".to_string() }),
            _ => Ok(ToolStatus::Error("Cannot get version".to_string())),
        }
    }
}

impl Default for OpenAiCodexTool {
    fn default() -> Self { Self::new() }
}
