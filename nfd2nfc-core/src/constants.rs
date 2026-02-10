use std::time::Duration;

use crate::utils::expand_tilde;
use log::error;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::process;

pub const NFD2NFC_SERVICE_LABEL: &str = "homebrew.mxcl.nfd2nfc";

/// Base heartbeat interval in milliseconds. All other heartbeat timing constants are derived from this.
const HEARTBEAT_BASE_MS: u64 = 500;

/// How often the watcher writes the heartbeat file.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(HEARTBEAT_BASE_MS);
/// Maximum age of the heartbeat file before the watcher is considered dead (1.5x base interval).
pub const HEARTBEAT_MAX_AGE: Duration = Duration::from_millis(HEARTBEAT_BASE_MS * 3 / 2);
/// How often the TUI checks the heartbeat file (same as base interval).
pub const HEARTBEAT_CHECK_INTERVAL: Duration = Duration::from_millis(HEARTBEAT_BASE_MS);

pub static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| expand_tilde("~/.config/nfd2nfc/config.toml"));

pub static HEARTBEAT_PATH: Lazy<PathBuf> = Lazy::new(|| {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("nfd2nfc")
        .join("heartbeat")
});

pub static HOME_DIR: Lazy<PathBuf> = Lazy::new(|| match dirs::home_dir() {
    Some(path) => path,
    None => {
        error!("HOME environment variable is not set.");
        // Exit normally to prevent auto-restart.
        process::exit(0);
    }
});

pub fn plist_path() -> PathBuf {
    HOME_DIR
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", NFD2NFC_SERVICE_LABEL))
}

/// Plist path (exits with error if not found).
pub static PLIST_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = plist_path();
    if !path.exists() {
        error!("Plist file not found. Please run 'brew services start nfd2nfc'.");
        process::exit(1);
    }
    path
});
