# CLIverge - AI CLI工具集成管理平台

<p align="center">
  <strong>轻量化 • 可视化 • 跨平台</strong>
</p>

<p align="center">
  <a href="#功能特性">功能特性</a> •
  <a href="#快速开始">快速开始</a> •
  <a href="#架构设计">架构设计</a> •
  <a href="#支持工具">支持工具</a> •
  <a href="#开发计划">开发计划</a> •
  <a href="#贡献指南">贡献指南</a>
</p>

---

## 📖 项目简介

**CLIverge** 是一个轻量化的桌面AI工具管理平台，旨在简化AI命令行工具的安装、管理和使用。通过直观的GUI界面，用户可以轻松管理各种AI开发工具，无需记忆复杂的命令行操作。

### 核心理念

- **CLI** + **verge** (边缘、汇聚) = **CLIverge**
- 汇聚各种CLI工具于一个平台的统一管理解决方案

## ✨ 功能特性

### 已实现功能

- ✅ **现代化GUI界面** - 基于Egui的原生桌面应用
- ✅ **工具状态检测** - 实时检测工具安装状态和版本信息
- ✅ **智能缓存系统** - 缓存工具状态、版本信息和帮助文档
- ✅ **配置化管理** - 通过JSON配置文件管理工具，无需动态加载
- ✅ **异步执行架构** - 基于Tokio的高性能异步执行
- ✅ **跨平台支持** - Windows、macOS、Linux原生支持
- ✅ **多主题切换** - 支持明亮/暗色主题
- ✅ **实时通知系统** - 操作反馈和状态更新通知

### 开发中功能

- 🚧 **一键安装/卸载** - 智能识别平台，自动选择最佳安装方式
- 🚧 **版本更新检查** - 自动检查工具更新
- 🚧 **工具配置管理** - 可视化配置工具参数
- 🚧 **命令执行终端** - 集成终端模拟器

## 🛠 支持工具

目前支持以下AI命令行工具的管理：

| 工具名称 | 描述 | 状态 |
|---------|------|------|
| **Claude Code CLI** | Anthropic Claude AI代码助手 | ✅ 完整支持 |
| **Gemini CLI** | Google Gemini多模态AI助手 | ✅ 完整支持 |
| **Qwen Code CLI** | 阿里云通义千问代码版 | ✅ 基础支持 |
| **OpenAI CLI** | OpenAI官方命令行工具 | ✅ 基础支持 |
| **Cursor CLI** | Cursor编辑器命令行工具 | ✅ 基础支持 |
| **OpenCode** | 开源代码生成工具 | ✅ 基础支持 |
| **iFlow CLI** | 智能工作流自动化工具 | ✅ 基础支持 |

## 🚀 快速开始

### 系统要求

- **操作系统**: Windows 10+, macOS 10.15+, Linux (主流发行版)
- **运行环境**: Rust 1.78+ (开发), 无需运行时依赖(使用)
- **内存**: 最低 2GB RAM
- **磁盘空间**: 100MB

### 安装

#### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/yourusername/cliverge.git
cd cliverge

# 构建项目
cargo build --release

# 运行应用
cargo run --release -p cliverge-gui
```

#### 预编译二进制（开发中）

访问 [Releases](https://github.com/yourusername/cliverge/releases) 页面下载对应平台的安装包。

### 使用指南

1. **启动应用**: 双击可执行文件或从终端运行
2. **浏览工具**: 左侧面板显示所有可用的AI工具
3. **查看状态**: 自动检测每个工具的安装状态
4. **安装工具**: 点击"Install"按钮一键安装
5. **管理配置**: 在Settings中配置工具参数

## 🏗 架构设计

### 技术栈

- **核心语言**: Rust (2021 Edition)
- **GUI框架**: Egui (即时模式GUI)
- **异步运行时**: Tokio
- **序列化**: Serde + JSON
- **缓存系统**: 自定义JSON缓存

### 模块结构

```
cliverge/
├── crates/
│   ├── cliverge-gui/      # GUI应用主模块
│   │   ├── src/
│   │   │   ├── main.rs    # 应用入口
│   │   │   └── app.rs     # 主应用逻辑
│   │   └── Cargo.toml
│   └── cliverge-core/     # 核心服务层
│       ├── src/
│       │   ├── lib.rs     # 模块导出
│       │   ├── config.rs  # 配置管理
│       │   ├── tool.rs    # 工具管理
│       │   ├── version.rs # 版本检查
│       │   ├── cache.rs   # 缓存系统
│       │   └── error.rs   # 错误处理
│       └── Cargo.toml
├── configs/
│   ├── tools.json         # 工具配置文件
│   └── settings.json      # 应用设置模板
└── Cargo.toml            # 工作空间配置
```

### 设计原则

1. **轻量化优先** - 最小化复杂度，专注核心功能
2. **配置化管理** - 通过JSON配置文件管理工具，避免复杂的插件系统
3. **用户体验至上** - 简洁直观的界面，降低使用门槛
4. **性能优化** - 异步执行，智能缓存，快速响应

## 📊 项目状态

### 完成度评估

| 模块 | 完成度 | 说明 |
|-----|--------|-----|
| **GUI界面** | 85% | 主界面完整，部分功能待完善 |
| **核心引擎** | 60% | 基础功能实现，高级特性开发中 |
| **工具管理** | 70% | 状态检测完成，安装功能开发中 |
| **缓存系统** | 90% | 完整的缓存机制已实现 |
| **配置管理** | 80% | 配置读写完成，UI编辑器开发中 |
| **版本检查** | 40% | 基础架构完成，策略实现中 |

### 性能指标

- **启动时间**: < 3秒
- **内存占用**: < 50MB (空闲时)
- **CPU使用率**: < 1% (空闲时)
- **二进制大小**: ~30MB

## 🗺 开发计划

### 短期目标 (1-2周)

- [ ] 完善工具安装/卸载功能
- [ ] 实现版本更新检查机制
- [ ] 添加工具配置编辑器
- [ ] 优化错误处理和用户反馈

### 中期目标 (1个月)

- [ ] 集成终端模拟器
- [ ] 支持更多AI工具
- [ ] 实现工具使用统计
- [ ] 添加工具推荐功能

### 长期目标 (3个月)

- [ ] 插件市场（简化版）
- [ ] 云同步配置
- [ ] 工作流自动化
- [ ] 多语言支持

## 🤝 贡献指南

欢迎贡献代码、报告问题或提出建议！

### 如何贡献

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 提交 Pull Request

### 开发环境设置

```bash
# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装开发工具
cargo install cargo-watch cargo-edit

# 运行开发模式
cargo watch -x 'run -p cliverge-gui'
```

### 代码规范

- 遵循 Rust 官方代码风格指南
- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 进行代码检查
- 为公共API编写文档注释

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 🙏 致谢

- [Egui](https://github.com/emilk/egui) - 优秀的即时模式GUI框架
- [Tokio](https://tokio.rs/) - 强大的异步运行时
- 所有AI工具的开发者们

## 📞 联系方式

- 项目主页: [https://github.com/yourusername/cliverge](https://github.com/yourusername/cliverge)
- 问题反馈: [Issues](https://github.com/yourusername/cliverge/issues)
- 讨论交流: [Discussions](https://github.com/yourusername/cliverge/discussions)

---

<p align="center">
  Made with ❤️ by CLIverge Team
</p>
