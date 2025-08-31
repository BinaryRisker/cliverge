use cliverge_sdk::{async_trait, CliTool, ToolError, CommandOutput, InstallConfig, ToolStatus};
use cliverge_sdk::helpers::{execute_command, command_exists};

pub struct OpenCodeTool {
    name: String,
    version: String,
}

impl OpenCodeTool {
    pub fn new() -> Self {
        Self {
            name: "OpenCode CLI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl CliTool for OpenCodeTool {
    fn id(&self) -> &str { "opencode" }
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn detect(&self) -> Result<bool, ToolError> {
        Ok(command_exists("opencode"))
    }

    async fn install(&self, _config: &InstallConfig) -> Result<(), ToolError> {
        if self.detect().await? { return Ok(()); }

        #[cfg(target_os = "windows")]
        {
            if execute_command("winget", &["install".to_string(), "OpenCode.CLI".to_string()], None).await.is_err() {
                execute_command("npm", &["install".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if execute_command("brew", &["install".to_string(), "opencode/tap/opencode".to_string()], None).await.is_err() {
                execute_command("npm", &["install".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if execute_command("snap", &["install".to_string(), "opencode".to_string()], None).await.is_err() {
                execute_command("npm", &["install".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }

        Ok(())
    }

    async fn update(&self, _to_version: &str) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        {
            if execute_command("winget", &["upgrade".to_string(), "OpenCode.CLI".to_string()], None).await.is_err() {
                execute_command("npm", &["update".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if execute_command("brew", &["upgrade".to_string(), "opencode/tap/opencode".to_string()], None).await.is_err() {
                execute_command("npm", &["update".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if execute_command("snap", &["refresh".to_string(), "opencode".to_string()], None).await.is_err() {
                execute_command("npm", &["update".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        {
            if execute_command("winget", &["uninstall".to_string(), "OpenCode.CLI".to_string()], None).await.is_err() {
                execute_command("npm", &["uninstall".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }
        #[cfg(target_os = "macos")]
        {
            if execute_command("brew", &["uninstall".to_string(), "opencode/tap/opencode".to_string()], None).await.is_err() {
                execute_command("npm", &["uninstall".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }
        #[cfg(target_os = "linux")]
        {
            if execute_command("snap", &["remove".to_string(), "opencode".to_string()], None).await.is_err() {
                execute_command("npm", &["uninstall".to_string(), "-g".to_string(), "@opencode/cli".to_string()], None).await?;
            }
        }
        Ok(())
    }

    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError> {
        if !self.detect().await? { return Err(ToolError::NotFound("OpenCode CLI not installed".to_string())); }
        execute_command("opencode", args, None).await
    }

    fn help(&self) -> String {
        format!("{} - OpenCode Development Platform\n\nCommands:\n- opencode init - Initialize new project\n- opencode generate --template react - Generate code from templates\n- opencode review --file src/main.rs - Review code quality\n- opencode test --auto-generate - Generate unit tests\n- opencode refactor --pattern singleton - Refactor code patterns", self.name)
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "token": {"type": "string", "description": "OpenCode access token", "secret": true},
                "workspace": {"type": "string", "description": "Default workspace ID"},
                "template_dir": {"type": "string", "description": "Custom template directory path"},
                "auto_format": {"type": "boolean", "description": "Enable auto-formatting", "default": true}
            },
            "required": ["token"]
        }))
    }

    async fn status(&self) -> Result<ToolStatus, ToolError> {
        if !command_exists("opencode") { return Ok(ToolStatus::NotInstalled); }
        match execute_command("opencode", &["--version".to_string()], None).await {
            Ok(output) if output.exit_code == 0 => {
                let version = output.stdout.lines().next().unwrap_or("1.0.0").trim();
                Ok(ToolStatus::Installed { version: version.to_string() })
            },
            _ => Ok(ToolStatus::Error("Cannot get version".to_string())),
        }
    }
}

impl Default for OpenCodeTool {
    fn default() -> Self { Self::new() }
}
