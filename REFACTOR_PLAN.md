# CLIverge 轻量化重构实施计划

基于新的轻量化设计原则，本文档提供详细的项目重构计划和具体实施步骤。

## 🎯 重构目标

### 核心目标
- **移除复杂性**: 删除CLI/TUI/插件系统等复杂组件
- **专注GUI**: 只保留和完善桌面GUI应用
- **配置化管理**: 用JSON配置替代动态加载
- **轻量化架构**: 简化模块结构和依赖关系

### 成果预期
- 代码量减少 50%
- 构建时间减少 60%
- 内存占用 < 50MB
- 启动时间 < 3秒

## 📋 项目结构重构

### 现有结构
```
cliverge/
├── crates/
│   ├── cliverge-cli/      ❌ 删除
│   ├── cliverge-gui/      ✅ 保留并重构
│   ├── cliverge-core/     ✅ 重构为轻量服务层
│   ├── cliverge-sdk/      ❌ 删除 (简化为配置)
│   ├── cliverge-tools/    ✅ 简化为配置文件
│   ├── cliverge-registry/ ❌ 删除
│   └── cliverge-ui/       ❌ 删除
```

### 目标结构
```
cliverge/
├── crates/
│   ├── cliverge-gui/      # 主GUI应用
│   └── cliverge-core/     # 核心服务层
├── configs/
│   ├── tools.json         # 工具配置
│   └── settings.json      # 应用设置模板
├── docs/                  # 中文文档
├── assets/                # 应用资源
└── scripts/               # 构建脚本
```

## 🔧 Phase 1: 项目清理和重构准备 (3天)

### 任务清单

#### Day 1: 模块清理
- [ ] **备份当前项目状态**
  ```bash
  git branch backup-before-refactor
  git push origin backup-before-refactor
  ```

- [ ] **移除不需要的模块**
  ```bash
  rm -rf crates/cliverge-cli
  rm -rf crates/cliverge-sdk  
  rm -rf crates/cliverge-registry
  rm -rf crates/cliverge-ui
  ```

- [ ] **更新根Cargo.toml**
  ```toml
  [workspace]
  members = [
      "crates/cliverge-gui",
      "crates/cliverge-core",
  ]
  resolver = "2"

  [workspace.dependencies]
  tokio = { version = "1.0", features = ["rt-multi-thread", "fs", "process"] }
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  thiserror = "1.0"
  tracing = "0.1"
  dirs = "5.0"
  ```

#### Day 2: 依赖清理
- [ ] **清理cliverge-gui的依赖**
  ```toml
  # 移除不需要的依赖
  # clap = ...        ❌ 删除
  # ratatui = ...     ❌ 删除
  # rusqlite = ...    ❌ 删除
  # reqwest = ...     ❌ 删除 (简化网络需求)
  
  # 保留核心依赖
  [dependencies]
  cliverge-core = { path = "../cliverge-core" }
  eframe = "0.24"
  egui = "0.24"
  tokio = { workspace = true }
  serde = { workspace = true }
  serde_json = { workspace = true }
  tracing = { workspace = true }
  tracing-subscriber = "0.3"
  dirs = { workspace = true }
  ```

- [ ] **重写cliverge-core的Cargo.toml**
  ```toml
  [package]
  name = "cliverge-core"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  tokio = { workspace = true }
  serde = { workspace = true }
  serde_json = { workspace = true }
  thiserror = { workspace = true }
  tracing = { workspace = true }
  dirs = { workspace = true }
  ```

#### Day 3: 创建配置文件模板
- [ ] **创建工具配置文件模板**
  ```bash
  mkdir configs
  ```
  
- [ ] **设计tools.json结构** (见详细配置设计)

- [ ] **创建应用设置模板** (见详细配置设计)

## 🏗️ Phase 2: 核心服务层重构 (5天)

### Day 1-2: ConfigManager实现

