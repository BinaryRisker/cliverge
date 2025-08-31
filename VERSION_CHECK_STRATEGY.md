# CLIverge 轻量化版本检查策略

## 问题分析

移除网络组件（reqwest）后，我们需要重新设计AI工具的版本检查机制。传统方案是通过HTTP请求获取最新版本信息，但这与轻量化设计原则冲突。

## 解决方案对比

### 方案1：利用工具自身的版本检查功能 ⭐⭐⭐⭐⭐
**推荐度：★★★★★**

#### 原理
大多数现代CLI工具都内置了版本检查功能，我们可以利用这些工具自身的更新检查机制。

#### 实现方式
```rust
// crates/cliverge-core/src/version.rs
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    pub check_command: Vec<String>,
}

pub struct VersionChecker;

impl VersionChecker {
    /// 检查工具当前版本
    pub async fn check_current_version(tool_config: &ToolConfig) -> Result<String, VersionError> {
        let output = tokio::process::Command::new(&tool_config.command)
            .args(&tool_config.version_check)
            .output()
            .await?;
            
        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            Ok(Self::parse_version_string(&version_str))
        } else {
            Err(VersionError::CommandFailed)
        }
    }
    
    /// 利用工具自身的更新检查功能
    pub async fn check_for_updates(tool_config: &ToolConfig) -> Result<VersionInfo, VersionError> {
        let mut version_info = VersionInfo {
            current: None,
            latest: None,
            update_available: false,
            check_command: tool_config.update_check.clone().unwrap_or_default(),
        };
        
        // 1. 获取当前版本
        if let Ok(current) = Self::check_current_version(tool_config).await {
            version_info.current = Some(current);
        }
        
        // 2. 使用工具自身的更新检查
        if let Some(update_cmd) = &tool_config.update_check {
            let output = tokio::process::Command::new(&update_cmd[0])
                .args(&update_cmd[1..])
                .output()
                .await?;
                
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                version_info.latest = Self::parse_latest_version(&output_str);
                version_info.update_available = Self::compare_versions(
                    &version_info.current, 
                    &version_info.latest
                );
            }
        }
        
        Ok(version_info)
    }
    
    fn parse_version_string(output: &str) -> String {
        // 解析版本字符串的通用逻辑
        output.lines()
            .find_map(|line| {
                // 匹配 x.y.z 格式的版本号
                if let Some(captures) = regex::Regex::new(r"(\d+\.\d+\.\d+)")
                    .unwrap()
                    .captures(line) 
                {
                    captures.get(1).map(|m| m.as_str().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}
```

#### 配置文件扩展
```json
{
  "id": "claude-code",
  "name": "Claude Code CLI",
  "command": "claude",
  "version_check": ["--version"],
  "update_check": ["claude", "update", "--check-only"],  // 新增
  "self_update": ["claude", "update"],                   // 新增
  "install": {
    // ... 现有安装配置
  }
}
```

#### 优点
- ✅ 无需网络依赖
- ✅ 利用工具官方检查机制，准确可靠
- ✅ 支持离线使用
- ✅ 响应速度快

#### 缺点
- ❌ 依赖工具自身支持更新检查
- ❌ 不同工具的检查方式可能不同

### 方案2：基于包管理器的版本检查 ⭐⭐⭐⭐
**推荐度：★★★★☆**

#### 原理
利用系统包管理器（npm, brew, pip等）的版本查询功能。

#### 实现方式
```rust
impl VersionChecker {
    pub async fn check_via_package_manager(tool_config: &ToolConfig) -> Result<VersionInfo, VersionError> {
        let platform = std::env::consts::OS;
        let install_config = tool_config.install.get(platform)
            .ok_or(VersionError::UnsupportedPlatform)?;
            
        match install_config.method.as_str() {
            "npm" => Self::check_npm_version(tool_config).await,
            "brew" => Self::check_brew_version(tool_config).await,
            "pip" => Self::check_pip_version(tool_config).await,
            _ => Err(VersionError::UnsupportedMethod),
        }
    }
    
    async fn check_npm_version(tool_config: &ToolConfig) -> Result<VersionInfo, VersionError> {
        // npm view @anthropic-ai/claude-cli version
        let package_name = Self::extract_npm_package_name(tool_config)?;
        
        let output = tokio::process::Command::new("npm")
            .args(&["view", &package_name, "version"])
            .output()
            .await?;
            
        if output.status.success() {
            let latest = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let current = Self::check_current_version(tool_config).await.ok();
            
            Ok(VersionInfo {
                current,
                latest: Some(latest.clone()),
                update_available: Self::compare_versions(&current, &Some(latest)),
                check_command: vec!["npm".to_string(), "view".to_string(), package_name, "version".to_string()],
            })
        } else {
            Err(VersionError::PackageNotFound)
        }
    }
    
    async fn check_brew_version(tool_config: &ToolConfig) -> Result<VersionInfo, VersionError> {
        // brew info claude-cli
        let formula_name = Self::extract_brew_formula_name(tool_config)?;
        
        let output = tokio::process::Command::new("brew")
            .args(&["info", &formula_name, "--json"])
            .output()
            .await?;
            
        // 解析JSON获取版本信息
        // ... 实现细节
        todo!()
    }
}
```

