# nfd2nfc
nfd2nfc is a macOS CLI tool that converts filenames between NFD and NFC for consistent cross-platform compatibility. It also includes a background service that continuously monitors specified folders and automatically applies the necessary conversions in real time.


⚠️ **Note:** The background service for automatic monitoring (`nfd2nfc-watcher`) is not yet implemented. Currently, only the manual conversion tool (`nfd2nfc`) is available.

## nfd2nfc

Help Documentation – a concise overview of nfd2nfc’s commands, options, and usage examples.

<img width="750" alt="Image" src="https://github.com/user-attachments/assets/8eab1691-745d-4136-9c85-48cecd09e8fa" />


### NFD vs. NFC
**Background & Test Setup:**  
- 10,000 sample folders and files were generated in the `tools/test_data` directory using the `tools/generate_test_data.py` script.
- A [zellij](https://zellij.dev) session running [yazi](https://github.com/sxyazi/yazi) was used to visually inspect the test data.
- Note: [zellij 0.41.2](https://github.com/zellij-org/zellij/releases/tag/v0.41.2) does not properly handle NFD normalization.
- Conversion commands used:
  - Convert from NFD to NFC: `nfd2nfc -r .`
  - Convert from NFC back to NFD: `nfd2nfc -rR .`

Below are screenshots that illustrate the conversion results.

<img width="373" alt="Image" src="https://github.com/user-attachments/assets/d1d55f90-ff66-4e87-9958-3f17ef954ca3" />

<img width="373" alt="Image" src="https://github.com/user-attachments/assets/a23c4d79-d33f-472f-af3c-456db31cf42e" />

## nfd2nfc-watcher

Not yet implemented.
