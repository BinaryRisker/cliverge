#!/usr/bin/env powershell
# Manual test script to create and push a tag for release testing

Write-Host "=== Manual Tag Creation for Release Testing ===" -ForegroundColor Green

try {
    # Get current version from Cargo.toml
    $cargoToml = Get-Content "crates/cliverge-gui/Cargo.toml" -Raw
    if ($cargoToml -match 'version = "([^"]+)"') {
        $version = "v" + $matches[1]
        Write-Host "Current version: $version" -ForegroundColor Yellow
    } else {
        Write-Host "❌ Could not find version in Cargo.toml" -ForegroundColor Red
        exit 1
    }

    # Check if tag already exists
    $existingTag = git tag -l $version
    if ($existingTag) {
        Write-Host "⚠️ Tag $version already exists locally" -ForegroundColor Yellow
        Write-Host "Deleting existing tag..." -ForegroundColor Yellow
        git tag -d $version
    }

    # Check remote tags
    Write-Host "Checking remote tags..." -ForegroundColor Yellow
    $remoteTags = git ls-remote --tags origin
    if ($remoteTags -match "refs/tags/$version") {
        Write-Host "⚠️ Tag $version exists on remote, deleting..." -ForegroundColor Yellow
        git push origin ":refs/tags/$version"
        Start-Sleep -Seconds 3
    }

    # Create new tag
    Write-Host "Creating new tag: $version" -ForegroundColor Green
    $tagMessage = @"
Release $version

## Changes
- Fix cargo-dist configuration for binary release
- Add publish field to specify package for release  
- Enable checksum generation and dist build profile
- This should fix the missing executable files issue

## Installation
Download the appropriate package for your platform from the release assets.
"@

    git tag -a $version -m $tagMessage

    # Push tag
    Write-Host "Pushing tag to origin..." -ForegroundColor Green
    git push origin $version

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Tag $version pushed successfully!" -ForegroundColor Green
        Write-Host "🚀 Release workflow should be triggered automatically" -ForegroundColor Green
        Write-Host "Monitor progress at: https://github.com/你的用户名/cliverge/actions" -ForegroundColor Cyan
        
        # Wait and verify
        Write-Host "Waiting 5 seconds for tag propagation..." -ForegroundColor Yellow
        Start-Sleep -Seconds 5
        
        $verifyTags = git ls-remote --tags origin
        if ($verifyTags -match "refs/tags/$version") {
            Write-Host "✅ Tag verified on remote repository" -ForegroundColor Green
        } else {
            Write-Host "⚠️ Warning: Could not verify tag on remote" -ForegroundColor Yellow
        }
    } else {
        Write-Host "❌ Failed to push tag" -ForegroundColor Red
        exit 1
    }

} catch {
    Write-Host "Error: $_" -ForegroundColor Red
    exit 1
}