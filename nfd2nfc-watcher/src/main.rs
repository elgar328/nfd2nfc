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

    if std::env::args().any(|a| a == "--check-fda") {
        std::process::exit(check_fda());
    }

    let verbose = 3;
    init_logger(LogBackend::OSLog, verbose);

    // Force HOME_DIR initialization via lazy singleton.
    // If HOME is not set, exit normally (code 0) to prevent auto-restart.
    Lazy::force(&HOME_DIR);

    // Load the configuration file. On error, use empty config (no watch paths).
    let (config, config_error) = config::load_config();
    if let Some(e) = &config_error {
        warn!("Config load issue: {}", e);
    }
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

/// Check Full Disk Access by attempting to access FDA-protected paths.
/// Uses approach from inket/FullDiskAccess (macOS 12+) with multiple fallbacks.
/// Returns exit code: 0 = granted, 1 = not granted, 2 = unknown.
fn check_fda() -> i32 {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            println!("unknown");
            return 2;
        }
    };

    // FDA-protected paths to try in order.
    // If any is accessible → granted. If any exists but denied → not granted.
    // If none exist → unknown.
    let dir_paths = [
        home.join("Library/Containers/com.apple.stocks"), // macOS 12+
    ];
    let file_paths = [
        home.join("Library/Safari/CloudTabs.db"),
        home.join("Library/Messages/chat.db"),
        home.join("Library/Suggestions/snippets.db"),
    ];

    for path in &dir_paths {
        if path.exists() {
            match std::fs::read_dir(path) {
                Ok(_) => {
                    println!("granted");
                    return 0;
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    println!("not_granted");
                    return 1;
                }
                Err(_) => continue,
            }
        }
    }

    for path in &file_paths {
        match std::fs::File::open(path) {
            Ok(_) => {
                println!("granted");
                return 0;
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                println!("not_granted");
                return 1;
            }
            Err(_) => continue,
        }
    }

    println!("unknown");
    2
}
