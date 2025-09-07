# 版本管理脚本 (PowerShell)
# 使用方法: .\scripts\bump-version.ps1 [major|minor|patch]

param(
    [string]$BumpType
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot

# 获取当前版本
function Get-CurrentVersion {
    $cargoToml = Get-Content "$ProjectRoot\crates\cliverge-gui\Cargo.toml"
    $versionLine = $cargoToml | Select-String '^version = '
    if ($versionLine) {
        return $versionLine.ToString() -replace '^version = "([^"]*)".*', '$1'
    }
    throw "无法找到版本信息"
}

# 更新版本号
function Update-Version {
    param([string]$Type)
    
    $currentVersion = Get-CurrentVersion
    Write-Host "当前版本: $currentVersion"
    
    # 解析版本号
    $versionParts = $currentVersion.Split('.')
    $major = [int]$versionParts[0]
    $minor = [int]$versionParts[1]
    $patch = [int]$versionParts[2]
    
    # 根据类型增加版本号
    switch ($Type) {
        "major" {
            $major++
            $minor = 0
            $patch = 0
        }
        "minor" {
            $minor++
            $patch = 0
        }
        "patch" {
            $patch++
        }
        default {
            Write-Error "错误: 无效的版本类型。请使用 major, minor, 或 patch"
            exit 1
        }
    }
    
    $newVersion = "$major.$minor.$patch"
    Write-Host "新版本: $newVersion"
    
    # 更新 Cargo.toml 文件
    $guiCargoToml = "$ProjectRoot\crates\cliverge-gui\Cargo.toml"
    $coreCargoToml = "$ProjectRoot\crates\cliverge-core\Cargo.toml"
    
    (Get-Content $guiCargoToml) -replace '^version = ".*"', "version = `"$newVersion`"" | Set-Content $guiCargoToml
    (Get-Content $coreCargoToml) -replace '^version = ".*"', "version = `"$newVersion`"" | Set-Content $coreCargoToml
    
    Write-Host "✅ 版本已更新到 $newVersion" -ForegroundColor Green
    Write-Host "💡 提示: 现在提交并推送到主分支将自动触发发布" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "建议的命令:"
    Write-Host "  git add ."
    Write-Host "  git commit -m `"chore: bump version to $newVersion`""
    Write-Host "  git push origin main"
}

# 显示帮助信息
function Show-Help {
    Write-Host "版本管理工具"
    Write-Host ""
    Write-Host "使用方法:"
    Write-Host "  .\scripts\bump-version.ps1 [major|minor|patch]"
    Write-Host ""
    Write-Host "参数:"
    Write-Host "  major    主版本号 (x.0.0)"
    Write-Host "  minor    次版本号 (0.x.0)"
    Write-Host "  patch    补丁版本号 (0.0.x)"
    Write-Host ""
    Write-Host "示例:"
    Write-Host "  .\scripts\bump-version.ps1 patch   # 0.1.0 -> 0.1.1"
    Write-Host "  .\scripts\bump-version.ps1 minor   # 0.1.1 -> 0.2.0"
    Write-Host "  .\scripts\bump-version.ps1 major   # 0.2.0 -> 1.0.0"
}

# 主函数
if (-not $BumpType) {
    try {
        $currentVersion = Get-CurrentVersion
        Write-Host "当前版本: $currentVersion"
        Write-Host ""
        Show-Help
    }
    catch {
        Write-Error $_
    }
}
else {
    switch ($BumpType.ToLower()) {
        "help" { Show-Help }
        "-h" { Show-Help }
        "--help" { Show-Help }
        "major" { Update-Version $BumpType }
        "minor" { Update-Version $BumpType }
        "patch" { Update-Version $BumpType }
        default {
            Write-Error "错误: 未知参数 '$BumpType'"
            Write-Host ""
            Show-Help
        }
    }
}