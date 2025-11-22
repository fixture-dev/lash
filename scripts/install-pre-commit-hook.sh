#!/bin/bash
# Install pre-commit hook for Lash

set -e

HOOK_SOURCE="scripts/pre-commit"
HOOK_TARGET=".git/hooks/pre-commit"

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Check if the hook source exists
if [ ! -f "$HOOK_SOURCE" ]; then
    echo "Error: Hook source not found: $HOOK_SOURCE"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p ".git/hooks"

# Install the hook
echo "Installing pre-commit hook..."
cp "$HOOK_SOURCE" "$HOOK_TARGET"
chmod +x "$HOOK_TARGET"

echo "✅ Pre-commit hook installed successfully!"
echo ""
echo "The hook will run before each commit and check:"
echo "  - Code formatting (rustfmt)"
echo "  - Lint errors (clippy)"
echo "  - Unit tests"
echo "  - Doc tests"
echo ""
echo "To skip the hook (not recommended), use: git commit --no-verify"
