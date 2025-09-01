# CLIverge Tools.json 数据结构分析

## 概览

CLIverge 使用 JSON 配置文件来管理AI工具的定义和配置。主要的配置文件是 `configs/tools.json`，它包含了所有支持的AI工具的详细配置信息。

## 数据结构层次

```
ToolsConfig (根对象)
├── version: String              # 配置文件版本
├── last_updated: String         # 最后更新时间 (ISO 8601)
└── tools: Vec<ToolConfig>       # 工具配置数组
    └── ToolConfig
        ├── id: String                                    # 工具唯一标识符 (必填)
        ├── name: String                                  # 工具显示名称 (必填)
        ├── description: String                           # 工具描述 (必填)
        ├── website: String                               # 工具官网 (必填)
        ├── command: String                               # 工具执行命令 (必填)
        ├── version_check: Vec<String>                    # 版本检查命令参数 (必填)
        ├── update_check: Option<Vec<String>>             # 更新检查命令 (可选)
        ├── install: HashMap<String, InstallMethod>       # 平台安装方法 (必填)
        │   └── InstallMethod (按平台: windows/macos/linux)
        │       ├── method: String                        # 安装方法 (npm/brew/pip/script等)
        │       ├── command: Option<Vec<String>>          # 安装命令 (可选)
        │       ├── url: Option<String>                   # 脚本URL (可选)
        │       └── package_name: Option<String>          # 包名 (可选)
        └── config_schema: Option<HashMap<String, ConfigField>> # 配置字段定义 (可选)
            └── ConfigField
                ├── field_type: String                    # 字段类型 (string/enum/boolean/number)
                ├── secret: Option<bool>                  # 是否为敏感信息
                ├── required: Option<bool>                # 是否必填
                ├── description: String                   # 字段描述
                ├── default: Option<serde_json::Value>    # 默认值
                └── values: Option<Vec<String>>           # 枚举值选项
```

## 字段详细分析

### 1. 必填字段

| 字段名 | 类型 | 描述 | 示例 | 校验规则 |
|--------|------|------|------|----------|
| `id` | String | 工具唯一标识符 | `"claude-code"` | 必须唯一，建议使用kebab-case |
| `name` | String | 工具显示名称 | `"Claude Code CLI"` | 不能为空 |
| `description` | String | 工具描述 | `"Anthropic Claude AI Code Assistant"` | 不能为空 |
| `website` | String | 工具官网URL | `"https://claude.ai/code"` | 必须是有效URL |
| `command` | String | 执行命令 | `"claude"` | 不能为空 |
| `version_check` | Vec<String> | 版本检查参数 | `["--version"]` | 不能为空数组 |
| `install` | HashMap | 平台安装方法 | 见下方 | 至少包含当前平台 |

### 2. 可选字段

| 字段名 | 类型 | 描述 | 示例 |
|--------|------|------|------|
| `update_check` | Option<Vec<String>> | 更新检查命令 | `["claude", "update", "--check-only"]` |
| `config_schema` | Option<HashMap> | 配置字段定义 | 见下方ConfigField |

### 3. 安装方法 (InstallMethod)

支持的安装方法类型：

| 方法 | 描述 | 适用平台 | 必需字段 |
|------|------|----------|----------|
| `npm` | Node.js包管理器 | 所有平台 | `command` 或 `package_name` |
| `brew` | macOS Homebrew | macOS, Linux | `command` 或 `package_name` |
| `pip` | Python包管理器 | 所有平台 | `command` 或 `package_name` |
| `script` | 自定义脚本安装 | 所有平台 | `url` 或 `command` |
| `winget` | Windows包管理器 | Windows | `package_name` |

### 4. 配置字段类型 (ConfigField)

支持的字段类型：

| 类型 | 描述 | 特殊属性 |
|------|------|----------|
| `string` | 字符串 | `secret` - 敏感信息标记 |
| `enum` | 枚举选择 | `values` - 可选值数组 |
| `boolean` | 布尔值 | `default` - 默认值 |
| `number` | 数字 | `default` - 默认值 |

## 平台专用字段

### 当前支持的平台标识符：
- `windows` - Windows系统
- `macos` - macOS系统  
- `linux` - Linux系统

### 平台差异处理：
1. **安装方法差异**：不同平台可能使用不同的包管理器
2. **命令路径差异**：Windows可能需要特殊处理 (PowerShell封装)
3. **依赖检查**：某些平台可能需要额外的前置条件检查

## 数据示例

```json
{
  "id": "claude-code",
  "name": "Claude Code CLI", 
  "description": "Anthropic Claude AI Code Assistant",
  "website": "https://claude.ai/code",
  "command": "claude",
  "version_check": ["--version"],
  "update_check": ["claude", "update", "--check-only"],
  "install": {
    "windows": {
      "method": "npm",
      "command": ["npm", "install", "-g", "@anthropic-ai/claude-cli"],
      "package_name": "@anthropic-ai/claude-cli"
    },
    "macos": {
      "method": "brew", 
      "command": ["brew", "install", "claude-cli"],
      "package_name": "claude-cli"
    }
  },
  "config_schema": {
    "api_key": {
      "field_type": "string",
      "secret": true,
      "required": true,
      "description": "Anthropic API Key"
    }
  }
}
```

## 校验规则

### 数据完整性校验：
1. **必填字段检查** - 所有必填字段必须存在且不为空
2. **ID唯一性检查** - 工具ID在整个配置文件中必须唯一
3. **URL格式校验** - website字段必须是有效的HTTP/HTTPS URL
4. **平台兼容性检查** - 至少要包含当前运行平台的安装方法
5. **命令可用性检查** - version_check命令应该能够成功执行
6. **依赖关系校验** - 安装方法的依赖工具应该可用 (如npm, brew等)

### 数据类型校验：
1. **枚举值检查** - ConfigField中的enum类型必须提供values数组
2. **默认值类型匹配** - default值必须与field_type兼容
3. **安装方法完整性** - 每个InstallMethod必须包含足够的信息来执行安装

## 扩展设计考虑

### 未来可能添加的字段：
- `tags`: Vec<String> - 工具标签分类
- `dependencies`: Vec<String> - 依赖的其他工具
- `conflicts`: Vec<String> - 冲突的工具列表
- `min_version`: String - 支持的最低版本要求
- `documentation`: HashMap<String, String> - 多语言文档链接
- `license`: String - 工具许可证信息
- `maintainer`: String - 维护者信息

### 向后兼容性：
- 新字段应设计为可选字段
- 提供默认值处理机制
- 版本迁移脚本支持

## 错误处理模式

### 常见错误类型：
1. `ConfigError::InvalidFormat` - JSON格式错误
2. `ConfigError::MissingField` - 缺少必填字段
3. `ConfigError::InvalidValue` - 字段值不符合要求
4. `ConfigError::DuplicateId` - 工具ID重复
5. `ConfigError::UnsupportedPlatform` - 不支持的平台

### 错误恢复策略：
1. 部分配置损坏时跳过该工具，继续加载其他工具
2. 提供默认值填充机制
3. 用户友好的错误消息提示具体问题位置

---

*此文档基于CLIverge v0.1.0的实现分析，随项目发展可能会有更新*
