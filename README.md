# CLIverge - Universal CLI Tool Manager

<p align="center">
  <strong>Lightweight • Visual • Cross-Platform</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#installation">Installation</a> •
  <a href="#supported-tools">Supported Tools</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <a href="README_zh.md">中文</a> | 
  <strong>English</strong>
</p>

---

## 📖 About CLIverge

**CLIverge** is a lightweight desktop CLI tool management platform designed to simplify the installation, management, and usage of command-line tools across all platforms. Through an intuitive GUI interface, users can easily manage various development tools, DevOps utilities, system tools, multimedia tools, and more without memorizing complex command-line operations.

### Core Concept

- **CLI** + **verge** (convergence) = **CLIverge**
- A unified management solution that converges various CLI tools into one platform
- Support for tools across all categories: development, system administration, DevOps, multimedia, and more

## ✨ Features

### ✅ Implemented Features

- ✅ **Modern GUI Interface** - Native desktop app built with Egui
- ✅ **Tool Status Detection** - Real-time detection of tool installation status and version information
- ✅ **Smart Caching System** - Cache tool status, version info, and help documentation
- ✅ **Configuration Management** - Manage tools via JSON config files without dynamic loading
- ✅ **Async Architecture** - High-performance async execution based on Tokio
- ✅ **Cross-Platform Support** - Native support for Windows, macOS, and Linux
- ✅ **Multi-Theme Support** - Light/Dark theme switching
- ✅ **Real-time Notifications** - Operation feedback and status update notifications
- ✅ **Unified Log System** - Resizable bottom panel with text selection and copy support
- ✅ **Ultra-Lightweight** - Optimized to only 1.5MB (73.9% size reduction)

### 🚧 In Development

- ✅ **One-Click Install/Uninstall** - Smart platform detection with optimal installation methods (Basic functionality completed)
- 🚧 **Version Update Checker** - Automatic tool update checking
- 🚧 **Tool Configuration Management** - Visual configuration of tool parameters
- 🚧 **Integrated Terminal** - Built-in terminal emulator

## 🔧 Supported Tools

CLIverge supports management of command-line tools across multiple categories:

### 🤖 AI & Machine Learning
| Tool Name | Description | Status |
|-----------|-------------|-------|
| **Claude Code CLI** | Anthropic Claude AI code assistant | ✅ Full Support |
| **Gemini CLI** | Google Gemini multimodal AI assistant | ✅ Full Support |
| **Qwen Code CLI** | Alibaba Cloud Qwen code version | ✅ Basic Support |
| **OpenAI CLI** | Official OpenAI command-line tool | ✅ Basic Support |

### 💻 Development Tools
| Tool Name | Description | Status |
|-----------|-------------|-------|
| **Cursor CLI** | Cursor editor command-line tool | ✅ Basic Support |
| **OpenCode** | Open-source code generation tool | ✅ Basic Support |
| **Git** | Distributed version control system | 🔄 Planned |
| **Node.js/npm** | JavaScript runtime and package manager | 🔄 Planned |
| **Python/pip** | Python interpreter and package manager | 🔄 Planned |

### ⚙️ DevOps & System Tools
| Tool Name | Description | Status |
|-----------|-------------|-------|
| **Docker** | Container platform | 🔄 Planned |
| **kubectl** | Kubernetes command-line tool | 🔄 Planned |
| **Terraform** | Infrastructure as code tool | 🔄 Planned |
| **Ansible** | Automation and configuration management | 🔄 Planned |

### 📦 Package Managers
| Tool Name | Description | Status |
|-----------|-------------|-------|
| **Homebrew** | macOS package manager | 🔄 Planned |
| **Chocolatey** | Windows package manager | 🔄 Planned |
| **Cargo** | Rust package manager | 🔄 Planned |
| **Composer** | PHP dependency manager | 🔄 Planned |

### 📊 Data & Workflow Tools
| Tool Name | Description | Status |
|-----------|-------------|-------|
| **iFlow CLI** | Intelligent workflow automation tool | ✅ Basic Support |
| **jq** | JSON processor | 🔄 Planned |
| **curl** | Data transfer tool | 🔄 Planned |
| **wget** | Web file retrieval tool | 🔄 Planned |

