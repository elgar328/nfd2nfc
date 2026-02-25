use nfd2nfc_core::constants::NFD2NFC_SERVICE_LABEL;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use unicode_normalization::UnicodeNormalization;

/// Log entry with timestamp and message
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub display_time: String,   // UI display: "11:23:45"
    pub full_timestamp: String, // Query use: "2026-01-21 11:23:45.123456+0900"
    pub message: String,
    pub level: String, // macOS unified log messageType: "Default", "Info", "Debug", "Error", "Fault"
}

/// Events sent from background threads to the main UI thread
pub enum LogEvent {
    Live(LogEntry),
    HistoryChunk { entries: Vec<LogEntry> },
}

/// Extract a JSON string field value from ndjson line
/// Handles both: "field":"value" (ndjson) and "field" : "value" (pretty json)
fn extract_json_field(line: &str, field: &str) -> Option<String> {
    // Try ndjson format first (no spaces): "field":"value"
    let ndjson_prefix = format!("\"{}\":\"", field);
    // Then try pretty json format (with spaces): "field" : "value"
    let pretty_prefix = format!("\"{}\" : \"", field);

    let start = if let Some(idx) = line.find(&ndjson_prefix) {
        idx + ndjson_prefix.len()
    } else if let Some(idx) = line.find(&pretty_prefix) {
        idx + pretty_prefix.len()
    } else {
        return None;
    };

    // Find the closing quote, handling escaped characters
    let rest = &line[start..];
    let mut end_idx = 0;
    let mut chars = rest.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Skip the next character (escaped)
            if let Some(escaped) = chars.next() {
                end_idx += 1 + escaped.len_utf8();
            }
        } else if c == '"' {
            break;
        } else {
            end_idx += c.len_utf8();
        }
    }

    let escaped = &rest[..end_idx];
    // Use serde_json to properly unescape the string
    let json_str = format!("\"{}\"", escaped);
    serde_json::from_str(&json_str).ok()
}

/// Extract LogEntry from a JSON log line
pub fn extract_log_entry(line: &str) -> Option<LogEntry> {
    // Skip metadata lines like {"count":4,"finished":1}
    if line.contains("\"finished\"") || line.contains("\"count\"") {
        return None;
    }

    let full_timestamp = extract_json_field(line, "timestamp")?;

    // "2026-01-21 11:23:45.123456+0900" → "01-21 11:23:45"
    let display_time = {
        let parts: Vec<&str> = full_timestamp.split_whitespace().collect();
        if parts.len() >= 2 {
            let date_part = parts[0]; // "2026-01-21"
            let time_part = parts[1].split('.').next().unwrap_or(""); // "11:23:45"
            // Extract MM-DD and HH:MM:SS
            let date_short = date_part.get(5..).unwrap_or(date_part); // "01-21"
            format!("{} {}", date_short, time_part)
        } else {
            full_timestamp.clone()
        }
    };

    let message: String = extract_json_field(line, "eventMessage")?.nfkc().collect();
    let level = extract_json_field(line, "messageType").unwrap_or_default();

    Some(LogEntry {
        display_time,
        full_timestamp,
        message,
        level,
    })
}

/// Get log history for a duration (e.g., "5m", "30m", "1h")
pub fn get_log_history(duration: &str) -> Result<Vec<LogEntry>, String> {
    let predicate = format!("subsystem == \"{}\"", NFD2NFC_SERVICE_LABEL);
    let output = Command::new("log")
        .args([
            "show",
            "--predicate",
            &predicate,
            "--last",
            duration,
            "--style",
            "ndjson",
        ])
        .output()
        .map_err(|e| format!("Failed to execute log show command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter_map(extract_log_entry).collect())
}

/// Stream logs in real-time, sending LogEvent::Live for each entry
pub fn stream_logs(tx: Sender<LogEvent>) {
    let predicate = format!("subsystem == \"{}\"", NFD2NFC_SERVICE_LABEL);
    let mut child = match Command::new("log")
        .args(["stream", "--predicate", &predicate, "--style", "ndjson"])
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return,
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => return,
    };

    let reader = BufReader::new(stdout);

    for line in reader.lines().map_while(Result::ok) {
        if let Some(entry) = extract_log_entry(&line)
            && tx.send(LogEvent::Live(entry)).is_err()
        {
            break;
        }
    }
    let _ = child.wait();
}