#### 优点
- ✅ 准确可靠，数据来源权威
- ✅ 无需网络依赖（包管理器缓存）
- ✅ 统一的检查方式

#### 缺点
- ❌ 需要系统安装对应包管理器
- ❌ 包管理器缓存可能过时
- ❌ 不同平台实现差异大

### 方案3：本地版本数据库 + 手动更新 ⭐⭐⭐
**推荐度：★★★☆☆**

#### 原理
维护一个本地版本数据库，通过应用更新或手动更新机制同步最新版本信息。

#### 实现方式
```rust
// 版本数据库结构
#[derive(Serialize, Deserialize)]
pub struct VersionDatabase {
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub tools: HashMap<String, ToolVersionInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct ToolVersionInfo {
    pub latest_version: String,
    pub release_date: chrono::DateTime<chrono::Utc>,
    pub download_url: Option<String>,
    pub changelog_url: Option<String>,
}

impl VersionDatabase {
    pub fn load() -> Result<Self, VersionError> {
        let path = Self::get_database_path();
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
    
    pub fn get_latest_version(&self, tool_id: &str) -> Option<&str> {
        self.tools.get(tool_id).map(|info| info.latest_version.as_str())
    }
    
    pub fn is_stale(&self) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.last_updated);
        age.num_days() > 7 // 7天过期
    }
}
```

#### 版本数据文件示例
```json
{
  "last_updated": "2024-12-31T08:00:00Z",
  "tools": {
    "claude-code": {
      "latest_version": "1.2.5",
      "release_date": "2024-12-25T10:00:00Z",
      "download_url": "https://github.com/anthropics/claude-cli/releases/tag/v1.2.5",
      "changelog_url": "https://github.com/anthropics/claude-cli/releases/tag/v1.2.5"
    },
    "gemini-cli": {
      "latest_version": "2.1.0",
      "release_date": "2024-12-28T15:30:00Z"
    }
  }
}
```

#### 优点
- ✅ 完全离线工作
- ✅ 可控的数据来源
- ✅ 支持批量更新

#### 缺点
- ❌ 需要手动维护版本数据
- ❌ 版本信息可能滞后
- ❌ 增加维护工作量

### 方案4：混合策略 ⭐⭐⭐⭐⭐
**推荐度：★★★★★**

#### 设计思路
结合多种方案的优点，提供灵活的版本检查策略：

```rust
#[derive(Debug, Clone)]
pub enum VersionCheckStrategy {
    SelfCheck,      // 工具自检
    PackageManager, // 包管理器
    LocalDatabase,  // 本地数据库
    Auto,          // 自动选择
}

impl VersionChecker {
    pub async fn check_version(
        tool_config: &ToolConfig, 
        strategy: VersionCheckStrategy
    ) -> Result<VersionInfo, VersionError> {
        match strategy {
            VersionCheckStrategy::SelfCheck => {
                Self::check_for_updates(tool_config).await
            },
            VersionCheckStrategy::PackageManager => {
                Self::check_via_package_manager(tool_config).await
            },
            VersionCheckStrategy::LocalDatabase => {
                Self::check_local_database(tool_config).await
            },
            VersionCheckStrategy::Auto => {
                // 自动选择最佳策略
                Self::auto_check(tool_config).await
            },
        }
    }
    
    async fn auto_check(tool_config: &ToolConfig) -> Result<VersionInfo, VersionError> {
        // 优先级：工具自检 > 包管理器 > 本地数据库
        if tool_config.update_check.is_some() {
            if let Ok(result) = Self::check_for_updates(tool_config).await {
                return Ok(result);
            }
        }
        
        if let Ok(result) = Self::check_via_package_manager(tool_config).await {
            return Ok(result);
        }
        
        Self::check_local_database(tool_config).await
    }
}
```