*✅ Full Support: Complete install/uninstall/version management*  
*✅ Basic Support: Basic install/version detection*  
*🔄 Planned: Future support planned*

## 🚀 Quick Start

### System Requirements

- **Operating System**: Windows 10+, macOS 10.15+, Linux (mainstream distributions)
- **Runtime**: No runtime dependencies required
- **Memory**: Minimum 2GB RAM
- **Disk Space**: 2MB

### Development Requirements

- **Rust**: 1.82.0 or newer (MSRV)
- **Cargo**: Latest stable

### Installation

#### Option 1: Download Pre-built Binaries (Recommended)

Visit the [Releases](https://github.com/yourusername/cliverge/releases) page and download the appropriate installer for your platform:

- **Windows**: Download `.msi` installer or `.exe` file
- **macOS**: Download `.dmg` installer or `.tar.xz` archive
- **Linux**: Download `.deb`/`.rpm` packages or `.tar.xz` archive

#### Option 2: Install via Script

**Shell (Linux/macOS):**
```bash
curl -fsSL https://github.com/yourusername/cliverge/releases/latest/download/install.sh | sh
```

**PowerShell (Windows):**
```powershell
irm https://github.com/yourusername/cliverge/releases/latest/download/install.ps1 | iex
```

#### Option 3: Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/cliverge.git
cd cliverge

# Build the project (optimized version)
cargo build --profile release-min -p cliverge

# Optional: Compress with UPX for minimal size
upx --best target/release-min/cliverge.exe

# Run the application
cargo run --profile release-min -p cliverge
```

### Usage Guide

1. **Launch Application**: Double-click the executable or run from terminal
2. **Browse Tools**: Left panel displays all available CLI tools
3. **Check Status**: Automatically detects each tool's installation status
4. **Install Tools**: Click "Install" button for one-click installation
5. **Manage Configuration**: Configure tool parameters in Settings

## 🏗 Architecture

### Tech Stack

- **Core Language**: Rust (2021 Edition)
- **GUI Framework**: Egui (immediate mode GUI)
- **Async Runtime**: Tokio
- **Serialization**: Serde + JSON
- **Caching System**: Custom JSON cache

### Module Structure

```
cliverge/
├── crates/
│   ├── cliverge-gui/      # GUI application main module
│   │   ├── src/
│   │   │   ├── main.rs    # Application entry point
│   │   │   └── app.rs     # Main application logic
│   │   └── Cargo.toml
│   └── cliverge-core/     # Core service layer
│       ├── src/
│       │   ├── lib.rs     # Module exports
│       │   ├── config.rs  # Configuration management
│       │   ├── tool.rs    # Tool management
│       │   ├── version.rs # Version checking
│       │   ├── cache.rs   # Caching system
│       │   └── error.rs   # Error handling
│       └── Cargo.toml
├── configs/
│   ├── tools.json         # Tool configuration file
│   └── settings.json      # Application settings template
└── Cargo.toml            # Workspace configuration
```

### Design Principles

1. **Lightweight First** - Minimize complexity, focus on core functionality
2. **Configuration-Driven** - Manage tools via JSON config files, avoid complex plugin systems
3. **User Experience First** - Clean and intuitive interface, lower the barrier to entry
4. **Performance Optimized** - Async execution, smart caching, fast response

## 📊 Project Status

### Completion Assessment

| Module | Completion | Notes |
|--------|------------|-------|
| **GUI Interface** | 90% | Main interface complete, log system optimized |
| **Core Engine** | 70% | Basic functionality implemented, advanced features in development |
| **Tool Management** | 85% | Status detection complete, install/uninstall functionality implemented |
| **Caching System** | 90% | Complete caching mechanism implemented |
| **Configuration Management** | 80% | Config read/write complete, UI editor in development |
| **Version Checking** | 40% | Basic architecture complete, strategy implementation in progress |

### Performance Metrics

- **Startup Time**: < 3 seconds
- **Memory Usage**: < 35MB (idle)
- **CPU Usage**: < 1% (idle)
- **Binary Size**: 1.5MB (73.9% reduction achieved)

## 🎯 Development Roadmap

### Short-term Goals (1-2 weeks)

- [x] Complete tool install/uninstall functionality ✅ **Completed**
- [x] Optimize log system and user feedback ✅ **Completed**
- [ ] Implement version update checking mechanism
- [ ] Add tool configuration editor

### Medium-term Goals (1 month)

- [ ] Integrate terminal emulator
- [ ] Support more CLI tools across different categories
- [ ] Implement tool usage statistics
- [ ] Add tool recommendation features

### Long-term Goals (3 months)

- [ ] Plugin marketplace (simplified version)
- [ ] Cloud configuration sync
- [ ] Workflow automation
- [ ] Multi-language support

## 🤝 Contributing

Contributions are welcome! Feel free to contribute code, report issues, or suggest improvements.

### How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Submit a Pull Request

### 🚀 Automated Release Process

CLIverge uses an automated release system that creates new releases whenever you push changes to the main branch:

#### For Maintainers: How to Release

1. **Update Version**: Use the provided version management scripts
   ```bash
   # Linux/macOS
   ./scripts/bump-version.sh patch   # 0.1.0 -> 0.1.1
   ./scripts/bump-version.sh minor   # 0.1.0 -> 0.2.0
   ./scripts/bump-version.sh major   # 0.1.0 -> 1.0.0
   
   # Windows
   .\scripts\bump-version.ps1 patch
   .\scripts\bump-version.ps1 minor
   .\scripts\bump-version.ps1 major
   ```

2. **Commit and Push**: The version change will automatically trigger a release
   ```bash
   git add .
   git commit -m "chore: bump version to v0.1.1"
   git push origin main
   ```

3. **Automated Process**: The CI will:
   - Detect the version change
   - Create a git tag
   - Build cross-platform binaries (Windows, macOS, Linux)
   - Generate installers (MSI, DMG, DEB/RPM)
   - Create a GitHub Release with download links
   - Generate changelog from commit history

#### Release Artifacts

Each release automatically includes:
- **Windows**: `.msi` installer and `.exe` executable
- **macOS**: `.dmg` installer and `.tar.xz` archive
- **Linux**: `.deb`/`.rpm` packages and `.tar.xz` archive
- **Install Scripts**: One-line installation scripts for all platforms
- **Checksums**: SHA256 checksums for all artifacts

#### Manual Release (if needed)

You can also trigger a release manually:
```bash
# Create and push a version tag
git tag v0.1.1
git push origin v0.1.1

# Or use GitHub Actions workflow dispatch
# Go to Actions -> Auto Release -> Run workflow
```

1. Fork this repository
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Submit a Pull Request

### Development Environment Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
cargo install cargo-watch cargo-edit

# Run in development mode
cargo watch -x 'run -p cliverge'
```

### Code Standards

- Follow Rust official style guidelines
- Use `cargo fmt` to format code
- Use `cargo clippy` for code linting
- Write documentation comments for public APIs

## 🏆 Optimization Achievements

CLIverge has undergone extensive optimization to achieve minimal binary size while maintaining full functionality:

- **Original Size**: 5.88MB
- **Optimized Size**: 1.53MB
- **Size Reduction**: 73.9%
- **Optimization Techniques**: Dependency pruning, feature gating, custom algorithms, LTO, UPX compression

For detailed optimization information, see [PHASE3_FINAL_OPTIMIZATION.md](docs/PHASE3_FINAL_OPTIMIZATION.md).

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Egui](https://github.com/emilk/egui) - Excellent immediate mode GUI framework
- [Tokio](https://tokio.rs/) - Powerful async runtime
- All CLI tool developers and the open-source community

## 📞 Contact

- Project Homepage: [https://github.com/yourusername/cliverge](https://github.com/yourusername/cliverge)
- Issue Reports: [Issues](https://github.com/yourusername/cliverge/issues)
- Discussions: [Discussions](https://github.com/yourusername/cliverge/discussions)

---

<p align="center">
  Made with ❤️ by CLIverge Team
</p>