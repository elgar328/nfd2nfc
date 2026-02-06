# nfd2nfc

[![Release](https://img.shields.io/github/v/release/elgar328/nfd2nfc)](https://github.com/elgar328/nfd2nfc/releases)
[![CI](https://github.com/elgar328/nfd2nfc/actions/workflows/ci.yml/badge.svg)](https://github.com/elgar328/nfd2nfc/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/elgar328/nfd2nfc)](https://github.com/elgar328/nfd2nfc#license)

[![한국어](https://img.shields.io/badge/lang-한국어_↗-blue)](docs/README.ko.md)

A macOS tool that watches directories and converts NFD filenames to NFC in real time, ensuring cross-platform compatibility.

## What is NFD/NFC?

Unicode has two main ways to represent composed characters such as Korean Hangul, accented Latin (é, ü, ñ), Japanese kana (が, ぱ), and more:

- **NFC** (Composed): One codepoint per character — `가` = `U+AC00`
- **NFD** (Decomposed): Base + combining marks — `가` = `U+1100 U+1161`

macOS stores filenames in NFD, while Windows and Linux use NFC. This mismatch causes problems when sharing files across platforms:

- Hangul syllables appear as decomposed jamo (e.g., 한글 → ㅎㅏㄴㄱㅡㄹ)
- Accented characters may render or sort incorrectly
- Git sees identical files as untracked or modified
- Cloud sync and archive tools fail to match paths

**nfd2nfc** provides an interactive TUI and a background watcher service that handles conversion automatically.

## Features

- **Real-time monitoring** — Converts NFD filenames to NFC as soon as they appear
- **Flexible watch paths** — Mix `watch`/`ignore` actions with `recursive`/`children` modes to control exactly which directories are monitored
- **Manual conversion** — Browse the filesystem and convert names between NFD and NFC on the spot
- **Log viewer** — Review past watcher logs or follow new entries live
- **Intuitive controls** — Every keybinding is shown on screen, and full mouse support is included

## Home

<img src="assets/home.png" alt="Home" width="750" />

The Home tab displays the current watcher service status and provides controls to start, stop, or restart it.

## Config

<img src="assets/config.png" alt="Config" width="750" />

The Config tab manages which directories the watcher monitors. Each path entry has an action (`watch` or `ignore`), mode (`recursive` or `children`), and validation status.

## Logs

<img src="assets/logs.png" alt="Logs" width="750" />

The Logs tab lets you browse past watcher logs and follow new entries in real time. Logs are stored in macOS system logs, and retention is managed by the OS.

## Browser

<img src="assets/browser.png" alt="Browser" width="750" />

The Browser tab lets you inspect files and directories for their Unicode normalization form and convert them directly. The watcher only picks up newly created or modified files, so use this tab to convert any existing NFD names.

## Installation

Requires [Homebrew](https://brew.sh).

```bash
brew install elgar328/nfd2nfc/nfd2nfc
```

Then run it:

```bash
nfd2nfc
```

On first launch, the watcher service is automatically registered via `brew services`. After that, you can start and stop it from the app.

### Permissions

If macOS repeatedly prompts for folder access when the watcher converts files, grant **Full Disk Access** to the watcher binary:

1. Run `which nfd2nfc-watcher` to find the binary path
2. Open **System Settings → Privacy & Security → Full Disk Access**
3. Click `+`, press `Cmd+Shift+G`, paste the path from step 1, and add it

## Upgrading from v1

```bash
brew upgrade nfd2nfc
```

v2 replaces the previous CLI with a more user-friendly TUI. The config format (`~/.config/nfd2nfc/config.toml`) has changed as well, so you will need to re-add your watch paths in the Config tab after upgrading.

## Uninstallation

```bash
brew uninstall nfd2nfc
```

## License

[MIT](LICENSE)
