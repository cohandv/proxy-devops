#!/bin/bash
# Setup script for pre-commit hooks

set -e

echo "🔧 Setting up pre-commit hooks for Rust project..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "❌ pre-commit not found. Installing..."

    # Try different installation methods
    if command -v pip &> /dev/null; then
        echo "📦 Installing pre-commit via pip..."
        pip install pre-commit
    elif command -v brew &> /dev/null; then
        echo "🍺 Installing pre-commit via Homebrew..."
        brew install pre-commit
    elif command -v apt-get &> /dev/null; then
        echo "📦 Installing pre-commit via apt..."
        sudo apt-get update && sudo apt-get install -y python3-pip
        pip3 install pre-commit
    else
        echo "❌ Could not find a package manager to install pre-commit."
        echo "Please install pre-commit manually:"
        echo "  https://pre-commit.com/#installation"
        exit 1
    fi
fi

# Verify pre-commit is now available
if ! command -v pre-commit &> /dev/null; then
    echo "❌ pre-commit installation failed or not in PATH"
    exit 1
fi

echo "✅ pre-commit found: $(pre-commit --version)"

# Install the git hook scripts
echo "🪝 Installing pre-commit hooks..."
pre-commit install

# Run hooks on all files to test setup
echo "🧪 Testing pre-commit hooks on all files..."
if pre-commit run --all-files; then
    echo "✅ All pre-commit hooks passed!"
else
    echo "⚠️  Some pre-commit hooks failed. Please fix the issues and commit again."
    echo "💡 You can also run 'pre-commit run --all-files' to test all hooks manually."
fi

echo ""
echo "🎉 Pre-commit setup complete!"
echo ""
echo "ℹ️  The following hooks will now run before each commit:"
echo "   • trailing-whitespace: Remove trailing whitespace"
echo "   • end-of-file-fixer: Ensure files end with newline"
echo "   • check-yaml: Validate YAML files"
echo "   • check-toml: Validate TOML files"
echo "   • check-merge-conflict: Check for merge conflict markers"
echo "   • cargo fmt: Format Rust code"
echo "   • cargo clippy: Lint Rust code"
echo "   • cargo test: Run Rust tests"
echo ""
echo "💡 To skip hooks for a specific commit, use: git commit --no-verify"
echo "💡 To run hooks manually: pre-commit run --all-files"
