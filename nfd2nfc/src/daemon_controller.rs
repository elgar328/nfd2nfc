use log::{error, info};
use nfd2nfc_common::config::{read_or_default_config, RawConfig};
use nfd2nfc_common::constants::{
    CONFIG_PATH, HOME_DIR, NFD2NFC_SERVICE_LABEL, WATCHER_LIVE_MESSAGE,
};
use nfd2nfc_common::utils::expand_tilde;
use once_cell::sync::Lazy;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

pub static PLIST_PATH: Lazy<String> = Lazy::new(|| {
    let path = format!(
        "{}/Library/LaunchAgents/{}.plist",
        HOME_DIR.display(),
        NFD2NFC_SERVICE_LABEL
    );
    let plist_path = std::path::Path::new(&path);
    if !plist_path.exists() {
        error!("Plist file not found at {}.", path);
        std::process::exit(1);
    }
    path
});

pub fn cmd_start_watcher() {
    if check_watcher_status() {
        println!("nfd2nfc-watcher service is already running.");
        std::process::exit(0);
    }

    match launch_watcher_and_confirm() {
        Ok(_) => {
            println!("nfd2nfc-watcher service started.");
        }
        Err(err_logs) => {
            error!("{}\nFailed to start nfd2nfc-watcher service.", err_logs);
            std::process::exit(1);
        }
    }
}

pub fn cmd_stop_watcher() {
    if !check_watcher_status() {
        println!("nfd2nfc-watcher service is not running.");
        std::process::exit(0);
    }
    unload_watcher_service();
    println!("nfd2nfc-watcher service stopped.");
}

pub fn cmd_restart_watcher() {
    if !check_watcher_status() {
        println!("nfd2nfc-watcher service is not running.");
        return;
    }
    unload_watcher_service();
    match launch_watcher_and_confirm() {
        Ok(_) => {
            println!("nfd2nfc-watcher service restarted.");
        }
        Err(err_logs) => {
            error!("{}\nFailed to restart nfd2nfc-watcher service.", err_logs);
            std::process::exit(1);
        }
    }
}

pub fn cmd_status_watcher() {
    if check_watcher_status() {
        println!("nfd2nfc-watcher service is running.");
    } else {
        println!("nfd2nfc-watcher service is not running.");
    }
}

pub fn cmd_remove_watch_path_all() {
    delete_config();
    println!("All watch paths have been deleted.");
}

pub fn cmd_list_watch_paths() {
    let raw_config = match read_or_default_config(&*CONFIG_PATH) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to read config: {}", e);
            std::process::exit(1);
        }
    };

    println!("[Recursive Watch Paths]");
    for path in raw_config.recursive_watch_paths {
        println!(" - {}", path);
    }
    println!();

    println!("[Non-Recursive Watch Paths]");
    for path in raw_config.non_recursive_watch_paths {
        println!(" - {}", path);
    }
    println!();

    println!("[Recursive Ignore Paths]");
    for path in raw_config.recursive_ignore_paths {
        println!(" - {}", path);
    }
}

pub fn cmd_add_watch_path(path: &str, mode: WatchMode) {
    let canonical_path = resolve_and_canonicalize(path);

    // Read the current RawConfig from CONFIG_PATH.
    let mut raw_config: RawConfig = match read_or_default_config(&*CONFIG_PATH) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to read config: {}", e);
            std::process::exit(1);
        }
    };

    // Add the new path to the appropriate section.
    match mode {
        WatchMode::Recursive => {
            raw_config
                .recursive_watch_paths
                .push(canonical_path.clone());
        }
        WatchMode::NonRecursive => {
            raw_config
                .non_recursive_watch_paths
                .push(canonical_path.clone());
        }
        WatchMode::Ignore => {
            raw_config
                .recursive_ignore_paths
                .push(canonical_path.clone());
        }
    }

    // Update the config file.
    if let Err(e) = raw_config.save_to_file(CONFIG_PATH.as_path()) {
        error!("Failed to save updated config: {}", e);
        std::process::exit(1);
    }

    // Determine a human-friendly description for the mode.
    let mode_desc = match mode {
        WatchMode::Recursive => "recursive watch",
        WatchMode::NonRecursive => "non-recursive watch",
        WatchMode::Ignore => "ignore",
    };

    println!("Successfully added {} path: {}", mode_desc, canonical_path);

    // Reload config to apply changes.
    reload_config();
}

pub fn cmd_remove_watch_path(path: &str) {
    let canonical_path = resolve_and_canonicalize(path);

    // Read the current RawConfig from CONFIG_PATH.
    let mut raw_config: RawConfig = match read_or_default_config(&*CONFIG_PATH) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to read config: {}", e);
            std::process::exit(1);
        }
    };

    // Remove the canonical path from each section if present.
    let mut found = false;

    {
        let mut remove_from = |section: &str, paths: &mut Vec<String>| {
            let initial = paths.len();
            paths.retain(|p| p != &canonical_path);
            if paths.len() < initial {
                info!("Removed '{}' from {} paths.", canonical_path, section);
                found = true;
            }
        };

        remove_from("recursive watch", &mut raw_config.recursive_watch_paths);
        remove_from(
            "non-recursive watch",
            &mut raw_config.non_recursive_watch_paths,
        );
        remove_from("ignore", &mut raw_config.recursive_ignore_paths);
    }

    // Save the updated config.
    if let Err(e) = raw_config.save_to_file(CONFIG_PATH.as_path()) {
        error!("Failed to save updated config: {}", e);
        std::process::exit(1);
    }

    if found {
        println!("Successfully removed watch path: {}", canonical_path);
    } else {
        println!(
            "No matching watch path '{}' found in config.",
            canonical_path
        );
    }

    // Reload config to apply changes.
    reload_config();
}

