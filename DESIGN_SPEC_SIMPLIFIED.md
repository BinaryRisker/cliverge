# CLIverge - 轻量化AI工具管理平台设计方案

## 项目概述

**CLIverge** 是一个轻量化的桌面AI工具管理平台，专注于通过简洁的GUI界面管理各种AI CLI工具。采用配置化而非复杂插件系统的设计理念，确保项目的简洁性和可维护性。

### 核心设计原则
- **轻量化优先** - 最小化复杂度，专注核心功能
- **GUI单一界面** - 仅提供桌面GUI，不开发CLI和TUI
- **配置化管理** - 通过JSON配置文件管理工具，无需动态加载
- **英文界面** - 代码和界面使用英文，文档使用中文

## 简化架构设计

### 整体架构图

```
┌─────────────────────────────────────────┐
│               GUI Application           │
├─────────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────────────┐ │
│ │ Main Window │ │ Settings Window     │ │
│ │ (Tool Mgmt) │ │ (Config Editor)     │ │
│ └─────────────┘ └─────────────────────┘ │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│            Core Services                │
├─────────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────────────┐ │
│ │Tool Manager │ │ Config Manager      │ │
│ └─────────────┘ └─────────────────────┘ │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│           Foundation Layer              │
├─────────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────────────┐ │
│ │ File I/O    │ │ Process Execution   │ │
│ └─────────────┘ └─────────────────────┘ │
└─────────────────────────────────────────┘
```

### 模块重构方案

**保留模块:**
- `cliverge-gui` - GUI应用 (核心)
- `cliverge-core` - 核心服务层
- `cliverge-tools` - 工具适配器 (简化)

**移除模块:**
- `cliverge-cli` - CLI应用 (不需要)
- `cliverge-sdk` - 插件SDK (简化为配置)
- `cliverge-registry` - 注册表服务 (简化为本地配置)
- `cliverge-ui` - TUI框架 (不需要)

## 核心功能设计

### 1. GUI主界面功能

#### 主窗口 (Main Window)
- **工具列表视图** - 显示所有配置的AI工具
- **工具状态监控** - 实时检测工具安装状态
- **一键安装/卸载** - 简化的工具管理操作
- **快速启动** - 直接启动已安装的工具
- **搜索和筛选** - 按名称、状态筛选工具

#### 设置窗口 (Settings Window)
- **工具配置编辑器** - 可视化编辑工具配置
- **应用设置** - 主题、更新检查等基础设置
- **导入/导出配置** - 配置文件的备份和共享

### 2. 配置化工具管理

#### 工具配置文件结构
```json
{
  "version": "1.0",
  "tools": [
    {
      "id": "claude-code",
      "name": "Claude Code CLI",
      "description": "Anthropic Claude AI Code Assistant",
      "website": "https://www.anthropic.com/claude",
      "command": "claude",
      "version_check": ["--version"],
      "install": {
        "windows": {
          "method": "npm",
          "command": ["npm", "install", "-g", "@anthropic/claude-cli"]
        },
        "macos": {
          "method": "brew",
          "command": ["brew", "install", "claude-cli"]
        },
        "linux": {
          "method": "script",
          "url": "https://install.anthropic.com/claude-cli.sh"
        }
      },
      "config_schema": {
        "api_key": {
          "type": "string",
          "secret": true,
          "required": true,
          "description": "Anthropic API Key"
        },
        "model": {
          "type": "enum",
          "values": ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
          "default": "claude-3-sonnet",
          "description": "Claude model to use"
        }
      }
    }
  ]
}
```

#### 应用设置文件
```json
{
  "appearance": {
    "theme": "dark",
    "font_size": 14,
    "window_size": [1200, 800]
  },
  "behavior": {
    "auto_check_updates": true,
    "check_interval_minutes": 30,
    "show_notifications": true
  },
  "tools_config_path": "./tools.json",
  "data_directory": "~/.cliverge"
}
```

## 技术实现方案

### 1. 技术栈

#### 核心技术
- **语言**: Rust 2021 Edition
- **GUI框架**: Egui (保持现有选择)
- **异步运行时**: Tokio
- **序列化**: Serde (JSON)
- **进程执行**: tokio::process
- **文件系统**: tokio::fs

#### 依赖精简
移除不必要的依赖：
- `clap` - CLI框架 (不需要)
- `ratatui` - TUI框架 (不需要)
- `rusqlite` - 数据库 (使用JSON文件)
- `reqwest` - 网络请求 (简化为本地操作)

### 2. 核心服务实现

#### ConfigManager - 配置管理器
```rust
pub struct ConfigManager {
    app_settings: AppSettings,
    tools_config: ToolsConfig,
    config_dir: PathBuf,
}

impl ConfigManager {
    pub async fn load() -> Result<Self, ConfigError>;
    pub async fn save(&self) -> Result<(), ConfigError>;
    pub fn get_tool_config(&self, id: &str) -> Option<&ToolConfig>;
    pub fn update_tool_config(&mut self, id: &str, config: ToolConfig);
    pub fn add_tool(&mut self, tool: ToolConfig);
    pub fn remove_tool(&mut self, id: &str);
}
```

