use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

use nfd2nfc_core::config::{self, PathAction, PathEntry, PathMode, PathStatus};
use nfd2nfc_core::constants::{CONFIG_PATH, NFD2NFC_SERVICE_LABEL, PLIST_PATH};
use nfd2nfc_core::normalizer::{self, NormalizationTarget, get_actual_file_name};
use nfd2nfc_core::utils::{abbreviate_home_path, expand_tilde};

use crate::daemon_controller;
use crate::log_service;
use crate::version;
use crate::{Command, ConfigAction, LogAction, WatcherAction};

pub fn run(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Command::Status { json } => cmd_status(json),
        Command::Watcher { action } => cmd_watcher(action),
        Command::Config { action } => cmd_config(action),
        Command::Convert {
            path,
            mode,
            target,
            dry_run,
            json,
        } => cmd_convert(&path, &mode, &target, dry_run, json),
        Command::Log { action } => cmd_log(action),
    }
}

// === Status ===

fn cmd_status(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let watcher_running = daemon_controller::check_watcher_status();
    let version = env!("CARGO_PKG_VERSION");
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| fs::canonicalize(&p).ok().or(Some(p)))
        .map(|p| abbreviate_home_path(&p))
        .unwrap_or_else(|| "unknown".to_string());

    // Watcher binary from plist
    let (watcher_binary_display, watcher_binary_raw, watcher_binary_exists) =
        get_watcher_binary_from_plist();

    // Config & Plist existence
    let config_path = abbreviate_home_path(&CONFIG_PATH);
    let config_exists = CONFIG_PATH.exists();
    let plist_path = abbreviate_home_path(&PLIST_PATH);
    let plist_exists = PLIST_PATH.exists();

    // Registered paths
    let (config, _) = config::load_config();
    let mut active = 0u32;
    let mut redundant = 0u32;
    let mut not_found = 0u32;
    let mut not_a_directory = 0u32;
    let mut permission_denied = 0u32;
    for entry in &config.paths {
        match entry.status {
            PathStatus::Active => active += 1,
            PathStatus::Redundant(_) => redundant += 1,
            PathStatus::NotFound => not_found += 1,
            PathStatus::NotADirectory => not_a_directory += 1,
            PathStatus::PermissionDenied => permission_denied += 1,
        }
    }
    let total = config.paths.len() as u32;

    // FDA check (use raw path for spawning)
    let fda = check_fda_via_watcher(&watcher_binary_raw);

    // Update check (with timeout)
    let update_status = check_update_status_with_timeout();

    if json {
        let watcher_binary_json = match &watcher_binary_display {
            Some(p) => format!(r#"{{"path":"{}","exists":{}}}"#, p, watcher_binary_exists),
            None => r#"{"path":null,"exists":false}"#.to_string(),
        };

        let (update_status_str, latest_version) = match &update_status {
            UpdateStatus::Available(ver) => ("available", format!(r#""{}""#, ver)),
            UpdateStatus::UpToDate(ver) => ("up_to_date", format!(r#""{}""#, ver)),
            UpdateStatus::Unknown => ("unknown", "null".to_string()),
        };

        println!(
            r#"{{"watcher":"{}","version":"{}","binary":"{}","watcher_binary":{},"config":{{"path":"{}","exists":{}}},"plist":{{"path":"{}","exists":{}}},"registered_paths":{{"total":{},"active":{},"redundant":{},"not_found":{},"not_a_directory":{},"permission_denied":{}}},"full_disk_access":"{}","update_status":"{}","latest_version":{}}}"#,
            if watcher_running {
                "running"
            } else {
                "stopped"
            },
            version,
            binary,
            watcher_binary_json,
            config_path,
            config_exists,
            plist_path,
            plist_exists,
            total,
            active,
            redundant,
            not_found,
            not_a_directory,
            permission_denied,
            fda,
            update_status_str,
            latest_version,
        );
    } else {
        println!(
            "Watcher:          {}",
            if watcher_running {
                "running"
            } else {
                "stopped"
            }
        );
        println!("Version:          {}", version);
        println!("Binary:           {}", binary);

        // Watcher binary
        match &watcher_binary_display {
            Some(p) if watcher_binary_exists => {
                println!("Watcher binary:   {}", p);
                // Check mismatch with sibling (both canonicalized)
                if let Ok(sibling) = daemon_controller::watcher_binary_path() {
                    let sibling_canonical =
                        fs::canonicalize(&sibling).unwrap_or_else(|_| sibling.clone());
                    let plist_canonical = watcher_binary_raw
                        .as_ref()
                        .and_then(|r| fs::canonicalize(r).ok());
                    if let Some(plist_c) = plist_canonical
                        && sibling_canonical != plist_c
                    {
                        let exe_dir = std::env::current_exe()
                            .ok()
                            .and_then(|e| e.parent().map(|p| p.display().to_string()))
                            .unwrap_or_default();
                        eprintln!(
                            "  ⚠ Mismatch: current nfd2nfc is at {}, but plist points elsewhere.",
                            exe_dir
                        );
                        eprintln!("              Run 'nfd2nfc watcher restart' to update plist.");
                    }
                }
            }
            Some(_) => println!("Watcher binary:   not found"),
            None => {
                if plist_exists {
                    println!("Watcher binary:   unknown (failed to parse plist)");
                } else {
                    println!("Watcher binary:   not registered");
                }
            }
        }

        // Config
        if config_exists {
            println!("Config:           {}", config_path);
        } else {
            println!("Config:           not created");
        }

        // Plist
        if plist_exists {
            println!("Plist:            {}", plist_path);
        } else {
            println!("Plist:            not created");
        }

        // Registered paths
        if total == 0 {
            println!("Registered paths: 0 total");
        } else {
            let mut parts = Vec::new();
            if active > 0 {
                parts.push(format!("{} active", active));
            }
            if redundant > 0 {
                parts.push(format!("{} redundant", redundant));
            }
            if not_found > 0 {
                parts.push(format!("{} not found", not_found));
            }
            if not_a_directory > 0 {
                parts.push(format!("{} not a directory", not_a_directory));
            }
            if permission_denied > 0 {
                parts.push(format!("{} permission denied", permission_denied));
            }
            println!("Registered paths: {} total ({})", total, parts.join(", "));
        }

        // FDA
        println!("Full Disk Access: {}", fda);

        // Update
        match &update_status {
            UpdateStatus::Available(ver) => println!("Update:           v{} available", ver),
            UpdateStatus::UpToDate(_) => println!("Update:           up to date"),
            UpdateStatus::Unknown => println!("Update:           unknown"),
        }
    }

    Ok(())
}

enum UpdateStatus {
    Available(String),
    UpToDate(String),
    Unknown,
}

fn check_update_status_with_timeout() -> UpdateStatus {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(version::check_latest_version());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Some(ver)) => UpdateStatus::Available(ver),
        Ok(None) => {
            let current = env!("CARGO_PKG_VERSION").to_string();
            UpdateStatus::UpToDate(current)
        }
        Err(_) => UpdateStatus::Unknown,
    }
}

/// Parse watcher binary path from plist XML.
/// Returns (Some(display_path), Some(raw_path_for_fda), exists) or (None, None, false).
/// display_path is canonicalized and abbreviated, raw_path is for spawning --check-fda.
fn get_watcher_binary_from_plist() -> (Option<String>, Option<String>, bool) {
    if !PLIST_PATH.exists() {
        return (None, None, false);
    }

    let content = match fs::read_to_string(&*PLIST_PATH) {
        Ok(c) => c,
        Err(_) => return (None, None, false),
    };

    // Parse ProgramArguments: find <key>ProgramArguments</key> then first <string>...</string>
    let key = "ProgramArguments";
    if let Some(key_pos) = content.find(key) {
        let after_key = &content[key_pos + key.len()..];
        if let Some(string_start) = after_key.find("<string>") {
            let value_start = string_start + "<string>".len();
            if let Some(string_end) = after_key[value_start..].find("</string>") {
                let raw_path = after_key[value_start..value_start + string_end]
                    .trim()
                    .to_string();
                let path_buf = Path::new(&raw_path);
                let exists = path_buf.exists();
                let display_path = fs::canonicalize(path_buf)
                    .map(|p| abbreviate_home_path(&p))
                    .unwrap_or_else(|_| raw_path.clone());
                return (Some(display_path), Some(raw_path), exists);
            }
        }
    }

    (None, None, false)
}

/// Check FDA by spawning the watcher binary with --check-fda.
const FDA_CHECK_LABEL: &str = "nfd2nfc-fda-check";

/// Check FDA via launchctl submit to avoid inheriting terminal's TCC permissions.
fn check_fda_via_watcher(watcher_path: &Option<String>) -> &'static str {
    let path = match watcher_path {
        Some(p) if Path::new(p).exists() => p,
        _ => return "unknown",
    };

    // Clean up any leftover label from previous run
    let _ = ProcessCommand::new("launchctl")
        .args(["remove", FDA_CHECK_LABEL])
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status();

    // Submit via launchd (doesn't inherit terminal TCC)
    let submit_status = ProcessCommand::new("launchctl")
        .args(["submit", "-l", FDA_CHECK_LABEL, "--", path, "--check-fda"])
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status();

    match submit_status {
        Ok(status) if status.success() => {}
        _ => return "unknown",
    }

    // Poll for completion (max 1 second, 50ms intervals)
    let result = poll_fda_result();

    // Clean up
    let _ = ProcessCommand::new("launchctl")
        .args(["remove", FDA_CHECK_LABEL])
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status();

    result
}

/// Poll launchctl list for the FDA check job's exit code.
/// Output format:
/// ```
/// {
///     "PID" = 12345;             ← present only while running
///     "LastExitStatus" = 0;      ← present after completion
///     ...
/// };
/// ```
fn poll_fda_result() -> &'static str {
    let max_attempts = 20; // 20 * 50ms = 1 second
    for _ in 0..max_attempts {
        std::thread::sleep(std::time::Duration::from_millis(50));

        let output = match ProcessCommand::new("launchctl")
            .args(["list", FDA_CHECK_LABEL])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            Ok(o) => o,
            Err(_) => continue,
        };

        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Still running if PID is present
        if stdout.contains("\"PID\"") {
            continue;
        }

        // Parse: "LastExitStatus" = 0;
        for line in stdout.lines() {
            if line.contains("\"LastExitStatus\"")
                && let Some(value) = line.split('=').nth(1)
            {
                // launchd stores exit status left-shifted by 8 bits
                // (e.g., 256 = exit code 1)
                let raw = value.trim().trim_end_matches(';').trim();
                let code = raw.parse::<i32>().map(|n| n >> 8).unwrap_or(-1);
                return match code {
                    0 => "granted",
                    1 => "not granted",
                    _ => "unknown",
                };
            }
        }
    }

    "unknown"
}