#### 创建配置管理模块
```rust
// crates/cliverge-core/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

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
    pub version_check: Vec<String>,
    pub install: HashMap<String, InstallMethod>,
    pub config_schema: Option<HashMap<String, ConfigField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMethod {
    pub method: String,
    pub command: Option<Vec<String>>,
    pub url: Option<String>,
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
```

#### 实现配置管理器
```rust
// crates/cliverge-core/src/config.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Config not found: {0}")]
    NotFound(String),
}

pub struct ConfigManager {
    app_settings: AppSettings,
    tools_config: ToolsConfig,
    config_dir: PathBuf,
}

impl ConfigManager {
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
    
    pub fn get_tool_config(&self, id: &str) -> Option<&ToolConfig> {
        self.tools_config.tools.iter().find(|t| t.id == id)
    }
    
    // ... 其他方法实现
    
    fn get_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cliverge")
    }
}
```

### Day 3-4: ToolManager实现

#### 工具状态定义
```rust
// crates/cliverge-core/src/tool.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolStatus {
    Unknown,
    NotInstalled,
    Installed { version: String },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub config: ToolConfig,
    pub status: ToolStatus,
    pub user_config: HashMap<String, serde_json::Value>,
}
```

#### 工具管理器实现
```rust
// crates/cliverge-core/src/tool.rs
use std::process::Output;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Installation failed: {0}")]
    InstallFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ToolManager {
    config_manager: Arc<Mutex<ConfigManager>>,
    status_cache: Arc<Mutex<HashMap<String, ToolStatus>>>,
}

impl ToolManager {
    pub fn new(config_manager: Arc<Mutex<ConfigManager>>) -> Self {
        Self {
            config_manager,
            status_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn check_tool_status(&self, tool_id: &str) -> Result<ToolStatus, ToolError> {
        // 实现工具状态检测
        todo!()
    }
    
    pub async fn install_tool(&self, tool_id: &str) -> Result<(), ToolError> {
        // 实现工具安装
        todo!()
    }
    
    pub async fn uninstall_tool(&self, tool_id: &str) -> Result<(), ToolError> {
        // 实现工具卸载
        todo!()
    }
    
    pub async fn execute_tool(&self, tool_id: &str, args: &[String]) -> Result<Output, ToolError> {
        // 实现工具执行
        todo!()
    }
}
```

### Day 5: 核心模块集成测试
- [ ] **编写单元测试**
- [ ] **测试配置文件读写**
- [ ] **测试工具状态检测**
- [ ] **修复发现的问题**

## 🎨 Phase 3: GUI应用重构 (7天)

### Day 1-2: GUI主框架重构

#### 简化应用状态
```rust
// crates/cliverge-gui/src/app.rs
use cliverge_core::{ConfigManager, ToolManager};
use std::sync::{Arc, Mutex};

pub struct CLIvergeApp {
    config_manager: Arc<Mutex<ConfigManager>>,
    tool_manager: ToolManager,
    ui_state: UiState,
}

pub struct UiState {
    selected_tool: Option<String>,
    search_query: String,
    show_only_installed: bool,
    settings_window_open: bool,
    notifications: Vec<Notification>,
}

pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub timestamp: std::time::Instant,
}

pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}
```

#### 主界面重构
```rust
impl eframe::App for CLIvergeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_top_panel(ctx);
        self.render_main_content(ctx);
        self.render_status_bar(ctx);
        self.render_settings_window(ctx);
        self.render_notifications(ctx);
    }
}

impl CLIvergeApp {
    fn render_main_content(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tool_list")
            .resizable(true)
            .show(ctx, |ui| {
                self.render_tool_list(ui);
            });
            
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_tool_details(ui);
        });
    }
}
```

### Day 3-4: 工具列表和详情界面

