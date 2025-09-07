#!/usr/bin/env pwsh

# Test script for cargo-dist release process
Write-Host "🧪 Testing cargo-dist release configuration..." -ForegroundColor Cyan

# Check if cargo-dist is installed
if (!(Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Cargo not found. Please install Rust first." -ForegroundColor Red
    exit 1
}

# Build the project first
Write-Host "🔨 Building project..." -ForegroundColor Yellow
cargo build --release -p cliverge
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Build successful" -ForegroundColor Green

# Check if we can find the binary
$binaryPath = "target/release/cliverge.exe"
if (Test-Path $binaryPath) {
    $size = (Get-Item $binaryPath).Length / 1MB
    Write-Host "✅ Binary found: $binaryPath (${size:N1} MB)" -ForegroundColor Green
} else {
    Write-Host "❌ Binary not found at $binaryPath" -ForegroundColor Red
    exit 1
}

# Check workspace structure
Write-Host "📁 Checking workspace structure..." -ForegroundColor Yellow
$workspaceMembers = @()
if (Test-Path "crates/cliverge-core/Cargo.toml") {
    Write-Host "✅ Found: crates/cliverge-core" -ForegroundColor Green
    $workspaceMembers += "cliverge-core"
}
if (Test-Path "crates/cliverge-gui/Cargo.toml") {
    Write-Host "✅ Found: crates/cliverge-gui (binary package)" -ForegroundColor Green
    $workspaceMembers += "cliverge"
}

Write-Host "📦 Workspace members: $($workspaceMembers -join ', ')" -ForegroundColor Cyan

# Display current version
$version = (Select-String -Path "crates/cliverge-gui/Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
Write-Host "📋 Current version: v$version" -ForegroundColor Cyan

Write-Host "`n🎯 Ready for release!" -ForegroundColor Green
Write-Host "To create a release:" -ForegroundColor White
Write-Host "1. Update version in crates/cliverge-gui/Cargo.toml" -ForegroundColor Gray
Write-Host "2. Commit and push to main branch" -ForegroundColor Gray
Write-Host "3. The auto-release workflow will detect the version change" -ForegroundColor Gray
Write-Host "4. A tag will be created and cargo-dist will build the release" -ForegroundColor Gray