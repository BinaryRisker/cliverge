#!/usr/bin/env powershell
# Quick test script to verify cargo-dist configuration

Write-Host "=== Testing cargo-dist configuration ===" -ForegroundColor Green

try {
    # Test if cargo-dist can plan properly
    Write-Host "1. Testing cargo dist plan..." -ForegroundColor Yellow
    $planOutput = cargo dist plan 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ cargo dist plan succeeded" -ForegroundColor Green
        Write-Host $planOutput
    } else {
        Write-Host "❌ cargo dist plan failed:" -ForegroundColor Red
        Write-Host $planOutput
    }

    # Check if the binary exists in the workspace
    Write-Host "`n2. Checking binary configuration..." -ForegroundColor Yellow
    $cargoToml = Get-Content "crates/cliverge-gui/Cargo.toml" -Raw
    if ($cargoToml -match 'name = "cliverge"') {
        Write-Host "✅ Binary name 'cliverge' found in Cargo.toml" -ForegroundColor Green
    } else {
        Write-Host "❌ Binary name not found correctly" -ForegroundColor Red
    }

    # Check workspace dist config
    Write-Host "`n3. Checking workspace dist config..." -ForegroundColor Yellow
    $workspaceToml = Get-Content "Cargo.toml" -Raw
    if ($workspaceToml -match 'publish = \["cliverge"\]') {
        Write-Host "✅ Publish configuration found" -ForegroundColor Green
    } else {
        Write-Host "❌ Publish configuration missing" -ForegroundColor Red
    }

    Write-Host "`n=== Configuration check complete ===" -ForegroundColor Green
    
} catch {
    Write-Host "Error during test: $_" -ForegroundColor Red
}