// === Watcher ===

fn cmd_watcher(action: WatcherAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        WatcherAction::Start => {
            if daemon_controller::check_watcher_status() {
                println!("Watcher is already running.");
                return Ok(());
            }
            daemon_controller::ensure_plist_up_to_date()
                .map_err(|e| format!("Failed to prepare plist: {}", e))?;
            daemon_controller::try_start_watcher()
                .map_err(|e| format!("Failed to start watcher: {}", e))?;
            println!("Watcher started.");
        }
        WatcherAction::Stop => {
            if !daemon_controller::check_watcher_status() {
                println!("Watcher is not running.");
                return Ok(());
            }
            daemon_controller::try_stop_watcher()
                .map_err(|e| format!("Failed to stop watcher: {}", e))?;
            println!("Watcher stopped.");
        }
        WatcherAction::Restart => {
            let was_running = daemon_controller::check_watcher_status();
            if was_running {
                daemon_controller::try_stop_watcher()
                    .map_err(|e| format!("Failed to stop watcher: {}", e))?;
            }
            daemon_controller::ensure_plist_up_to_date()
                .map_err(|e| format!("Failed to prepare plist: {}", e))?;
            daemon_controller::try_start_watcher()
                .map_err(|e| format!("Failed to start watcher: {}", e))?;
            if was_running {
                println!("Watcher restarted.");
            } else {
                println!("Watcher started.");
            }
        }
    }
    Ok(())
}

