use nfd2nfc_core::constants::{
    HEARTBEAT_MAX_AGE, HEARTBEAT_PATH, HOME_DIR, NFD2NFC_SERVICE_LABEL, PLIST_PATH,
};
use std::process::Command;
use std::time::Duration;

fn launch_watcher_and_confirm() -> Result<(), String> {
    // Load the watcher service (starts automatically with RunAtLoad: true).
    let status = Command::new("launchctl")
        .arg("load")
        .arg("-w")
        .arg(&*PLIST_PATH)
        .status()
        .map_err(|e| format!("Failed to start watcher: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to start watcher: {}", status));
    }

    // Wait for heartbeat file to be created/updated (max 5 seconds).
    for _ in 0..250 {
        std::thread::sleep(Duration::from_millis(20));
        if check_heartbeat_fresh(HEARTBEAT_MAX_AGE) {
            return Ok(());
        }
    }

    Err("Watcher started but heartbeat not detected".to_string())
}

fn unload_watcher_service() -> Result<(), String> {
    let status = Command::new("launchctl")
        .arg("unload")
        .arg("-w")
        .arg(&*PLIST_PATH)
        .status()
        .map_err(|e| format!("Failed to stop service: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to stop service: {}", status));
    }

    Ok(())
}

/// Check if heartbeat file exists and was modified within max_age.
fn check_heartbeat_fresh(max_age: Duration) -> bool {
    std::fs::metadata(&*HEARTBEAT_PATH)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().is_ok_and(|e| e < max_age))
        .unwrap_or(false)
}

/// Check if plist file exists.
fn is_plist_installed() -> bool {
    HOME_DIR
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", NFD2NFC_SERVICE_LABEL))
        .exists()
}

/// Register the service via `brew services start nfd2nfc` if the plist is not installed.
pub fn install_plist_if_missing() -> Result<(), String> {
    if is_plist_installed() {
        return Ok(());
    }

    println!("Service not registered. Running 'brew services start nfd2nfc'...");

    let status = Command::new("brew")
        .args(["services", "start", "nfd2nfc"])
        .status()
        .map_err(|e| format!("Failed to run brew command: {}", e))?;

    if status.success() {
        println!("Service registered successfully.");
        Ok(())
    } else {
        Err(format!(
            "Failed to register service (exit code: {})",
            status
        ))
    }
}

/// Check if the watcher is running by verifying heartbeat file mtime is recent.
pub fn check_watcher_status() -> bool {
    check_heartbeat_fresh(HEARTBEAT_MAX_AGE)
}

// TUI-specific functions that return Results instead of exiting

pub fn try_start_watcher() -> Result<(), String> {
    if check_watcher_status() {
        return Err("Watcher service is already running".to_string());
    }

    launch_watcher_and_confirm()
}

fn stop_watcher_internal() -> Result<(), String> {
    unload_watcher_service()?;
    let _ = std::fs::remove_file(&*HEARTBEAT_PATH);
    Ok(())
}

pub fn try_stop_watcher() -> Result<(), String> {
    if !check_watcher_status() {
        return Err("Watcher service is not running".to_string());
    }

    stop_watcher_internal()
}

pub fn try_restart_watcher() -> Result<(), String> {
    if !check_watcher_status() {
        return Err("Watcher service is not running".to_string());
    }

    stop_watcher_internal()?;
    launch_watcher_and_confirm()
}
