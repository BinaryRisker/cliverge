#!/usr/bin/env bash
# Direct tag creation script to test release workflow

set -e

echo "=== Creating and pushing tag for release testing ==="

# Get current version from Cargo.toml
VERSION=$(grep '^version = ' crates/cliverge-gui/Cargo.toml | sed 's/version = "\(.*\)"/v\1/')
echo "Current version: $VERSION"

# Check if tag already exists locally
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    echo "⚠️ Tag $VERSION already exists locally, deleting..."
    git tag -d "$VERSION"
fi

# Check if tag exists on remote
if git ls-remote --tags origin | grep -q "refs/tags/$VERSION"; then
    echo "⚠️ Tag $VERSION exists on remote, deleting..."
    git push origin ":refs/tags/$VERSION"
    sleep 3
fi

# Create new annotated tag
echo "Creating annotated tag: $VERSION"
git tag -a "$VERSION" -m "Release $VERSION

Fix cargo-dist and workflow configuration:
- Fix workflow triggering issues between auto-release and release workflows
- Add workflow_dispatch to release.yml for manual triggering
- Improve error handling and debugging output
- Add proper tag format handling for manual triggers

This should generate all platform binaries correctly."

# Push tag to trigger release workflow
echo "Pushing tag $VERSION to origin..."
if git push origin "$VERSION"; then
    echo "✅ Tag $VERSION pushed successfully!"
    echo "🚀 Release workflow should be triggered automatically"
    echo "Monitor progress at: https://github.com/$(git config --get remote.origin.url | sed 's/.*github.com[:\/]\(.*\)\.git/\1/')/actions"
    
    # Wait and verify
    echo "Waiting 5 seconds for tag propagation..."
    sleep 5
    
    if git ls-remote --tags origin | grep -q "refs/tags/$VERSION"; then
        echo "✅ Tag verified on remote repository"
    else
        echo "⚠️ Warning: Could not verify tag on remote"
    fi
else
    echo "❌ Failed to push tag"
    exit 1
fi

echo "=== Tag creation complete ==="