// === Config ===

fn cmd_config(action: ConfigAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ConfigAction::List { json } => cmd_config_list(json),
        ConfigAction::Add {
            path,
            action,
            mode,
            dry_run,
            json,
        } => cmd_config_add(&path, &action, &mode, dry_run, json),
        ConfigAction::Remove {
            index,
            dry_run,
            json,
        } => cmd_config_remove(index, dry_run, json),
        ConfigAction::Sort => cmd_config_sort(),
    }
}

fn path_status_to_str(status: &PathStatus) -> &'static str {
    match status {
        PathStatus::Active => "active",
        PathStatus::Redundant(_) => "redundant",
        PathStatus::NotFound => "not_found",
        PathStatus::NotADirectory => "not_a_directory",
        PathStatus::PermissionDenied => "permission_denied",
    }
}

fn path_status_note(status: &PathStatus, paths: &[PathEntry]) -> Option<String> {
    match status {
        PathStatus::Active => None,
        PathStatus::Redundant(idx) => {
            let covered_by = &paths[*idx];
            Some(format!(
                "covered by #{}: {}",
                idx + 1,
                abbreviate_path_str(&covered_by.raw)
            ))
        }
        PathStatus::NotFound => Some("path does not exist".to_string()),
        PathStatus::NotADirectory => Some("path is not a directory".to_string()),
        PathStatus::PermissionDenied => Some("permission denied".to_string()),
    }
}

