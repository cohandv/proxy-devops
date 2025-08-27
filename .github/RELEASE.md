# Release Process

This document describes how to create releases for the Proxy project.

## Automated Release Process

### Creating a Release

1. **Tag the release**:
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

2. **GitHub Actions automatically**:
   - Builds CLI binaries for all supported platforms
   - Builds plugin libraries for all supported platforms
   - Creates a GitHub release
   - Uploads all artifacts
   - Generates checksums

### Manual Release Trigger

You can also trigger a release manually:

1. Go to GitHub Actions
2. Select "Release" workflow
3. Click "Run workflow"
4. Enter the tag version (e.g., `v0.1.0`)

## Supported Platforms

### CLI Binaries
- `proxy-linux-x86_64` - Linux x86_64
- `proxy-linux-aarch64` - Linux ARM64
- `proxy-macos-x86_64` - macOS Intel
- `proxy-macos-aarch64` - macOS Apple Silicon
- `proxy-windows-x86_64.exe` - Windows x86_64

### Plugin Libraries
- `plugins-linux-x86_64.tar.gz` - Linux x86_64 plugins
- `plugins-linux-aarch64.tar.gz` - Linux ARM64 plugins
- `plugins-macos-x86_64.tar.gz` - macOS Intel plugins
- `plugins-macos-aarch64.tar.gz` - macOS Apple Silicon plugins
- `plugins-windows-x86_64.zip` - Windows x86_64 plugins

## Pre-Release Checklist

Before creating a release:

- [ ] Update version in `Cargo.toml`
- [ ] Update version in plugin `Cargo.toml` files
- [ ] Update `CHANGELOG.md` if it exists
- [ ] Test on multiple platforms
- [ ] Run security audit: `cargo audit`
- [ ] Run all tests: `cargo test --all-features --workspace`
- [ ] Update documentation if needed

## Release Notes Template

When creating a release, use this template:

```markdown
## Changes in this Release

### 🚀 New Features
- New feature descriptions

### 🐛 Bug Fixes
- Bug fix descriptions

### 🔧 Improvements
- Improvement descriptions

### 📦 Dependencies
- Dependency updates

### CLI Tool
- Multi-platform support (Linux, macOS, Windows)
- Both x86_64 and ARM64 architectures

### Plugins
- plugin_name: Description of plugin functionality

### Installation

#### CLI Tool
Download the appropriate binary for your platform:
- Linux x86_64: `proxy-linux-x86_64`
- Linux ARM64: `proxy-linux-aarch64`
- macOS x86_64: `proxy-macos-x86_64`
- macOS ARM64: `proxy-macos-aarch64`
- Windows x86_64: `proxy-windows-x86_64.exe`

#### Plugins
Download the plugin package for your platform and extract to:
- Linux/macOS: `~/.cohandv/proxy/plugins/`
- Windows: `%USERPROFILE%\.cohandv\proxy\plugins\`

### Usage
```bash
# Make executable (Linux/macOS)
chmod +x proxy-*

# Run CLI
./proxy-* --help

# List available plugins
./proxy-* --help

# Use specific plugin
./proxy-* plugin_name --help
```

### Verification
Download `SHA256SUMS` to verify file integrity:
```bash
sha256sum -c SHA256SUMS
```
```

## Post-Release Tasks

After a release is created:

- [ ] Test download and installation on different platforms
- [ ] Update any external documentation
- [ ] Announce the release (if applicable)
- [ ] Close related issues/milestones

## Troubleshooting

### Build Failures

If builds fail:
1. Check the GitHub Actions logs
2. Test locally with the same Rust version
3. Ensure all dependencies are compatible
4. Check for platform-specific issues

### Missing Artifacts

If artifacts are missing:
1. Check that the workflow completed successfully
2. Verify the artifact upload steps succeeded
3. Ensure proper permissions for GitHub token

### Version Conflicts

If there are version conflicts:
1. Ensure all Cargo.toml files have consistent versions
2. Check that the git tag matches the Cargo.toml version
3. Verify no duplicate tags exist