## 推荐实施方案

### 第一阶段：基础实现
1. **实现工具自检机制**（方案1）
2. **添加包管理器检查**（方案2）
3. **创建混合策略框架**（方案4）

### 第二阶段：完善功能
1. **添加本地版本数据库**（方案3）
2. **实现用户选择策略的设置**
3. **添加缓存机制提高性能**

## 具体实现计划

### 1. 工具配置扩展
```json
{
  "id": "claude-code",
  "name": "Claude Code CLI",
  "command": "claude",
  "version_check": ["--version"],
  "update_check": ["claude", "update", "--check-only"],
  "version_check_strategy": "auto", // auto, self, package_manager, local
  "install": {
    "windows": {
      "method": "npm",
      "command": ["npm", "install", "-g", "@anthropic-ai/claude-cli"],
      "package_name": "@anthropic-ai/claude-cli"  // 用于包管理器查询
    }
  }
}
```

### 2. GUI界面集成
```rust
impl CLIvergeApp {
    fn render_version_info(&mut self, ui: &mut egui::Ui, tool_id: &str) {
        if let Some(version_info) = self.get_version_info(tool_id) {
            ui.horizontal(|ui| {
                ui.label("Current:");
                ui.strong(version_info.current.as_deref().unwrap_or("Unknown"));
                
                if let Some(latest) = &version_info.latest {
                    ui.label("Latest:");
                    ui.strong(latest);
                    
                    if version_info.update_available {
                        ui.colored_label(egui::Color32::YELLOW, "🔄 Update Available");
                    }
                }
                
                if ui.button("🔍 Check Version").clicked() {
                    self.check_tool_version(tool_id);
                }
            });
        }
    }
    
    fn check_tool_version(&mut self, tool_id: &str) {
        let tool_manager = self.tool_manager.clone();
        let tool_id = tool_id.to_string();
        
        self.runtime.spawn(async move {
            match tool_manager.check_version(&tool_id, VersionCheckStrategy::Auto).await {
                Ok(version_info) => {
                    // 更新UI状态
                },
                Err(e) => {
                    tracing::error!("Version check failed: {}", e);
                }
            }
        });
    }
}
```

### 3. 设置界面
```rust
impl CLIvergeApp {
    fn render_version_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Version Check Settings");
        
        ui.horizontal(|ui| {
            ui.label("Default Strategy:");
            egui::ComboBox::from_label("")
                .selected_text(format!("{:?}", self.settings.version_check_strategy))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.version_check_strategy, 
                                      VersionCheckStrategy::Auto, "Auto");
                    ui.selectable_value(&mut self.settings.version_check_strategy, 
                                      VersionCheckStrategy::SelfCheck, "Tool Self-Check");
                    ui.selectable_value(&mut self.settings.version_check_strategy, 
                                      VersionCheckStrategy::PackageManager, "Package Manager");
                    ui.selectable_value(&mut self.settings.version_check_strategy, 
                                      VersionCheckStrategy::LocalDatabase, "Local Database");
                });
        });
        
        ui.checkbox(&mut self.settings.auto_check_versions, "Auto check versions on startup");
        
        ui.horizontal(|ui| {
            ui.label("Check interval:");
            ui.add(egui::DragValue::new(&mut self.settings.version_check_interval_hours)
                .clamp_range(1..=168)
                .suffix(" hours"));
        });
    }
}
```

## 优势总结

1. **轻量化兼容**：无需引入重型网络依赖
2. **多重保障**：多种检查方式互为备份
3. **用户可控**：用户可选择检查策略
4. **性能优秀**：本地操作，响应快速
5. **扩展性好**：易于添加新的检查方式

## 建议实施优先级

1. **P0 - 立即实施**：工具自检 + 混合策略框架
2. **P1 - 短期实施**：包管理器检查
3. **P2 - 长期实施**：本地版本数据库 + 高级功能

这样的设计既保持了轻量化特性，又提供了可靠的版本检查功能，是在移除网络组件后的最佳平衡方案。