fn abbreviate_path_str(path: &str) -> String {
    abbreviate_home_path(Path::new(path))
}

fn entry_to_json(index: usize, entry: &PathEntry, paths: &[PathEntry]) -> String {
    let note = path_status_note(&entry.status, paths);
    let note_json = match &note {
        Some(n) => format!(r#""{}""#, n),
        None => "null".to_string(),
    };
    format!(
        r#"{{"index":{},"path":"{}","action":"{}","mode":"{}","status":"{}","note":{}}}"#,
        index + 1,
        abbreviate_path_str(&entry.raw),
        entry.action.as_str().to_lowercase(),
        entry.mode.as_str().to_lowercase(),
        path_status_to_str(&entry.status),
        note_json,
    )
}

fn cmd_config_list(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (config, _) = config::load_config();

    if json {
        let entries: Vec<String> = config
            .paths
            .iter()
            .enumerate()
            .map(|(i, entry)| entry_to_json(i, entry, &config.paths))
            .collect();
        println!("[{}]", entries.join(",\n "));
    } else {
        if config.paths.is_empty() {
            println!("No paths configured.");
            return Ok(());
        }
        for (i, entry) in config.paths.iter().enumerate() {
            let note = path_status_note(&entry.status, &config.paths);
            let note_str = match &note {
                Some(n) => format!("  ({})", n),
                None => String::new(),
            };
            println!(
                "#{:<3} {}  {}  {}  {}{}",
                i + 1,
                abbreviate_path_str(&entry.raw),
                entry.action.as_str().to_lowercase(),
                entry.mode.as_str().to_lowercase(),
                path_status_to_str(&entry.status),
                note_str,
            );
        }
    }

    Ok(())
}

fn parse_action(s: &str) -> Result<PathAction, String> {
    match s.to_lowercase().as_str() {
        "watch" => Ok(PathAction::Watch),
        "ignore" => Ok(PathAction::Ignore),
        _ => Err(format!("Invalid action '{}'. Use 'watch' or 'ignore'.", s)),
    }
}

fn parse_mode(s: &str) -> Result<PathMode, String> {
    match s.to_lowercase().as_str() {
        "recursive" => Ok(PathMode::Recursive),
        "children" => Ok(PathMode::Children),
        _ => Err(format!(
            "Invalid mode '{}'. Use 'recursive' or 'children'.",
            s
        )),
    }
}

fn cmd_config_add(
    path: &str,
    action: &str,
    mode: &str,
    dry_run: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let action_val = parse_action(action)?;
    let mode_val = parse_mode(mode)?;

    if action_val == PathAction::Ignore && mode_val == PathMode::Children {
        return Err("ignore action only supports recursive mode.".into());
    }

    let (mut config, _) = config::load_config();

    // Save original statuses for affected detection
    let original_statuses: Vec<PathStatus> = config.paths.iter().map(|p| p.status).collect();

    // Add new entry
    let expanded = expand_tilde(path);
    let new_entry = PathEntry::new(expanded, action_val, mode_val);
    config.paths.push(new_entry);
    config.refresh_statuses();

    let new_idx = config.paths.len() - 1;
    let new_entry = &config.paths[new_idx];

    if dry_run {
        // Find affected entries (status changed)
        let mut affected = Vec::new();
        for (i, entry) in config.paths.iter().enumerate() {
            if i < original_statuses.len() && entry.status != original_statuses[i] {
                affected.push(i);
            }
        }

        if json {
            let added_json = entry_to_json(new_idx, new_entry, &config.paths);
            let affected_json: Vec<String> = affected
                .iter()
                .map(|&i| entry_to_json(i, &config.paths[i], &config.paths))
                .collect();
            println!(
                r#"{{"would_add":{},"affected":[{}]}}"#,
                added_json,
                affected_json.join(","),
            );
        } else {
            let note = path_status_note(&new_entry.status, &config.paths);
            println!(
                "[dry-run] Would add: {} ({}, {})",
                abbreviate_path_str(path),
                action_val.as_str().to_lowercase(),
                mode_val.as_str().to_lowercase(),
            );
            println!(
                "  → Status: {}{}",
                path_status_to_str(&new_entry.status),
                match &note {
                    Some(n) => format!(" ({})", n),
                    None => String::new(),
                },
            );
            if affected.is_empty() {
                println!("  No existing entries affected.");
            } else {
                println!("  Affected entries:");
                for &i in &affected {
                    let e = &config.paths[i];
                    let n = path_status_note(&e.status, &config.paths);
                    println!(
                        "    #{}: {} → {}{}",
                        i + 1,
                        abbreviate_path_str(&e.raw),
                        path_status_to_str(&e.status),
                        match &n {
                            Some(n) => format!(" ({})", n),
                            None => String::new(),
                        },
                    );
                }
            }
        }
    } else {
        config.save_to_file(&CONFIG_PATH)?;

        if json {
            println!(
                r#"{{"added":{}}}"#,
                entry_to_json(new_idx, new_entry, &config.paths)
            );
        } else {
            println!(
                "Added: {} ({}, {})",
                abbreviate_path_str(path),
                action_val.as_str().to_lowercase(),
                mode_val.as_str().to_lowercase(),
            );
        }
    }

    Ok(())
}

fn cmd_config_remove(
    index: usize,
    dry_run: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if index == 0 {
        return Err("Index must be 1 or greater.".into());
    }

    let (mut config, _) = config::load_config();
    let idx = index - 1;

    if idx >= config.paths.len() {
        return Err(format!(
            "Index {} out of range. {} paths configured.",
            index,
            config.paths.len()
        )
        .into());
    }

    if dry_run {
        // Save original statuses for affected detection
        let original_statuses: Vec<PathStatus> = config.paths.iter().map(|p| p.status).collect();

        let removed_entry = config.paths[idx].clone();
        let removed_json = entry_to_json(idx, &removed_entry, &config.paths);

        config.paths.remove(idx);
        config.refresh_statuses();

        // Find affected entries (status changed)
        let mut affected = Vec::new();
        for (i, entry) in config.paths.iter().enumerate() {
            let orig_idx = if i < idx { i } else { i + 1 };
            if orig_idx < original_statuses.len() && entry.status != original_statuses[orig_idx] {
                affected.push(i);
            }
        }

        if json {
            let affected_json: Vec<String> = affected
                .iter()
                .map(|&i| entry_to_json(i, &config.paths[i], &config.paths))
                .collect();
            println!(
                r#"{{"would_remove":{},"affected":[{}]}}"#,
                removed_json,
                affected_json.join(","),
            );
        } else {
            println!(
                "[dry-run] Would remove #{}: {} ({}, {})",
                index,
                abbreviate_path_str(&removed_entry.raw),
                removed_entry.action.as_str().to_lowercase(),
                removed_entry.mode.as_str().to_lowercase(),
            );
            if affected.is_empty() {
                println!("  No existing entries affected.");
            } else {
                println!("  Affected entries:");
                for &i in &affected {
                    let e = &config.paths[i];
                    let n = path_status_note(&e.status, &config.paths);
                    println!(
                        "    #{}: {} → {}{}",
                        i + 1,
                        abbreviate_path_str(&e.raw),
                        path_status_to_str(&e.status),
                        match &n {
                            Some(n) => format!(" ({})", n),
                            None => String::new(),
                        },
                    );
                }
            }
        }
    } else {
        let removed_info = entry_to_json(idx, &config.paths[idx], &config.paths);
        let removed = config.paths.remove(idx);
        config.refresh_statuses();
        config.save_to_file(&CONFIG_PATH)?;

        if json {
            println!(r#"{{"removed":{}}}"#, removed_info);
        } else {
            println!("Removed #{}: {}", index, abbreviate_path_str(&removed.raw));
        }
    }

    Ok(())
}

fn cmd_config_sort() -> Result<(), Box<dyn std::error::Error>> {
    let (mut config, _) = config::load_config();

    if config.paths.len() <= 1 {
        println!("Paths sorted.");
        return Ok(());
    }

    config.paths.sort_by(|a, b| a.raw.cmp(&b.raw));
    config.refresh_statuses();
    config.save_to_file(&CONFIG_PATH)?;

    println!("Paths sorted.");
    Ok(())
}

// === Convert ===

enum ConvertMode {
    Name,
    Children,
    Recursive,
}

fn parse_convert_mode(s: &str) -> Result<ConvertMode, String> {
    match s.to_lowercase().as_str() {
        "name" => Ok(ConvertMode::Name),
        "children" => Ok(ConvertMode::Children),
        "recursive" => Ok(ConvertMode::Recursive),
        _ => Err(format!(
            "Invalid mode '{}'. Use 'name', 'children', or 'recursive'.",
            s
        )),
    }
}

fn parse_target(s: &str) -> Result<NormalizationTarget, String> {
    match s.to_lowercase().as_str() {
        "nfc" => Ok(NormalizationTarget::NFC),
        "nfd" => Ok(NormalizationTarget::NFD),
        _ => Err(format!("Invalid target '{}'. Use 'nfc' or 'nfd'.", s)),
    }
}

fn cmd_convert(
    path: &str,
    mode: &str,
    target: &str,
    dry_run: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mode_val = parse_convert_mode(mode)?;
    let target_val = parse_target(target)?;
    let expanded = expand_tilde(path);

    if !expanded.exists() {
        return Err(format!("Path does not exist: {}", path).into());
    }

    if dry_run {
        cmd_convert_dry_run(&expanded, mode_val, target_val, json)
    } else {
        cmd_convert_execute(&expanded, mode_val, target_val, json)
    }
}

fn cmd_convert_dry_run(
    path: &Path,
    mode: ConvertMode,
    target: NormalizationTarget,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut would_convert = Vec::new();

    match mode {
        ConvertMode::Name => {
            if let Ok(actual_name) = get_actual_file_name(path)
                && target.needs_conversion(&actual_name)
            {
                would_convert.push(abbreviate_home_path(path));
            }
        }
        ConvertMode::Children | ConvertMode::Recursive => {
            let recursive = matches!(mode, ConvertMode::Recursive);
            let mut queue = VecDeque::new();
            queue.push_back(path.to_path_buf());

            while let Some(dir) = queue.pop_front() {
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();

                        if target.needs_conversion(&name) {
                            would_convert.push(abbreviate_home_path(&entry_path));
                        }

                        if recursive && entry_path.is_dir() && !entry_path.is_symlink() {
                            queue.push_back(entry_path);
                        }
                    }
                }
            }
        }
    }

    if json {
        let paths_json: Vec<String> = would_convert
            .iter()
            .map(|p| format!(r#""{}""#, p))
            .collect();
        println!(
            r#"{{"target":"{}","would_convert":[{}],"count":{}}}"#,
            target.to_string().to_lowercase(),
            paths_json.join(","),
            would_convert.len(),
        );
    } else if would_convert.is_empty() {
        println!("No files need conversion.");
    } else {
        println!(
            "Would convert to {} ({} files):",
            target,
            would_convert.len()
        );
        for p in &would_convert {
            println!("  {}", p);
        }
    }

    Ok(())
}

fn cmd_convert_execute(
    path: &Path,
    mode: ConvertMode,
    target: NormalizationTarget,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        ConvertMode::Name => {
            let result = normalizer::normalize_single_file(path, target)?;
            if json {
                let converted: Vec<String> = result
                    .iter()
                    .map(|r| format!(r#""{}""#, abbreviate_home_path(&r.to)))
                    .collect();
                println!(
                    r#"{{"target":"{}","converted":[{}],"errors":[],"count":{}}}"#,
                    target.to_string().to_lowercase(),
                    converted.join(","),
                    converted.len(),
                );
            } else if let Some(r) = &result {
                println!("Converted to {}:", target);
                println!("  {}", abbreviate_home_path(&r.to));
                println!("1 converted, 0 errors");
            } else {
                println!("No files need conversion.");
            }
        }
        ConvertMode::Children | ConvertMode::Recursive => {
            let recursive = matches!(mode, ConvertMode::Recursive);
            let result = normalizer::normalize_directory(path, recursive, target)?;

            if json {
                let converted: Vec<String> = result
                    .converted
                    .iter()
                    .map(|r| format!(r#""{}""#, abbreviate_home_path(&r.to)))
                    .collect();
                let errors: Vec<String> = result
                    .errors
                    .iter()
                    .map(|e| {
                        format!(
                            r#"{{"path":"{}","error":"{}"}}"#,
                            abbreviate_home_path(&e.path),
                            e.error
                        )
                    })
                    .collect();
                println!(
                    r#"{{"target":"{}","converted":[{}],"errors":[{}],"count":{}}}"#,
                    target.to_string().to_lowercase(),
                    converted.join(","),
                    errors.join(","),
                    result.converted.len(),
                );
            } else if result.converted.is_empty() && result.errors.is_empty() {
                println!("No files need conversion.");
            } else {
                if !result.converted.is_empty() {
                    println!("Converted to {}:", target);
                    for r in &result.converted {
                        println!("  {}", abbreviate_home_path(&r.to));
                    }
                }
                println!(
                    "{} converted, {} errors",
                    result.converted.len(),
                    result.errors.len()
                );
                for e in &result.errors {
                    eprintln!("  Error: {} — {}", abbreviate_home_path(&e.path), e.error);
                }
            }
        }
    }

    Ok(())
}

// === Log ===

fn cmd_log(action: Option<LogAction>) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        None | Some(LogAction::Show { .. }) => {
            let (last, json) = match action {
                Some(LogAction::Show { last, json }) => (last, json),
                _ => ("30m".to_string(), false),
            };
            cmd_log_show(&last, json)
        }
        Some(LogAction::Stream { json }) => cmd_log_stream(json),
    }
}

fn cmd_log_show(last: &str, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let entries =
        log_service::get_log_history(last).map_err(|e| format!("Failed to get logs: {}", e))?;

    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|e| {
                format!(
                    r#"{{"timestamp":"{}","message":{}}}"#,
                    e.display_time,
                    serde_json::to_string(&e.message).unwrap_or_default(),
                )
            })
            .collect();
        println!("[{}]", items.join(",\n "));
    } else if entries.is_empty() {
        println!("No logs found for the last {}.", last);
    } else {
        for entry in &entries {
            println!("{} {}", entry.display_time, entry.message);
        }
    }

    Ok(())
}

fn cmd_log_stream(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let predicate = format!("subsystem == \"{}\"", NFD2NFC_SERVICE_LABEL);
    let mut child = ProcessCommand::new("log")
        .args(["stream", "--predicate", &predicate, "--style", "ndjson"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start log stream: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or("Failed to capture log stream stdout")?;

    let reader = BufReader::new(stdout);

    for line in reader.lines().map_while(Result::ok) {
        if let Some(entry) = log_service::extract_log_entry(&line) {
            if json {
                println!(
                    r#"{{"timestamp":"{}","message":{}}}"#,
                    entry.display_time,
                    serde_json::to_string(&entry.message).unwrap_or_default(),
                );
            } else {
                println!("{} {}", entry.display_time, entry.message);
            }
        }
    }

    let _ = child.wait();
    Ok(())
}
