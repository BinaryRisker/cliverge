use cliverge_sdk::{async_trait, CliTool, ToolError, CommandOutput, InstallConfig, ToolStatus};
use cliverge_sdk::helpers::{execute_command, command_exists};

pub struct QwenCodeTool {
    name: String,
    version: String,
}

impl QwenCodeTool {
    pub fn new() -> Self {
        Self {
            name: "Qwen Code CLI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl CliTool for QwenCodeTool {
    fn id(&self) -> &str { "qwen-code-cli" }
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn detect(&self) -> Result<bool, ToolError> {
        Ok(command_exists("qwen"))
    }

    async fn install(&self, _config: &InstallConfig) -> Result<(), ToolError> {
        if self.detect().await? { return Ok(()); }

        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["bucket".to_string(), "add".to_string(), "alibaba".to_string(), "https://github.com/alibaba/scoop-bucket".to_string()], None).await?;
          execute_command("scoop", &["install".to_string(), "qwen-code-cli".to_string()], None).await?; }

        #[cfg(target_os = "macos")]
        { execute_command("brew", &["tap".to_string(), "alibaba/qwen".to_string()], None).await?;
          execute_command("brew", &["install".to_string(), "qwen-code-cli".to_string()], None).await?; }

        #[cfg(target_os = "linux")]
        { execute_command("sh", &["-c".to_string(), "curl -fsSL https://qwen-cli.aliyun.com/install.sh | sudo sh".to_string()], None).await?; }

        Ok(())
    }

    async fn update(&self, _to_version: &str) -> Result<(), ToolError> {
        execute_command("qwen", &["self-update".to_string()], None).await.map(|_| ()).map_err(|_| ToolError::UpdateFailed("Update failed".to_string()))
    }

    async fn uninstall(&self) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        { execute_command("scoop", &["uninstall".to_string(), "qwen-code-cli".to_string()], None).await?; }
        #[cfg(target_os = "macos")]
        { execute_command("brew", &["uninstall".to_string(), "qwen-code-cli".to_string()], None).await?; }
        #[cfg(target_os = "linux")]
        { execute_command("sudo", &["rm".to_string(), "/usr/local/bin/qwen".to_string()], None).await?; }
        Ok(())
    }

    async fn execute(&self, args: &[String]) -> Result<CommandOutput, ToolError> {
        if !self.detect().await? { return Err(ToolError::NotFound("Qwen Code CLI not installed".to_string())); }
        execute_command("qwen", args, None).await
    }

    fn help(&self) -> String {
        format!("{} - 阿里云通义千问代码助手\n\n常用命令:\n- qwen code \"生成REST API\" - 生成代码\n- qwen explain --file main.go - 解释代码\n- qwen review --path src - 代码审查", self.name)
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "api_key": {"type": "string", "description": "阿里云API密钥", "secret": true}
            },
            "required": ["api_key"]
        }))
    }

    async fn status(&self) -> Result<ToolStatus, ToolError> {
        if !command_exists("qwen") { return Ok(ToolStatus::NotInstalled); }
        match execute_command("qwen", &["--version".to_string()], None).await {
            Ok(output) if output.exit_code == 0 => Ok(ToolStatus::Installed { version: "1.0.0".to_string() }),
            _ => Ok(ToolStatus::Error("Cannot get version".to_string())),
        }
    }
}

impl Default for QwenCodeTool {
    fn default() -> Self { Self::new() }
}
