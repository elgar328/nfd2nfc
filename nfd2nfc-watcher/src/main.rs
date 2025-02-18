mod handler;
mod watcher;
use log::info;
use nfd2nfc_common::config;
use nfd2nfc_common::constants::HOME_DIR;
use nfd2nfc_common::logger::{init_logger, LogBackend};
use once_cell::sync::Lazy;

#[tokio::main]
async fn main() {
    let verbose = 3;
    init_logger(LogBackend::OSLog, verbose);
    info!("Launching nfd2nfc-watcher service...");

    // Force HOME_DIR initialization via lazy singleton.
    // If HOME is not set, exit normally (code 0) to prevent auto-restart.
    Lazy::force(&HOME_DIR);

    // Load the configuration file.
    // If configuration fails to load, exit normally (code 0) to prevent auto-restart.
    let config = config::load_config().unwrap_or_else(|_e| {
        std::process::exit(0);
    });

    let rt_handle = tokio::runtime::Handle::current();
    watcher::start_watcher(rt_handle, config).await;
}
