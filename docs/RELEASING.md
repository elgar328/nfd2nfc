# Release Guide

## Overview

Development happens directly on the `main` branch. During development, the version in `Cargo.toml` is set to `X.Y.Z-dev`. When ready to release, the `-dev` suffix is removed, the release is tagged, and the version is bumped to the next dev version.

## Release Steps

### 1. Finalize the version

Remove the `-dev` suffix from the version in `Cargo.toml`, then commit:

```bash
# Edit Cargo.toml: "X.Y.Z-dev" → "X.Y.Z"
cargo check
git add Cargo.toml Cargo.lock
git commit -m "Release vX.Y.Z"
git push origin main
```

### 2. Tag and push

Pushing a `v*` tag triggers the automated release pipeline:

```bash
git tag vX.Y.Z
git push origin --tags
```

### 3. Set the next dev version

Bump to the next development version:

```bash
# Edit Cargo.toml: "X.Y.Z" → "X.Y.(Z+1)-dev"
cargo check
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.(Z+1)-dev"
git push origin main
```

## Automated Pipeline

The `release.yml` workflow runs automatically when a `v*` tag is pushed. It performs the following:

1. **Build** — Compiles release binaries for both `x86_64-apple-darwin` and `aarch64-apple-darwin` targets
2. **Universal binary** — Merges the two architecture binaries into universal binaries using `lipo -create`
3. **Package** — Creates a tarball: `nfd2nfc-{VERSION}-universal-apple-darwin.tar.gz`
4. **GitHub Release** — Creates a GitHub Release with auto-generated release notes and attaches the tarball
5. **Homebrew tap update** — Clones `elgar328/homebrew-nfd2nfc`, applies the formula template (`.github/formula-template.rb`) with the new version and SHA256, then pushes the updated formula

## Post-release Checklist

- [ ] Verify the [GitHub Release](https://github.com/elgar328/nfd2nfc/releases) was created with the correct tarball
- [ ] Edit the release notes on GitHub if needed (auto-generated notes may need refinement)
- [ ] Verify the Homebrew tap was updated: `brew update && brew info elgar328/nfd2nfc/nfd2nfc`
- [ ] Confirm installation works: `brew upgrade nfd2nfc` or `brew install elgar328/nfd2nfc/nfd2nfc`

## Release Artifacts

- **Tarball**: `nfd2nfc-{VERSION}-universal-apple-darwin.tar.gz` (contains `nfd2nfc` and `nfd2nfc-watcher` universal binaries)
- **Formula template**: `.github/formula-template.rb` — the release workflow substitutes `PLACEHOLDER_TAG`, `PLACEHOLDER_VERSION`, and `PLACEHOLDER_SHA256` placeholders with actual values
