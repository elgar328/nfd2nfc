# Development Environment Setup

## Overview

The Homebrew-installed version and local development builds share the same service label (`io.github.elgar328.nfd2nfc`), which causes conflicts. When switching between them, always clean up the active one first.

## Switching to Development (brew → dev)

```bash
# 1. Uninstall the Homebrew version (automatically stops the service and removes the plist)
brew uninstall nfd2nfc

# 2. Build the release binary
cargo build --release

# 3. Run the TUI (automatically generates the plist and registers the LaunchAgent)
cargo run --bin nfd2nfc
```

## Development Workflow

After making code changes:

- **TUI changes only**: Run `cargo run --bin nfd2nfc` to test immediately.
- **Watcher changes**: Run `cargo build --release` first, then restart the watcher from the TUI. If the plist is outdated, the TUI automatically restarts the service.

## Switching Back to Homebrew (dev → brew)

```bash
# 1. Start the watcher from the TUI and confirm it is running

# 2. Remove the development binary (the watcher detects removal and auto-cleans the plist)
cargo clean

# 3. Reinstall the Homebrew version
brew install elgar328/nfd2nfc/nfd2nfc
```
