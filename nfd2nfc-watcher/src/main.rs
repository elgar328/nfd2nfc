mod handler;
mod watcher;
use log::{debug, warn};
use nfd2nfc_core::config;
use nfd2nfc_core::constants::{HEARTBEAT_PATH, HOME_DIR};
use nfd2nfc_core::logger::{LogBackend, init_logger};
use once_cell::sync::Lazy;

#[tokio::main]
async fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("nfd2nfc-watcher {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let verbose = 3;
    init_logger(LogBackend::OSLog, verbose);

    // Force HOME_DIR initialization via lazy singleton.
    // If HOME is not set, exit normally (code 0) to prevent auto-restart.
    Lazy::force(&HOME_DIR);

    // Load the configuration file. Errors are ignored (empty config = no watch paths).
    let (config, _) = config::load_config();
    let active = config.active_entries();

    // Initialize heartbeat: create directory and initial file
    if let Some(parent) = HEARTBEAT_PATH.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        warn!("Failed to create heartbeat directory: {}", e);
    }
    if let Err(e) = std::fs::write(&*HEARTBEAT_PATH, "") {
        warn!("Failed to write initial heartbeat file: {}", e);
    }
    debug!("Heartbeat file initialized: {}", HEARTBEAT_PATH.display());

    let rt_handle = tokio::runtime::Handle::current();
    watcher::start_watcher(rt_handle, active).await;
}
