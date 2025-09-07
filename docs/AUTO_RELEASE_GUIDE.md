# CLIverge 自动发布系统使用说明

## 🎯 概述

CLIverge 项目配置了完全自动化的发布系统，当您推送代码到主分支时，会自动检测版本变更并创建新的发布版本。

## 🚀 如何触发自动发布

### 方法 1: 使用版本管理脚本（推荐）

项目提供了跨平台的版本管理脚本，可以自动更新版本号：

#### Linux/macOS
```bash
# 补丁版本 (0.1.0 -> 0.1.1)
./scripts/bump-version.sh patch

# 次版本 (0.1.1 -> 0.2.0)  
./scripts/bump-version.sh minor

# 主版本 (0.2.0 -> 1.0.0)
./scripts/bump-version.sh major
```

#### Windows
```powershell
# 补丁版本 (0.1.0 -> 0.1.1)
.\scripts\bump-version.ps1 patch

# 次版本 (0.1.1 -> 0.2.0)
.\scripts\bump-version.ps1 minor

# 主版本 (0.2.0 -> 1.0.0)
.\scripts\bump-version.ps1 major
```

### 方法 2: 手动编辑版本号

1. 编辑 `crates/cliverge-gui/Cargo.toml`
2. 编辑 `crates/cliverge-core/Cargo.toml`
3. 更新两个文件中的 `version = "x.x.x"` 字段

### 方法 3: 直接推送标签

```bash
# 创建版本标签
git tag v0.1.1

# 推送标签（将触发 cargo-dist 发布流程）
git push origin v0.1.1
```

## 🔄 自动化流程

当您推送到主分支后，系统会执行以下步骤：

### 第一阶段：版本检查
1. **检测版本变更**：比较 Cargo.toml 中的版本与最新 Release
2. **生成变更日志**：从 Git 提交历史自动生成
3. **创建标签**：如果版本有变更，自动创建 Git 标签

### 第二阶段：构建发布（由 cargo-dist 触发）
1. **跨平台构建**：
   - Windows (x86_64)
   - macOS (x86_64 + ARM64)
   - Linux (x86_64)

2. **生成安装包**：
   - Windows: `.msi` 安装包 + `.exe` 可执行文件
   - macOS: `.dmg` 安装包 + `.tar.xz` 压缩包
   - Linux: `.deb`/`.rpm` 包 + `.tar.xz` 压缩包

3. **创建发布**：
   - 自动上传所有构建产物
   - 生成 SHA256 校验和
   - 创建安装脚本
   - 发布到 GitHub Releases

## 📦 发布产物

每次自动发布都会包含：

### 可执行文件
- `cliverge-x86_64-pc-windows-msvc.exe` - Windows 可执行文件
- `cliverge-x86_64-apple-darwin.tar.xz` - macOS Intel 版本
- `cliverge-aarch64-apple-darwin.tar.xz` - macOS Apple Silicon 版本
- `cliverge-x86_64-unknown-linux-gnu.tar.xz` - Linux 版本

### 安装包
- `cliverge-x86_64-pc-windows-msvc.msi` - Windows MSI 安装包
- `cliverge-x86_64-apple-darwin.dmg` - macOS 安装包 (Intel)
- `cliverge-aarch64-apple-darwin.dmg` - macOS 安装包 (Apple Silicon)
- `cliverge_x.x.x_amd64.deb` - Debian/Ubuntu 包
- `cliverge-x.x.x-1.x86_64.rpm` - Red Hat/CentOS/Fedora 包

### 安装脚本
- `install.sh` - Linux/macOS 一键安装脚本
- `install.ps1` - Windows PowerShell 安装脚本

### 其他文件
- `checksums.txt` - SHA256 校验和文件
- Release Notes - 自动生成的版本说明

## 🔍 监控发布状态

### GitHub Actions
- 访问：`https://github.com/你的用户名/cliverge/actions`
- 查看 "Auto Release" 和 "Release" 工作流程

### GitHub Releases
- 访问：`https://github.com/你的用户名/cliverge/releases`
- 查看最新发布的版本和下载统计

## 🛠️ 故障排除

### 常见问题

1. **版本号没有变更**
   - 确保 Cargo.toml 中的版本号与最新 Release 不同
   - 检查是否正确更新了两个 crate 的版本

2. **构建失败**
   - 检查代码是否通过所有测试
   - 确保 CI 检查（格式化、Clippy）都通过

3. **标签创建失败**
   - 检查 GitHub 权限设置
   - 确保没有重复的标签

### 手动干预

如果自动发布失败，您可以：

1. **手动创建标签**：
   ```bash
   git tag v0.1.1
   git push origin v0.1.1
   ```

2. **使用 GitHub Actions 手动触发**：
   - 转到 Actions 页面
   - 选择 "Auto Release" 工作流程
   - 点击 "Run workflow"

3. **本地构建发布**：
   ```bash
   # 使用项目提供的脚本
   ./scripts/release.bat  # Windows
   ./scripts/build-optimized.sh  # Linux/macOS
   ```

## 📝 最佳实践

1. **版本管理**：
   - 使用语义化版本控制 (SemVer)
   - patch: 向后兼容的错误修复
   - minor: 向后兼容的新功能
   - major: 破坏性变更

2. **提交信息**：
   - 使用清晰的提交信息
   - 遵循约定式提交 (Conventional Commits)
   - 示例：`feat: add new tool support`、`fix: resolve installation issue`

3. **发布频率**：
   - 定期发布小版本更新
   - 重大功能完成后发布次版本
   - 破坏性变更时发布主版本

4. **测试**：
   - 确保所有 CI 检查通过
   - 本地测试应用程序功能
   - 验证跨平台兼容性

## 🔧 配置文件

相关配置文件：
- `.github/workflows/auto-release.yml` - 自动发布工作流程
- `.github/workflows/release.yml` - cargo-dist 发布流程
- `Cargo.toml` - cargo-dist 配置
- `scripts/bump-version.sh` - Linux/macOS 版本管理脚本
- `scripts/bump-version.ps1` - Windows 版本管理脚本