#### 工具列表组件
```rust
impl CLIvergeApp {
    fn render_tool_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("AI Tools");
        ui.separator();
        
        // 搜索框
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.ui_state.search_query);
        });
        
        ui.checkbox(&mut self.ui_state.show_only_installed, "Show only installed");
        ui.separator();
        
        // 工具列表
        egui::ScrollArea::vertical().show(ui, |ui| {
            let tools = self.get_filtered_tools();
            for tool in tools {
                self.render_tool_item(ui, tool);
            }
        });
    }
    
    fn render_tool_item(&mut self, ui: &mut egui::Ui, tool: &ToolInfo) {
        let response = ui.selectable_label(
            self.ui_state.selected_tool.as_ref() == Some(&tool.config.id),
            format!("{} {}", self.get_status_icon(&tool.status), tool.config.name)
        );
        
        if response.clicked() {
            self.ui_state.selected_tool = Some(tool.config.id.clone());
        }
        
        ui.small(&tool.config.description);
        ui.separator();
    }
}
```

#### 工具详情面板
```rust
impl CLIvergeApp {
    fn render_tool_details(&mut self, ui: &mut egui::Ui) {
        if let Some(tool_id) = &self.ui_state.selected_tool.clone() {
            if let Some(tool) = self.get_tool_info(tool_id) {
                ui.heading(&tool.config.name);
                ui.label(&tool.config.description);
                
                ui.horizontal(|ui| {
                    ui.label("Website:");
                    ui.hyperlink(&tool.config.website);
                });
                
                ui.separator();
                
                // 状态显示
                self.render_tool_status(ui, &tool.status);
                
                // 操作按钮
                self.render_tool_actions(ui, tool_id);
                
                // 配置编辑
                if matches!(tool.status, ToolStatus::Installed { .. }) {
                    self.render_tool_config(ui, tool_id);
                }
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.heading("Welcome to CLIverge");
                ui.label("Select a tool from the list to see details");
            });
        }
    }
}
```

### Day 5-6: 设置窗口和配置编辑器

#### 设置窗口
```rust
impl CLIvergeApp {
    fn render_settings_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .open(&mut self.ui_state.settings_window_open)
            .show(ctx, |ui| {
                ui.heading("Appearance");
                // 主题选择
                // 字体大小
                // 窗口大小
                
                ui.separator();
                
                ui.heading("Behavior");  
                // 自动检查更新
                // 检查间隔
                // 显示通知
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.save_settings();
                        self.ui_state.settings_window_open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.ui_state.settings_window_open = false;
                    }
                });
            });
    }
}
```

### Day 7: GUI功能集成和测试
- [ ] **整合所有GUI组件**
- [ ] **测试用户交互流程**
- [ ] **性能优化**
- [ ] **UI响应性优化**

## 📝 Phase 4: 配置文件和工具集成 (5天)

### Day 1-2: 创建工具配置文件

#### tools.json配置文件
```json
{
  "version": "1.0",
  "tools": [
    {
      "id": "claude-code",
      "name": "Claude Code CLI",
      "description": "Anthropic Claude AI Code Assistant - Generate, explain and optimize code",
      "website": "https://www.anthropic.com/claude",
      "command": "claude",
      "version_check": ["--version"],
      "install": {
        "windows": {
          "method": "npm",
          "command": ["npm", "install", "-g", "@anthropic-ai/claude-cli"]
        },
        "macos": {
          "method": "brew",
          "command": ["brew", "install", "claude-cli"]
        },
        "linux": {
          "method": "npm",
          "command": ["npm", "install", "-g", "@anthropic-ai/claude-cli"]
        }
      },
      "config_schema": {
        "api_key": {
          "field_type": "string",
          "secret": true,
          "required": true,
          "description": "Anthropic API Key"
        },
        "model": {
          "field_type": "enum",
          "values": ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
          "default": "claude-3-sonnet",
          "description": "Claude model to use"
        }
      }
    },
    {
      "id": "gemini-cli",
      "name": "Gemini CLI",
      "description": "Google Gemini AI Assistant - Multimodal AI for text and code",
      "website": "https://ai.google.dev/",
      "command": "gemini",
      "version_check": ["--version"],
      "install": {
        "windows": {
          "method": "npm",
          "command": ["npm", "install", "-g", "@google-ai/gemini-cli"]
        },
        "macos": {
          "method": "brew",
          "command": ["brew", "install", "gemini-cli"]
        },
        "linux": {
          "method": "npm",
          "command": ["npm", "install", "-g", "@google-ai/gemini-cli"]
        }
      },
      "config_schema": {
        "api_key": {
          "field_type": "string",
          "secret": true,
          "required": true,
          "description": "Google AI API Key"
        }
      }
    }
  ]
}
```

