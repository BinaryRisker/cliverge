//! Configuration management for CLIverge

use crate::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// 类型别名以减少复杂度警告
type PlatformCommands = HashMap<String, Vec<String>>;
type PlatformMethods = HashMap<String, InstallMethod>;
type ConfigSchema = HashMap<String, ConfigField>;
type VersionCheckResult<E> = Result<PlatformCommands, E>;
type UpdateCheckResult<E> = Result<Option<PlatformCommands>, E>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub appearance: AppearanceSettings,
    pub behavior: BehaviorSettings,
    pub paths: PathSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: String,
    pub font_size: f32,
    pub window_size: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    pub auto_check_updates: bool,
    pub check_interval_minutes: u32,
    pub show_notifications: bool,
    pub auto_refresh_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSettings {
    pub tools_config_path: String,
    pub data_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub version: String,
    pub tools: Vec<ToolConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub website: String,
    pub command: String,
    #[serde(deserialize_with = "deserialize_version_check")]
    pub version_check: PlatformCommands, // 改为多平台支持
    #[serde(deserialize_with = "deserialize_update_check", default)]
    pub update_check: Option<PlatformCommands>, // 改为多平台支持
    pub install: PlatformMethods,
    pub uninstall: Option<PlatformMethods>, // 新增卸载配置
    pub update: Option<PlatformMethods>,    // 新增更新配置
    pub config_schema: Option<ConfigSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMethod {
    pub method: String,
    pub command: Option<Vec<String>>,
    pub url: Option<String>,
    pub package_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub field_type: String,
    pub secret: Option<bool>,
    pub required: Option<bool>,
    pub description: String,
    pub default: Option<serde_json::Value>,
    pub values: Option<Vec<String>>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            appearance: AppearanceSettings {
                theme: "dark".to_string(),
                font_size: 14.0,
                window_size: [1200.0, 800.0],
            },
            behavior: BehaviorSettings {
                auto_check_updates: true,
                check_interval_minutes: 30,
                show_notifications: true,
                auto_refresh_on_startup: true,
            },
            paths: PathSettings {
                tools_config_path: "tools.json".to_string(),
                data_directory: "~/.cliverge".to_string(),
            },
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            tools: Vec::new(),
        }
    }
}

pub struct ConfigManager {
    app_settings: AppSettings,
    tools_config: ToolsConfig,
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new_with_settings(app_settings: AppSettings) -> Self {
        Self {
            app_settings,
            tools_config: ToolsConfig::default(),
            config_dir: Self::get_config_dir(),
        }
    }

    pub async fn load() -> Result<Self, ConfigError> {
        let config_dir = Self::get_config_dir();
        tokio::fs::create_dir_all(&config_dir).await?;

        let app_settings = Self::load_app_settings(&config_dir).await?;
        let tools_config = Self::load_tools_config(&config_dir).await?;

        Ok(Self {
            app_settings,
            tools_config,
            config_dir,
        })
    }

    pub async fn save(&self) -> Result<(), ConfigError> {
        self.save_app_settings().await?;
        self.save_tools_config().await?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> &AppSettings {
        &self.app_settings
    }

    pub fn get_tools_config(&self) -> &ToolsConfig {
        &self.tools_config
    }

    pub fn get_tool_config(&self, id: &str) -> Option<&ToolConfig> {
        self.tools_config.tools.iter().find(|t| t.id == id)
    }

    pub fn update_app_settings(&mut self, settings: AppSettings) {
        self.app_settings = settings;
    }

    pub fn update_tool_config(&mut self, id: &str, config: ToolConfig) {
        if let Some(existing) = self.tools_config.tools.iter_mut().find(|t| t.id == id) {
            *existing = config;
        }
    }

    pub fn add_tool(&mut self, tool: ToolConfig) {
        self.tools_config.tools.push(tool);
    }

    pub fn remove_tool(&mut self, id: &str) {
        self.tools_config.tools.retain(|t| t.id != id);
    }

    pub fn set_tools_config(&mut self, tools_config: ToolsConfig) {
        self.tools_config = tools_config;
    }

    async fn load_app_settings(config_dir: &Path) -> Result<AppSettings, ConfigError> {
        let settings_path = config_dir.join("settings.json");

        if settings_path.exists() {
            let content = tokio::fs::read_to_string(&settings_path).await?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(AppSettings::default())
        }
    }

    async fn load_tools_config(config_dir: &Path) -> Result<ToolsConfig, ConfigError> {
        let tools_path = config_dir.join("tools.json");

        if tools_path.exists() {
            let content = tokio::fs::read_to_string(&tools_path).await?;
            Ok(serde_json::from_str(&content)?)
        } else {
            // Load default tools configuration from embedded data or create empty
            Ok(Self::create_default_tools_config())
        }
    }

    async fn save_app_settings(&self) -> Result<(), ConfigError> {
        let settings_path = self.config_dir.join("settings.json");
        let json = serde_json::to_string_pretty(&self.app_settings)?;
        tokio::fs::write(&settings_path, json).await?;
        Ok(())
    }

    async fn save_tools_config(&self) -> Result<(), ConfigError> {
        let tools_path = self.config_dir.join("tools.json");
        let json = serde_json::to_string_pretty(&self.tools_config)?;
        tokio::fs::write(&tools_path, json).await?;
        Ok(())
    }

    fn get_config_dir() -> PathBuf {
        // Use ~/.cliverge for all platforms for consistency
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cliverge")
    }

    fn create_default_tools_config() -> ToolsConfig {
        // Try to load from embedded default config first
        if let Ok(config) = Self::load_embedded_tools_config() {
            return config;
        }

        // Fallback to minimal config if embedded config fails
        ToolsConfig {
            version: "1.0".to_string(),
            tools: Vec::new(),
        }
    }

    fn load_embedded_tools_config() -> Result<ToolsConfig, ConfigError> {
        // Try to load from configs/tools.json in the project root
        let default_config_paths = [
            "./configs/tools.json",
            "../configs/tools.json",
            "../../configs/tools.json",
        ];

        for path in &default_config_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<ToolsConfig>(&content) {
                    tracing::debug!("Loaded default tools config from: {}", path);
                    return Ok(config);
                }
            }
        }
        Err(ConfigError::NotFound(
            "Default tools config not found".to_string(),
        ))
    }
}

