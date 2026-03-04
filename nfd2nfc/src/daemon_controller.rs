use nfd2nfc_core::constants::{
    HEARTBEAT_MAX_AGE, HEARTBEAT_PATH, LEGACY_PLIST_PATH, NFD2NFC_SERVICE_LABEL, PLIST_PATH,
};
use std::process::Command;
use std::time::Duration;

fn run_launchctl(
    subcommand: &str,
    plist: &std::path::Path,
    action_desc: &str,
) -> Result<(), String> {
    let status = Command::new("launchctl")
        .args([subcommand, "-w"])
        .arg(plist)
        .status()
        .map_err(|e| format!("Failed to {action_desc}: {e}"))?;
    if !status.success() {
        return Err(format!("Failed to {action_desc}: {status}"));
    }
    Ok(())
}

/// Remove any stale service registration from launchctl (by label, ignores errors).
fn remove_stale_service() {
    let _ = Command::new("launchctl")
        .args(["remove", NFD2NFC_SERVICE_LABEL])
        .status();
}

/// Remove stale registration then load the plist.
fn load_service(action_desc: &str) -> Result<(), String> {
    remove_stale_service();
    run_launchctl("load", &PLIST_PATH, action_desc)
}

fn launch_watcher_and_confirm() -> Result<(), String> {
    load_service("start watcher")?;

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
    run_launchctl("unload", &PLIST_PATH, "stop service")
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
    PLIST_PATH.exists()
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

/// Find the watcher binary path as a sibling of the current executable.
fn watcher_binary_path() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "Failed to get exe parent directory".to_string())?;
    let watcher = dir.join("nfd2nfc-watcher");
    if !watcher.exists() {
        return Err(format!("Watcher binary not found: {}", watcher.display()));
    }
    Ok(watcher)
}

/// Generate the launchd plist file for the watcher service.
fn generate_plist() -> Result<(), String> {
    let watcher_path = watcher_binary_path()?;
    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{watcher}</string>
    </array>
    <key>KeepAlive</key>
    <dict>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
        label = NFD2NFC_SERVICE_LABEL,
        watcher = watcher_path.display(),
    );

    // Ensure LaunchAgents directory exists.
    if let Some(parent) = PLIST_PATH.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create LaunchAgents directory: {e}"))?;
    }

    std::fs::write(&*PLIST_PATH, plist_content)
        .map_err(|e| format!("Failed to write plist: {e}"))?;

    Ok(())
}

/// Migrate from the legacy Homebrew-managed plist to the new self-managed plist.
fn migrate_legacy_plist() -> Result<bool, String> {
    if !LEGACY_PLIST_PATH.exists() {
        return Ok(false);
    }

    // Unload the legacy service (ignore errors — it may not be loaded).
    let _ = run_launchctl("unload", &LEGACY_PLIST_PATH, "unload legacy service");

    // Delete the legacy plist file.
    let _ = std::fs::remove_file(&*LEGACY_PLIST_PATH);

    // Generate new plist and load it.
    generate_plist()?;
    load_service("load migrated service")?;

    Ok(true)
}

/// Ensure the plist is installed and up to date.
/// - Legacy plist exists: migrate to new label
/// - Plist exists but stale (binary newer): regenerate and reload
/// - Plist missing: generate and load (first install)
pub fn ensure_plist_up_to_date() -> Result<(), String> {
    if migrate_legacy_plist()? {
        return Ok(());
    }

    if is_plist_installed() {
        if is_plist_stale() {
            let _ = unload_watcher_service();
            generate_plist()?;
            load_service("reload updated service")?;
        }
        return Ok(());
    }

    // Plist missing — first install, generate and start.
    generate_plist()?;
    load_service("start service")?;

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

    if !is_plist_installed() {
        generate_plist()?;
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