#### ToolManager - 工具管理器
```rust
pub struct ToolManager {
    config_manager: Arc<Mutex<ConfigManager>>,
    status_cache: Arc<Mutex<HashMap<String, ToolStatus>>>,
}

impl ToolManager {
    pub async fn check_tool_status(&self, tool_id: &str) -> Result<ToolStatus, ToolError>;
    pub async fn install_tool(&self, tool_id: &str) -> Result<(), ToolError>;
    pub async fn uninstall_tool(&self, tool_id: &str) -> Result<(), ToolError>;
    pub async fn execute_tool(&self, tool_id: &str, args: &[String]) -> Result<Output, ToolError>;
    pub async fn refresh_all_status(&self) -> Result<(), ToolError>;
}
```

### 3. GUI界面实现

#### 主应用结构
```rust
pub struct CLIvergeApp {
    config_manager: Arc<Mutex<ConfigManager>>,
    tool_manager: ToolManager,
    ui_state: UiState,
    notification_system: NotificationSystem,
}

pub struct UiState {
    current_view: ViewType,
    selected_tool: Option<String>,
    search_query: String,
    show_only_installed: bool,
    settings_window_open: bool,
}

pub enum ViewType {
    ToolList,
    ToolDetails,
    Settings,
}
```

#### 界面组件
```rust
impl CLIvergeApp {
    fn render_tool_list(&mut self, ui: &mut Ui);
    fn render_tool_details(&mut self, ui: &mut Ui, tool_id: &str);
    fn render_settings_window(&mut self, ctx: &Context);
    fn render_config_editor(&mut self, ui: &mut Ui);
    fn render_notification(&mut self, ctx: &Context);
}
```

## 数据存储方案

### 1. 文件结构
```
~/.cliverge/
├── settings.json          # 应用设置
├── tools.json            # 工具配置
├── cache/
│   ├── tool_status.json  # 工具状态缓存
│   └── icons/            # 工具图标缓存
└── logs/
    └── app.log           # 应用日志
```

### 2. 配置热重载
- 监控配置文件变化
- 自动重新加载配置
- UI实时更新

### 3. 备份和恢复
- 配置文件自动备份
- 支持手动导入/导出
- 版本兼容性检查

## 工具集成方案

### 1. 支持的AI工具列表

#### 第一批 (6个核心工具)
1. **Claude Code CLI** - Anthropic Claude代码助手
2. **Gemini CLI** - Google Gemini AI助手
3. **Qwen Code CLI** - 阿里云通义千问
4. **OpenAI CLI** - OpenAI官方命令行工具
5. **GitHub Copilot CLI** - GitHub Copilot命令行
6. **Cursor CLI** - Cursor编辑器命令行工具

#### 工具适配器简化
```rust
pub struct ToolAdapter {
    pub config: ToolConfig,
}

impl ToolAdapter {
    pub async fn check_installed(&self) -> Result<bool, ToolError>;
    pub async fn get_version(&self) -> Result<String, ToolError>;
    pub async fn install(&self) -> Result<(), ToolError>;
    pub async fn uninstall(&self) -> Result<(), ToolError>;
    pub async fn execute(&self, args: &[String]) -> Result<Output, ToolError>;
}
```

### 2. 安装方法支持

#### Windows
- **npm/yarn** - Node.js生态工具
- **pip** - Python生态工具
- **winget** - Windows包管理器
- **scoop** - 第三方包管理器
- **直接下载** - 可执行文件下载

#### macOS
- **brew** - Homebrew包管理器
- **npm/yarn** - Node.js生态工具
- **pip** - Python生态工具

#### Linux
- **apt/yum/pacman** - 系统包管理器
- **npm/yarn** - Node.js生态工具
- **pip** - Python生态工具
- **snap** - Snap包管理器

## 用户体验设计

### 1. 界面设计原则
- **简洁明了** - 减少UI元素，突出核心功能
- **响应式布局** - 适应不同窗口大小
- **状态清晰** - 工具状态一目了然
- **操作便捷** - 常用操作一键完成

### 2. 主界面布局
```
┌─────────────────────────────────────────────────────┐
│ CLIverge - AI Tool Manager                  [_][□][×]│
├─────────────────────────────────────────────────────┤
│ 🔍 Search: [___________]  [⚙Settings] [🔄Refresh]   │
├─────────────────┬───────────────────────────────────┤
│ 📋 Tool List    │ 📄 Tool Details                   │
│                 │                                   │
│ ✅ Claude Code  │ 🤖 Claude Code CLI                │
│ ❌ Gemini CLI   │ Anthropic Claude AI Assistant     │
│ ✅ Qwen Code    │                                   │
│ ❌ OpenAI CLI   │ Status: ✅ Installed v1.2.3      │
│ ❌ Copilot CLI  │                                   │
│ ❌ Cursor CLI   │ [🚀 Launch] [🔄 Update] [❌ Remove]│
│                 │                                   │
│                 │ 📋 Configuration:                 │
│                 │ API Key: [************]           │
│                 │ Model: [Claude-3-Sonnet ▼]       │
│                 │                                   │
└─────────────────┴───────────────────────────────────┘
│ 📊 Status: 3 installed, 3 available | Last check: now │
└─────────────────────────────────────────────────────┘
```