### Day 3-4: 实现工具适配逻辑
- [ ] **实现跨平台安装逻辑**
- [ ] **实现版本检测**
- [ ] **实现工具执行**
- [ ] **错误处理和日志**

### Day 5: 工具配置管理
- [ ] **实现用户配置保存**
- [ ] **配置验证**
- [ ] **敏感信息加密**

## 🧪 Phase 5: 测试和优化 (3天)

### Day 1: 功能测试
- [ ] **工具检测测试**
- [ ] **安装/卸载测试**  
- [ ] **配置管理测试**
- [ ] **GUI交互测试**

### Day 2: 跨平台测试
- [ ] **Windows测试**
- [ ] **macOS测试**  
- [ ] **Linux测试**
- [ ] **修复平台特定问题**

### Day 3: 性能优化
- [ ] **启动时间优化**
- [ ] **内存使用优化**
- [ ] **UI响应性优化**
- [ ] **构建大小优化**

## 🚀 Phase 6: 打包和发布准备 (2天)

### Day 1: 构建系统
- [ ] **更新构建配置**
- [ ] **创建发布脚本**
- [ ] **测试发布包**

### Day 2: 文档和发布
- [ ] **更新README**
- [ ] **编写用户手册**
- [ ] **准备发布说明**

## 📊 验收标准

### 功能完整性
- [ ] ✅ GUI应用可正常启动
- [ ] ✅ 工具列表正确显示
- [ ] ✅ 工具状态检测准确
- [ ] ✅ 安装/卸载功能正常
- [ ] ✅ 配置管理功能完整
- [ ] ✅ 设置窗口功能正常

### 性能指标
- [ ] ✅ 启动时间 < 3秒
- [ ] ✅ 内存占用 < 50MB
- [ ] ✅ 构建包 < 30MB
- [ ] ✅ CPU占用 < 1% (空闲时)

### 代码质量
- [ ] ✅ 编译无警告
- [ ] ✅ 核心模块有单元测试
- [ ] ✅ 代码符合Rust最佳实践
- [ ] ✅ 文档注释完整

### 用户体验
- [ ] ✅ 界面直观易用
- [ ] ✅ 操作响应及时
- [ ] ✅ 错误提示清晰
- [ ] ✅ 跨平台体验一致

## 🎯 成功指标

### 量化目标
- **代码行数**: 减少50% (目标 < 5000行)
- **依赖数量**: 减少60% (目标 < 20个直接依赖)
- **构建时间**: 减少60% (目标 < 2分钟)
- **二进制大小**: 减少40% (目标 < 30MB)

### 质量目标
- **测试覆盖率**: 核心模块 > 80%
- **文档覆盖率**: 公开API 100%
- **性能基准**: 所有操作 < 500ms响应
- **稳定性**: 连续运行24小时无崩溃

## 📝 任务分配建议

### 开发者技能要求
- **Rust经验**: 中级以上
- **GUI开发**: Egui框架经验
- **JSON/配置管理**: 熟悉
- **跨平台开发**: 了解

### 预计工作量
- **总工时**: 25个工作日
- **开发人员**: 1-2人
- **项目周期**: 5-6周
- **风险缓冲**: 20%额外时间

## 🚨 风险控制

### 主要风险
1. **配置复杂度**: 工具配置可能比预期复杂
2. **平台兼容性**: 不同平台安装方式差异大
3. **性能目标**: GUI性能可能不达标

### 缓解措施
1. **分阶段验证**: 每个Phase都有明确验收标准
2. **及时调整**: 发现问题及时调整设计
3. **用户反馈**: 早期用户测试和反馈

---

通过这个详细的重构计划，CLIverge将转变为一个轻量化、专注且高效的AI工具管理平台。
