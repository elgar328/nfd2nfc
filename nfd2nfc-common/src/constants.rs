use crate::utils::expand_tilde;
use log::error;
use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;
use std::process;

pub const WATCHER_LIVE_MESSAGE: &str = "nfd2nfc-watcher live: monitoring events.";
pub const NFD2NFC_SERVICE_LABEL: &str = "homebrew.mxcl.nfd2nfc";
pub const NFD2NFC_CONFIG_ENV: &str = "NFD2NFC_CONFIG";

pub static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    if let Ok(cfg) = env::var(NFD2NFC_CONFIG_ENV) {
        let trimmed = cfg.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    expand_tilde("~/.config/nfd2nfc/config.toml")
});

pub static HOME_DIR: Lazy<PathBuf> = Lazy::new(|| match dirs::home_dir() {
    Some(path) => path,
    None => {
        error!("HOME environment variable is not set.");
        // Exit normally to prevent auto-restart.
        process::exit(0);
    }
});