pub fn launch_watcher_and_confirm() -> Result<String, String> {
    // 1. Start reading the log stream before loading the watcher.
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    std::thread::spawn(move || {
        log_stream_reader(tx);
    });

    // 2. Load the watcher service.
    let plist = &*PLIST_PATH;
    let status = Command::new("launchctl")
        .arg("load")
        .arg("-w")
        .arg(plist)
        .status()
        .map_err(|e| format!("Failed to start watcher: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to start watcher: {}", status));
    }

    // Poll logs until live message appears.
    let timeout = Duration::from_secs_f32(0.3);
    let mut logs_accumulated = String::new();

    loop {
        match rx.recv_timeout(timeout) {
            Ok(msg) => {
                logs_accumulated.push_str(&msg);
                logs_accumulated.push('\n');
                if msg.contains(WATCHER_LIVE_MESSAGE) {
                    return Ok(logs_accumulated);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if check_watcher_status() {
                    continue;
                } else {
                    logs_accumulated.push_str("\nTimeout reached and watcher not running.");
                    return Err(logs_accumulated);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Err(logs_accumulated)
}

fn unload_watcher_service() {
    let plist = &*PLIST_PATH;
    let status = Command::new("launchctl")
        .arg("unload")
        .arg("-w")
        .arg(plist)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            error!("Failed to stop service: {}", s);
            std::process::exit(1);
        }
        Err(e) => {
            error!("Failed to stop service: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn check_watcher_status() -> bool {
    let output = Command::new("launchctl")
        .arg("list")
        .output()
        .unwrap_or_else(|e| {
            error!("Failed to execute launchctl list: {}", e);
            std::process::exit(1);
        });
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains(NFD2NFC_SERVICE_LABEL) {
        true
    } else {
        false
    }
}

pub fn reload_config() {
    let result = if check_watcher_status() {
        unload_watcher_service();
        launch_watcher_and_confirm()
    } else {
        let res = launch_watcher_and_confirm();
        unload_watcher_service();
        res
    };

    match result {
        Ok(logs) => {
            for line in logs.lines() {
                if line.starts_with(" - ") {
                    println!("{}", line);
                }
            }
        }
        Err(err_logs) => {
            error!("{}\nFailed to start nfd2nfc-watcher service.", err_logs);
            std::process::exit(1);
        }
    }
}

pub fn cmd_stream_logs() {
    println!("nfd2nfc: Streaming log output... (Press Ctrl+C to exit)");

    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    std::thread::spawn(move || {
        log_stream_reader(tx);
    });

    for message in rx {
        println!("{}", message);
    }
}

fn log_stream_reader(tx: Sender<String>) {
    let mut child = Command::new("log")
        .args(&[
            "stream",
            "--predicate",
            &format!("subsystem == \"{}\"", NFD2NFC_SERVICE_LABEL),
            "--style",
            "json",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start log streaming");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        match line {
            Ok(l) => {
                const EVENT_MESSAGE_PREFIX: &str = "\"eventMessage\" : ";
                const EVENT_MESSAGE_SUFFIX: &str = "\",";
                if let Some(prefix_idx) = l.find(EVENT_MESSAGE_PREFIX) {
                    let message_start = prefix_idx + EVENT_MESSAGE_PREFIX.len();
                    if let Some(relative_end_idx) = l[message_start..].rfind(EVENT_MESSAGE_SUFFIX) {
                        let message_end = message_start + relative_end_idx + 1;
                        let message_escaped = &l[message_start..message_end];
                        let unescaped: String = serde_json::from_str(message_escaped)
                            .unwrap_or_else(|_| message_escaped.to_string());
                        if tx.send(unescaped).is_err() {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error reading log: {}", e);
                break;
            }
        }
    }
    let _ = child.wait();
}

pub fn cmd_log_history(duration: &str) {
    let predicate = format!("subsystem == \"{}\"", NFD2NFC_SERVICE_LABEL);
    let output = Command::new("log")
        .args(&[
            "show",
            "--predicate",
            &predicate,
            "--last",
            duration,
            "--style",
            "compact",
        ])
        .output()
        .unwrap_or_else(|e| {
            error!("Failed to execute log show command: {}", e);
            std::process::exit(1);
        });
    let logs = String::from_utf8_lossy(&output.stdout);
    println!("{}", logs);
}

#[derive(Debug)]
pub enum WatchMode {
    Recursive,
    NonRecursive,
    Ignore,
}

fn resolve_and_canonicalize(path: &str) -> String {
    // Expand tilde if present.
    let expanded_path = expand_tilde(path);

    // Resolve relative paths using the current working directory.
    let resolved_path = {
        if expanded_path.is_absolute() {
            expanded_path.to_string_lossy().into_owned()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(expanded_path).to_string_lossy().into_owned(),
                Err(e) => {
                    log::error!("Error getting current directory: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    // Convert the resolved path to its canonical form.
    match std::fs::canonicalize(&resolved_path) {
        Ok(canon) => canon.to_string_lossy().into_owned(),
        Err(e) => {
            log::error!("Error canonicalizing path '{}': {}", resolved_path, e);
            std::process::exit(1);
        }
    }
}

fn delete_config() {
    let config_path = Path::new(&*CONFIG_PATH);

    if config_path.exists() {
        match std::fs::remove_file(config_path) {
            Ok(_) => info!(
                "Config file {} deleted successfully.",
                config_path.display()
            ),
            Err(e) => {
                error!(
                    "Failed to delete config file {}: {}",
                    config_path.display(),
                    e
                );
                std::process::exit(1);
            }
        }
    } else {
        error!("Config file {} does not exist.", config_path.display());
        std::process::exit(1);
    }
}
