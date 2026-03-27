# Development Guide

## Development Environment Setup

See [DEV-ENV-SETUP.md](DEV-ENV-SETUP.md) for switching between Homebrew and development builds.

## Test Data

Running `tools/generate_test_data.py` generates a test directory at `tools/test_data/` with NFD filenames in Korean, French, Japanese, Vietnamese, and more (10,000 entries by default). The output is deterministic, so it can be used to verify conversion behavior.

```bash
python tools/generate_test_data.py
```

## Testing with --HEAD

`brew install --HEAD nfd2nfc` installs directly from the main branch. This is useful for testing changes that require an actual Homebrew installation (e.g., service registration, plist paths, Full Disk Access behavior).

```bash
brew install --HEAD nfd2nfc
```

To switch back to the latest release:

```bash
brew uninstall nfd2nfc
brew install nfd2nfc
```

## Release

See [RELEASING.md](RELEASING.md) for the release process.
