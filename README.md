# nfd2nfc
nfd2nfc is a macOS CLI tool that converts filenames between NFD and NFC for consistent cross-platform compatibility. It also includes a background service that continuously monitors specified folders and automatically applies the necessary conversions in real time.


## Conversion mode

**nfd2nfc** converts filenames between NFD and NFC to ensure consistent cross-platform compatibility. For background monitoring, use the watch subcommand. See `nfd2nfc watch --help` for details.

<img width="750" alt="Image" src="https://github.com/user-attachments/assets/e45d7fb6-1a64-42b3-a99c-5b7600c05473" />

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


## Watch mode

**nfd2nfc-watcher** operates as a background service that continuously monitors specified paths and converts filesystem entry names from NFD to NFC. Manage this service with the `nfd2nfc watch` subcommand. For further details, run `nfd2nfc watch <COMMAND> --help`.

<img width="750" alt="Image" src="https://github.com/user-attachments/assets/d65bd952-47f9-4f8a-b8f9-7e950af56f9f" />


## Installation

You can install nfd2nfc via Homebrew using our tap. This installation includes two executables (nfd2nfc and nfd2nfc-watcher) and automatically registers the watcher service during installation.

### Steps

1.	**Add the Homebrew Tap:**
``` bash 
brew tap elgar328/nfd2nfc
```

2.	**Install nfd2nfc:**
``` bash
brew install nfd2nfc
```

3.	**Start the Service:**

After installation, register and start the background service by running:
``` bash
brew services start nfd2nfc
```

Alternatively, if you don’t need a background service, you can run the watcher manually:
``` bash
nfd2nfc-watcher
```

4.	**Managing the Watcher Service:**

By default, no paths are set to be monitored. To enable automatic filename conversion, you must add directories to the watch list. For example, to have the watcher automatically process your Desktop folder and all its subdirectories, run:
``` bash
nfd2nfc watch add ~/Desktop -r
```

After adding a watch path, you can verify that it was successfully added by running the command below. This command displays all paths currently being monitored, so you can quickly check that your Desktop folder is included.
``` bash
nfd2nfc watch list
```

For more details, see the help documentation:
``` bash
nfd2nfc -–help
nfd2nfc watch -–help
```
