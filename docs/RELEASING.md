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
git push origin vX.Y.Z
```

### 3. Write release notes

After the automated pipeline creates the GitHub Release, update the auto-generated release notes.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Available sections:

- **Added** — new features
- **Changed** — changes in existing functionality (improvements, refactoring, docs, etc.)
- **Deprecated** — soon-to-be removed features
- **Removed** — removed features
- **Fixed** — bug fixes
- **Security** — vulnerability fixes

Omit sections with no entries. Additional notes (e.g., git history rewrite warnings) can be written outside the sections.

```bash
gh release edit vX.Y.Z --notes "$(cat <<'EOF'
## What's Changed

### Added
- ...

### Changed
- ...

### Fixed
- ...

**Full Changelog**: https://github.com/elgar328/nfd2nfc/compare/vPREV...vX.Y.Z
EOF
)"
```

See [v2.0.2](https://github.com/elgar328/nfd2nfc/releases/tag/v2.0.2) for an example.

### 4. Set the next dev version

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
- [ ] Write release notes (Step 3) and verify with `gh release view vX.Y.Z`
- [ ] Verify the Homebrew tap was updated: `brew update && brew info elgar328/nfd2nfc/nfd2nfc`
- [ ] Confirm installation works: `brew upgrade nfd2nfc` or `brew install elgar328/nfd2nfc/nfd2nfc`

## Updating Screenshots

Screenshots are hosted as GitHub Release assets (not tracked in git).
Local working files are in the `assets/` directory (gitignored).

```bash
# Update specific images (overwrites existing)
gh release upload assets assets/home.png assets/config.png --clobber

# Or update all screenshots at once
gh release upload assets assets/*.png --clobber
```

## Release Artifacts

- **Tarball**: `nfd2nfc-{VERSION}-universal-apple-darwin.tar.gz` (contains `nfd2nfc` and `nfd2nfc-watcher` universal binaries)
- **Formula template**: `.github/formula-template.rb` — the release workflow substitutes `PLACEHOLDER_TAG`, `PLACEHOLDER_VERSION`, and `PLACEHOLDER_SHA256` placeholders with actual values
