# Building CLIverge

## Quick Build Guide

CLIverge provides optimized build scripts for different use cases:

### 🔧 Development Builds
```bash
# Interactive build with multiple options
scripts\build-optimized.bat

# Options:
# 1. release-min  - Ultra-optimized (1.5MB, recommended)
# 2. release      - Standard release (3MB)
# 3. dev          - Development build (larger, with debug info)
```

### 🚀 Production Releases
```bash
# Fully automated release build
scripts\release.bat

# Outputs:
# - UPX compressed binary (1.5MB)
# - Complete release package
# - Multi-language documentation
```

### 📊 Optimization Results

| Build Type | Size | Optimization | Use Case |
|------------|------|--------------|----------|
| **release-min** | 1.5MB | 73.9% reduction | Production deployment |
| **release** | 3.0MB | 49% reduction | Standard release |
| **dev** | 5.9MB | No optimization | Development/debugging |

### 🛠️ Manual Build Commands

```bash
# Minimal size build (recommended)
cargo build --profile release-min -p cliverge

# UPX compression (optional)
upx --best target/release-min/cliverge.exe

# Standard release
cargo build --release -p cliverge

# Development build
cargo build -p cliverge
```

### 📦 Build Profiles

The project uses custom optimization profiles defined in `Cargo.toml`:

- **release-min**: Ultra-optimized for size
  - Strip symbols
  - LTO (Link Time Optimization)
  - Size-focused optimization
  - Single codegen unit

- **release**: Standard optimizations
- **dev**: Fast compilation, debug symbols

### 🎯 Target Sizes

- **Original**: 5.88MB
- **Optimized**: 1.53MB (73.9% reduction)
- **Techniques**: Dependency pruning, custom algorithms, compiler optimization, UPX compression

For detailed optimization information, see [docs/PHASE3_FINAL_OPTIMIZATION.md](docs/PHASE3_FINAL_OPTIMIZATION.md).
