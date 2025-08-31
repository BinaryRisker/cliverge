use cliverge_sdk::{async_trait, CliTool, ToolError, CommandOutput, InstallConfig, ToolStatus};
use cliverge_sdk::helpers::{execute_command, command_exists};

pub struct IFlowTool {
    name: String,
    version: String,
}

impl IFlowTool {
    pub fn new() -> Self {
        Self {
            name: "iFlow CLI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl CliTool for IFlowTool {
    fn id(&self) -> &str { "iflow" }
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn detect(&self) -> Result<bool, ToolError> {
        Ok(command_exists("iflow"))
    }

    async fn install(&self, _config: &InstallConfig) -> Result<(), ToolError> {
        if self.detect().await? { return Ok(()); }

        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["bucket".to_string(), "add".to_string(), "iflow".to_string(), "https://github.com/iflow-ai/scoop-bucket".to_string()], None).await?;
          execute_command("scoop", &["install".to_string(), "iflow".to_string()], None).await?; }

        #[cfg(target_os = "macos")]
        { execute_command("brew", &["tap".to_string(), "iflow-ai/homebrew-iflow".to_string()], None).await?;
          execute_command("brew", &["install".to_string(), "iflow".to_string()], None).await?; }

        #[cfg(target_os = "linux")]
        { // Try different installation methods for Linux
          execute_command("curl", &["-fsSL".to_string(), "https://get.iflow.ai/install.sh".to_string()], None).await
            .and_then(|output| execute_command("bash", &["-s".to_string()], Some(&output.stdout)))
            .or_else(|_| execute_command("pip", &["install".to_string(), "iflow-cli".to_string()], None))
            .or_else(|_| execute_command("snap", &["install".to_string(), "iflow".to_string()], None))?; }

        Ok(())
    }

    async fn update(&self, _to_version: &str) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["update".to_string(), "iflow".to_string()], None).await?; }
        #[cfg(target_os = "macos")]
        { execute_command("brew", &["upgrade".to_string(), "iflow".to_string()], None).await?; }
        #[cfg(target_os = "linux")]
        { execute_command("iflow", &["self-update".to_string()], None).await
            .or_else(|_| execute_command("pip", &["install".to_string(), "--upgrade".to_string(), "iflow-cli".to_string()], None))
            .or_else(|_| execute_command("snap", &["refresh".to_string(), "iflow".to_string()], None))?; }
        
        Ok(())
    }

    async fn uninstall(&self) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["uninstall".to_string(), "iflow".to_string()], None).await?; }
        #[cfg(target_os = "macos")]
        { execute_command("brew", &["uninstall".to_string(), "iflow".to_string()], None).await?; }
        #[cfg(target_os = "linux")]
        { execute_command("pip", &["uninstall".to_string(), "iflow-cli".to_string()], None).await
            .or_else(|_| execute_command("snap", &["remove".to_string(), "iflow".to_string()], None))?; }
        Ok(())
    }

    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError> {
        if !self.detect().await? { return Err(ToolError::NotFound("iFlow CLI not installed".to_string())); }
        execute_command("iflow", args, None).await
    }

    fn help(&self) -> String {
        format!("{} - Intelligent Flow Management CLI\n\nCommands:\n- iflow create workflow - Create new workflow\n- iflow run --workflow deployment - Execute workflow\n- iflow status - Check workflow status\n- iflow logs --workflow test-suite - View workflow logs\n- iflow deploy --target production - Deploy workflows", self.name)
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "api_key": {"type": "string", "description": "iFlow API key", "secret": true},
                "endpoint": {"type": "string", "description": "iFlow server endpoint", "default": "https://api.iflow.ai"},
                "workspace": {"type": "string", "description": "Default workspace"},
                "timeout": {"type": "integer", "description": "Request timeout in seconds", "default": 30},
                "retry_attempts": {"type": "integer", "description": "Number of retry attempts", "default": 3}
            },
            "required": ["api_key"]
        }))
    }

    async fn status(&self) -> Result<ToolStatus, ToolError> {
        if !command_exists("iflow") { return Ok(ToolStatus::NotInstalled); }
        match execute_command("iflow", &["--version".to_string()], None).await {
            Ok(output) if output.exit_code == 0 => {
                let version = output.stdout.lines()
                    .find(|line| line.contains("iFlow"))
                    .and_then(|line| line.split_whitespace().last())
                    .unwrap_or("1.0.0");
                Ok(ToolStatus::Installed { version: version.to_string() })
            },
            _ => Ok(ToolStatus::Error("Cannot get version".to_string())),
        }
    }
}

impl Default for IFlowTool {
    fn default() -> Self { Self::new() }
}
