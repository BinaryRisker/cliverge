# CLIverge 图标使用指南

## 图标系统概述

CLIverge 已集成统一的图标系统，确保桌面快捷方式和应用程序窗口的图标保持一致。

## 图标设计

### 设计理念
- **终端风格**: 体现CLI工具管理的核心功能
- **现代化**: 使用蓝紫色渐变，符合现代应用程序审美
- **汇聚概念**: 通过连接点表达"CLIverge"（CLI + verge）的汇聚理念

### 图标元素
- 圆形背景：蓝紫色渐变 (#3F66F1 → #8B5CF6)
- 终端窗口：深色背景，模拟真实终端
- 窗口控制按钮：红、黄、绿三色圆点
- 汇聚标志：右侧连接点展示工具聚合概念

## 已实现功能

### ✅ 应用程序窗口图标
- **位置**: 应用程序窗口左上角
- **实现**: 通过 `eframe::ViewportBuilder::with_icon()` 设置
- **文件**: `src/main.rs` 中的 `load_icon()` 函数
- **格式**: 32x32 像素，RGBA格式，程序化生成

### ✅ Windows 资源文件配置
- **文件**: `build.rs` - Windows构建脚本
- **功能**: 设置应用程序元数据和图标资源
- **依赖**: `winres` crate (仅Windows平台)

### ✅ 桌面快捷方式
- **脚本**: `scripts/create_desktop_shortcut.bat`
- **功能**: 自动创建桌面快捷方式
- **图标**: 使用应用程序可执行文件的内嵌图标

## 文件结构

```
cliverge/
├── crates/cliverge-gui/
│   ├── assets/
│   │   ├── icon.svg              # 源SVG图标设计
│   │   ├── icon_32x32.png        # 32x32 PNG图标
│   │   └── app_icon.ico          # Windows ICO图标 (待创建)
│   ├── build.rs                  # Windows资源构建脚本
│   └── src/main.rs               # 图标加载和应用配置
├── scripts/
│   ├── create_simple_icon.py     # Python图标生成脚本
│   ├── generate_icons.py         # 完整图标生成脚本
│   └── create_desktop_shortcut.bat # Windows桌面快捷方式创建
└── docs/
    └── ICON_GUIDE.md             # 本指南文件
```

## 使用说明

### 构建应用程序
```bash
# 开发版本
cargo build -p cliverge

# 发布版本
cargo build --release -p cliverge
```

### 创建桌面快捷方式
```batch
# Windows
scripts\create_desktop_shortcut.bat
```

### 自定义图标

如果需要自定义图标，可以：

1. **修改SVG源文件**: 编辑 `assets/icon.svg`
2. **生成新图标**: 运行 `scripts/create_simple_icon.py`
3. **更新代码**: 修改 `src/main.rs` 中的 `load_icon()` 函数

## 图标规范

### 尺寸要求
- **窗口图标**: 32x32 像素（最小）
- **桌面图标**: 32x32, 48x48, 256x256 像素
- **ICO文件**: 支持多尺寸 (16, 24, 32, 48, 64, 128, 256)

### 色彩规范
- **主色调**: 蓝紫色渐变
- **终端色**: 深灰色系 (#1E293B, #374151)
- **强调色**: 青绿色 (#10B981, #06B6D4)

### 格式支持
- **SVG**: 矢量源文件，便于缩放
- **PNG**: 透明背景，适用于各种场景
- **ICO**: Windows专用，多尺寸支持

## 跨平台考虑

### Windows
- 使用 `winres` 设置资源文件
- 支持 `.ico` 格式的多尺寸图标
- 桌面快捷方式自动创建

### macOS
- 使用 `.icns` 格式（计划中）
- 应用程序包图标设置
- Dock 图标显示

### Linux
- 使用 `.png` 格式
- 符合 freedesktop.org 标准
- 支持各种桌面环境

## 故障排除

### 常见问题

1. **图标不显示**
   - 检查 `IconData` 导入是否正确
   - 确认图标数据格式为 RGBA
   - 验证图标尺寸设置

2. **Windows ICO文件缺失**
   - 运行 `python scripts/create_simple_icon.py`
   - 或手动创建 `assets/app_icon.ico`

3. **桌面快捷方式无图标**
   - 确保应用程序已构建
   - 运行 `scripts/create_desktop_shortcut.bat`
   - 检查快捷方式属性中的图标路径

### 调试步骤

1. 检查编译输出中的警告信息
2. 验证文件路径和权限
3. 确认依赖包正确安装
4. 测试不同平台的兼容性

## 更新日志

- **v0.1.0**: 初始图标系统实现
  - 程序化生成32x32应用图标
  - Windows资源文件配置
  - 桌面快捷方式支持
  - SVG源设计完成

## 后续计划

- [ ] 生成高分辨率ICO文件
- [ ] macOS .icns 文件支持
- [ ] Linux .desktop 文件集成
- [ ] 任务栏图标优化
- [ ] 启动画面图标集成
