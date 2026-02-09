# Development Environment Setup

## Overview

The Homebrew-installed version and local development builds share the same plist path (`~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist`), which causes conflicts. During development, you must uninstall the Homebrew version and manually create a plist pointing to your local build.

## Switching to Development (brew → dev)

```bash
# 1. Uninstall the Homebrew version (automatically stops the service and removes the plist)
brew uninstall nfd2nfc

# 2. Build the release binary
cargo build --release

# 3. Create a development plist
cat > ~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>homebrew.mxcl.nfd2nfc</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/nfd2nfc/target/release/nfd2nfc-watcher</string>
    </array>
    <key>KeepAlive</key>
    <dict>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF

# 4. Run the TUI (you can start/stop the watcher from within the TUI)
cargo run --bin nfd2nfc
```

## Development Workflow

After making code changes:

- **TUI changes only**: Run `cargo run --bin nfd2nfc` to test immediately.
- **Watcher changes**: Run `cargo build --release` first, then restart the watcher from the TUI (the plist points to the release binary, so a rebuild is required).

## Switching Back to Homebrew (dev → brew)

```bash
# 1. Unload the LaunchAgent and delete the development plist
launchctl unload ~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist
rm ~/Library/LaunchAgents/homebrew.mxcl.nfd2nfc.plist

# 2. Reinstall the Homebrew version
brew install elgar328/nfd2nfc/nfd2nfc
```
