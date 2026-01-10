# Releasing goa

This document describes the release process for goa (gitops-agent).

## Prerequisites

- Commit access to the repository
- `cargo` installed with crates.io credentials (`cargo login`)
- `gh` CLI installed and authenticated

## Version Numbering

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes to CLI interface or behavior
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

Pre-releases use suffixes: `0.1.0-rc.1`, `0.1.0-alpha.1`

## Release Process

### 1. Prepare the Release

```bash
# Checkout develop and ensure it's up to date
git checkout develop && git pull

# Create a release branch
git checkout -b release/X.Y.Z

# Update version in Cargo.toml
# Edit: version = "X.Y.Z"

# Run tests and lint
make test
make lint

# Verify crates.io publish will work
cargo publish --dry-run
```

### 2. Create Release PR

```bash
git add Cargo.toml
git commit -m "chore: release X.Y.Z"
git push -u origin release/X.Y.Z
gh pr create --title "chore: release X.Y.Z" --body "Release X.Y.Z"
```

### 3. Merge and Tag

After PR is approved and merged:

```bash
# Pull the merged changes
git checkout develop && git pull

# Create and push the release tag
git tag vX.Y.Z
git push origin vX.Y.Z
```

### 4. Automated Binary Builds

Pushing the tag triggers `.github/workflows/release.yml` which:
- Builds binaries for all supported platforms
- Creates a GitHub Release with changelog
- Uploads binaries as release assets

**Supported platforms:**
| Asset | Platform |
|-------|----------|
| `goa_amd64` | Linux x86_64 (Ubuntu) |
| `goa_aarch64` | Linux ARM 64-bit |
| `goa_arm` | Linux ARM 32-bit |
| `goa_centos_7` | CentOS 7 |
| `goa.exe` | Windows |
| `goa_darwin` | macOS x86_64 |

### 5. Publish to crates.io

After the GitHub Release is created:

```bash
# Publish to crates.io
cargo publish
```

Verify at: https://crates.io/crates/gitops-agent

### 6. Verify Release

- [ ] GitHub Release exists with all 6 binaries
- [ ] Changelog is accurate
- [ ] crates.io page is updated
- [ ] `cargo install gitops-agent` works
- [ ] Downloaded binaries execute correctly

## Hotfix Process

For urgent fixes to a released version:

```bash
# Create hotfix branch from the release tag
git checkout -b hotfix/X.Y.Z vX.Y.Z

# Make fixes, then follow normal release process
# Increment PATCH version (X.Y.Z+1)
```

## Pre-release Process

For release candidates or alpha/beta releases:

```bash
# Use pre-release version format
version = "X.Y.Z-rc.1"

# Tag with full version
git tag vX.Y.Z-rc.1
git push origin vX.Y.Z-rc.1

# Publish to crates.io (will be marked as pre-release)
cargo publish
```

Users can install pre-releases with:
```bash
cargo install gitops-agent@X.Y.Z-rc.1
```
