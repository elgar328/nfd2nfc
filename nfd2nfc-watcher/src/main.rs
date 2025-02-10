mod config;
mod handler;
mod watcher;
use config::HOME_DIR;
use log::info;
use log::LevelFilter;
use once_cell::sync::Lazy;
use oslog::OsLogger;

#[tokio::main]
async fn main() {
    OsLogger::new("com.github.elgar328.nfd2nfc")
        .level_filter(LevelFilter::Info)
        .init()
        .unwrap();

    info!("Starting nfd2nfc-watcher daemon...");

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
