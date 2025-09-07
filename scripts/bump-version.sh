#!/bin/bash
# 版本管理脚本
# 使用方法: ./scripts/bump-version.sh [major|minor|patch]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# 获取当前版本
get_current_version() {
    grep '^version = ' "$PROJECT_ROOT/crates/cliverge-gui/Cargo.toml" | sed 's/version = "\(.*\)"/\1/'
}

# 更新版本号
bump_version() {
    local bump_type="$1"
    local current_version=$(get_current_version)
    
    echo "当前版本: $current_version"
    
    # 解析版本号
    IFS='.' read -ra VERSION_PARTS <<< "$current_version"
    local major="${VERSION_PARTS[0]}"
    local minor="${VERSION_PARTS[1]}"
    local patch="${VERSION_PARTS[2]}"
    
    # 根据类型增加版本号
    case "$bump_type" in
        "major")
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        "minor")
            minor=$((minor + 1))
            patch=0
            ;;
        "patch")
            patch=$((patch + 1))
            ;;
        *)
            echo "错误: 无效的版本类型。请使用 major, minor, 或 patch"
            exit 1
            ;;
    esac
    
    local new_version="$major.$minor.$patch"
    echo "新版本: $new_version"
    
    # 更新 Cargo.toml 文件
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" "$PROJECT_ROOT/crates/cliverge-gui/Cargo.toml"
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" "$PROJECT_ROOT/crates/cliverge-core/Cargo.toml"
    
    # 删除备份文件
    rm -f "$PROJECT_ROOT/crates/cliverge-gui/Cargo.toml.bak"
    rm -f "$PROJECT_ROOT/crates/cliverge-core/Cargo.toml.bak"
    
    echo "✅ 版本已更新到 $new_version"
    echo "💡 提示: 现在提交并推送到主分支将自动触发发布"
    echo ""
    echo "建议的命令:"
    echo "  git add ."
    echo "  git commit -m \"chore: bump version to $new_version\""
    echo "  git push origin main"
}

# 显示帮助信息
show_help() {
    echo "版本管理工具"
    echo ""
    echo "使用方法:"
    echo "  $0 [major|minor|patch]"
    echo ""
    echo "参数:"
    echo "  major    主版本号 (x.0.0)"
    echo "  minor    次版本号 (0.x.0)"
    echo "  patch    补丁版本号 (0.0.x)"
    echo ""
    echo "示例:"
    echo "  $0 patch   # 0.1.0 -> 0.1.1"
    echo "  $0 minor   # 0.1.1 -> 0.2.0"
    echo "  $0 major   # 0.2.0 -> 1.0.0"
}

# 主函数
main() {
    if [ $# -eq 0 ]; then
        echo "当前版本: $(get_current_version)"
        echo ""
        show_help
        exit 0
    fi
    
    case "$1" in
        "help"|"-h"|"--help")
            show_help
            ;;
        "major"|"minor"|"patch")
            bump_version "$1"
            ;;
        *)
            echo "错误: 未知参数 '$1'"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

main "$@"