### 3. 通知系统
- **toast通知** - 操作完成提示
- **进度条** - 安装/卸载进度
- **状态图标** - 实时状态显示

## 实施计划

### Phase 1: 核心架构 (2周)
**目标**: 建立基础架构和配置系统

**任务清单**:
- [ ] 重构项目结构，移除不需要的模块
- [ ] 实现ConfigManager配置管理
- [ ] 实现基础的ToolManager
- [ ] 设计和实现配置文件格式
- [ ] 基础GUI框架搭建

**验收标准**:
- 配置文件可正常读写
- GUI主界面框架可显示
- 工具列表可从配置加载

### Phase 2: 工具管理 (2周)
**目标**: 实现工具检测、安装和管理功能

**任务清单**:
- [ ] 实现工具状态检测
- [ ] 实现跨平台安装逻辑
- [ ] 实现工具执行功能
- [ ] 添加错误处理和日志
- [ ] 实现状态缓存机制

**验收标准**:
- 可正确检测工具安装状态
- 可成功安装/卸载支持的工具
- 可启动已安装的工具

### Phase 3: GUI完善 (2周)
**目标**: 完善用户界面和交互体验

**任务清单**:
- [ ] 完善工具列表和详情界面
- [ ] 实现设置窗口和配置编辑器
- [ ] 添加搜索和筛选功能
- [ ] 实现通知系统
- [ ] 优化界面响应性和性能

**验收标准**:
- UI界面完整且美观
- 所有核心功能可通过GUI操作
- 用户交互流畅无卡顿

### Phase 4: 工具集成 (2周)
**目标**: 集成6个核心AI工具

**任务清单**:
- [ ] 完善Claude Code CLI集成
- [ ] 添加Gemini CLI支持
- [ ] 添加Qwen Code CLI支持
- [ ] 添加OpenAI CLI支持
- [ ] 添加GitHub Copilot CLI支持
- [ ] 添加Cursor CLI支持

**验收标准**:
- 所有6个工具可正确检测和安装
- 工具配置可通过GUI管理
- 基础功能测试通过

### Phase 5: 测试和优化 (1周)
**目标**: 全面测试和性能优化

**任务清单**:
- [ ] 跨平台测试 (Windows/macOS/Linux)
- [ ] 性能优化和内存泄漏检查
- [ ] 错误处理完善
- [ ] 用户体验优化
- [ ] 文档编写

**验收标准**:
- 所有平台功能正常
- 启动时间 < 3秒
- 内存占用 < 50MB
- 无明显bug和崩溃

### Phase 6: 发布准备 (1周)
**目标**: 准备v1.0正式发布

**任务清单**:
- [ ] 构建发布包
- [ ] 编写用户手册
- [ ] 准备安装包 (MSI/DMG/AppImage)
- [ ] 设置自动更新机制
- [ ] Beta版本测试

**验收标准**:
- 安装包可正常安装和卸载
- 功能完整且稳定
- 文档齐全

## 质量保证

### 1. 测试策略
- **单元测试** - 核心逻辑模块
- **集成测试** - GUI和服务层集成
- **手动测试** - 用户体验和边界情况
- **跨平台测试** - Windows/macOS/Linux

### 2. 性能指标
- **启动时间**: < 3秒
- **内存占用**: < 50MB
- **CPU使用率**: 空闲时 < 1%
- **文件大小**: 安装包 < 30MB

### 3. 代码质量
- **测试覆盖率**: > 80%
- **代码重复率**: < 5%
- **复杂度**: 函数复杂度 < 10
- **文档覆盖率**: 公开API 100%

## 风险评估

### 技术风险
1. **跨平台兼容性** - 不同平台的包管理器差异
   - **缓解措施**: 充分测试，提供手动安装说明

2. **工具API变更** - 第三方工具命令行接口变化
   - **缓解措施**: 配置化设计，支持快速适配

### 项目风险
1. **功能范围膨胀** - 需求增加导致复杂度上升
   - **缓解措施**: 严格控制范围，坚持轻量化原则

2. **用户接受度** - 简化设计可能不满足高级用户需求
   - **缓解措施**: 专注目标用户群体，收集反馈迭代

## 成功指标

### 技术指标
- **稳定性**: 崩溃率 < 0.1%
- **性能**: 响应时间 < 500ms
- **兼容性**: 支持主流操作系统版本

### 用户指标
- **易用性**: 新用户 5分钟内完成首次工具安装
- **满意度**: 用户评分 > 4.0/5.0
- **采用率**: 月活跃用户增长

### 开发指标
- **代码质量**: 静态检查无严重问题
- **维护性**: 新增工具支持 < 1天开发时间
- **文档完整性**: 用户手册和API文档齐全

---

通过这个轻量化的设计方案，CLIverge将成为一个简洁高效的AI工具管理平台，专注于核心功能，避免过度设计，确保项目的成功交付和长期维护。
