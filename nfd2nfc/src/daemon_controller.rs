use nfd2nfc_core::constants::{HEARTBEAT_MAX_AGE, HEARTBEAT_PATH, PLIST_PATH, plist_path};
use std::process::Command;
use std::time::Duration;

fn run_launchctl(subcommand: &str, action_desc: &str) -> Result<(), String> {
    let status = Command::new("launchctl")
        .args([subcommand, "-w"])
        .arg(&*PLIST_PATH)
        .status()
        .map_err(|e| format!("Failed to {action_desc}: {e}"))?;
    if !status.success() {
        return Err(format!("Failed to {action_desc}: {status}"));
    }
    Ok(())
}

fn launch_watcher_and_confirm() -> Result<(), String> {
    run_launchctl("load", "start watcher")?;

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
    run_launchctl("unload", "stop service")
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
    plist_path().exists()
}

/// Check if the current binary is newer than the installed plist.
fn is_plist_stale() -> bool {
    let exe_mtime = std::env::current_exe()
        .and_then(std::fs::metadata)
        .and_then(|m| m.modified())
        .ok();
    let plist_mtime = std::fs::metadata(&*PLIST_PATH)
        .and_then(|m| m.modified())
        .ok();

    match (exe_mtime, plist_mtime) {
        (Some(exe), Some(plist)) => exe > plist,
        _ => false,
    }
}

fn run_brew_services(subcommand: &str) -> Result<(), String> {
    let status = Command::new("brew")
        .args(["services", subcommand, "nfd2nfc"])
        .status()
        .map_err(|e| format!("Failed to run brew command: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("brew services {subcommand} failed: {status}"))
    }
}

/// Ensure the plist is installed and up to date.
/// - If missing: run `brew services start nfd2nfc`
/// - If stale (binary newer than plist): run `brew services restart nfd2nfc`
pub fn ensure_plist_up_to_date() -> Result<(), String> {
    if !is_plist_installed() {
        println!("Service not registered. Running 'brew services start nfd2nfc'...");
        run_brew_services("start")?;
        println!("Service registered successfully.");
        return Ok(());
    }

    if is_plist_stale() {
        println!("Upgrade detected. Running 'brew services restart nfd2nfc'...");
        run_brew_services("restart")?;
        println!("Service restarted with updated configuration.");
    }

    Ok(())
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