// 自定义反序列化函数，支持向后兼容
fn deserialize_version_check<'de, D>(deserializer: D) -> VersionCheckResult<D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct VersionCheckVisitor;

    impl<'de> Visitor<'de> for VersionCheckVisitor {
        type Value = HashMap<String, Vec<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a Vec<String> or HashMap<String, Vec<String>>")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            // 旧格式：Vec<String>，转换为所有平台使用相同配置
            let mut vec = Vec::new();
            while let Some(item) = seq.next_element::<String>()? {
                vec.push(item);
            }

            let mut map = HashMap::new();
            map.insert("windows".to_string(), vec.clone());
            map.insert("macos".to_string(), vec.clone());
            map.insert("linux".to_string(), vec);
            Ok(map)
        }

        fn visit_map<A>(self, mut map_access: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            // 新格式：HashMap<String, Vec<String>>
            let mut map = HashMap::new();
            while let Some((key, value)) = map_access.next_entry::<String, Vec<String>>()? {
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    deserializer.deserialize_any(VersionCheckVisitor)
}

fn deserialize_update_check<'de, D>(deserializer: D) -> UpdateCheckResult<D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct UpdateCheckVisitor;

    impl<'de> Visitor<'de> for UpdateCheckVisitor {
        type Value = Option<HashMap<String, Vec<String>>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null, a Vec<String> or HashMap<String, Vec<String>>")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            // 旧格式：Vec<String>，转换为所有平台使用相同配置
            let mut vec = Vec::new();
            while let Some(item) = seq.next_element::<String>()? {
                vec.push(item);
            }

            if vec.is_empty() {
                return Ok(None);
            }

            let mut map = HashMap::new();
            map.insert("windows".to_string(), vec.clone());
            map.insert("macos".to_string(), vec.clone());
            map.insert("linux".to_string(), vec);
            Ok(Some(map))
        }

        fn visit_map<A>(self, mut map_access: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            // 新格式：HashMap<String, Vec<String>>
            let mut map = HashMap::new();
            while let Some((key, value)) = map_access.next_entry::<String, Vec<String>>()? {
                map.insert(key, value);
            }
            Ok(Some(map))
        }
    }

    deserializer.deserialize_any(UpdateCheckVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_backward_compatibility_version_check() {
        // 测试旧格式的version_check (Vec<String>)
        let old_format = r#"{
            "id": "test",
            "name": "Test Tool",
            "description": "Test",
            "website": "https://test.com",
            "command": "test",
            "version_check": ["--version"],
            "install": {}
        }"#;

        let config: ToolConfig = serde_json::from_str(old_format).expect("Should parse old format");

        // 验证转换后的格式
        assert_eq!(config.version_check.len(), 3);
        assert_eq!(
            config.version_check.get("windows"),
            Some(&vec!["--version".to_string()])
        );
        assert_eq!(
            config.version_check.get("macos"),
            Some(&vec!["--version".to_string()])
        );
        assert_eq!(
            config.version_check.get("linux"),
            Some(&vec!["--version".to_string()])
        );
    }

    #[test]
    fn test_backward_compatibility_update_check() {
        // 测试旧格式的update_check (Vec<String>)
        let old_format = r#"{
            "id": "test",
            "name": "Test Tool", 
            "description": "Test",
            "website": "https://test.com",
            "command": "test",
            "version_check": ["--version"],
            "update_check": ["update", "--check"],
            "install": {}
        }"#;

        let config: ToolConfig = serde_json::from_str(old_format).expect("Should parse old format");

        // 验证转换后的格式
        assert!(config.update_check.is_some());
        let update_check = config.update_check.unwrap();
        assert_eq!(update_check.len(), 3);
        assert_eq!(
            update_check.get("windows"),
            Some(&vec!["update".to_string(), "--check".to_string()])
        );
    }

    #[test]
    fn test_new_format_compatibility() {
        // 测试新格式的配置
        let new_format = r#"{
            "id": "test",
            "name": "Test Tool",
            "description": "Test", 
            "website": "https://test.com",
            "command": "test",
            "version_check": {
                "windows": ["--version"],
                "linux": ["-V"]
            },
            "update_check": {
                "windows": ["update", "--check"],
                "linux": ["upgrade"]
            },
            "install": {}
        }"#;

        let config: ToolConfig = serde_json::from_str(new_format).expect("Should parse new format");

        // 验证新格式保持不变
        assert_eq!(
            config.version_check.get("windows"),
            Some(&vec!["--version".to_string()])
        );
        assert_eq!(
            config.version_check.get("linux"),
            Some(&vec!["-V".to_string()])
        );

        let update_check = config.update_check.unwrap();
        assert_eq!(
            update_check.get("windows"),
            Some(&vec!["update".to_string(), "--check".to_string()])
        );
        assert_eq!(
            update_check.get("linux"),
            Some(&vec!["upgrade".to_string()])
        );
    }
}
