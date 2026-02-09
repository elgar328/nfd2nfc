use crate::handler;
use log::{debug, error, info};
use nfd2nfc_core::config::{ActiveEntry, PathAction, PathMode};
use nfd2nfc_core::constants::{HEARTBEAT_INTERVAL, HEARTBEAT_PATH};
use notify::{Error as NotifyError, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::Semaphore;
use unicode_normalization::is_nfc;

/// Maximum number of concurrent file event handler tasks.
const MAX_CONCURRENT_TASKS: usize = 200;

/// Debounce window for deduplicating events on the same path.
const DEBOUNCE_DURATION: Duration = Duration::from_millis(50);

/// Determine the effective action for an event path based on active entries.
/// Returns the action of the most specific (longest canonical path) matching active entry.
fn effective_action(event_path: &Path, active_entries: &[ActiveEntry]) -> Option<PathAction> {
    active_entries
        .iter()
        .filter(|e| match e.mode {
            PathMode::Recursive => event_path.starts_with(&e.canonical),
            PathMode::Children => event_path.parent() == Some(&*e.canonical),
        })
        .max_by_key(|e| e.canonical.as_os_str().len())
        .map(|e| e.action)
}

/// Register watch paths with the notify watcher. Returns (recursive_count, children_count, ignore_count).
fn register_watch_paths(
    watcher: &mut RecommendedWatcher,
    entries: &[ActiveEntry],
) -> (usize, usize, usize) {
    let mut recursive_count = 0usize;
    let mut children_count = 0usize;
    let mut ignore_count = 0usize;

    for entry in entries {
        match entry.action {
            PathAction::Watch => {
                let notify_mode = match entry.mode {
                    PathMode::Recursive => RecursiveMode::Recursive,
                    PathMode::Children => RecursiveMode::NonRecursive,
                };
                match watcher.watch(&entry.canonical, notify_mode) {
                    Ok(()) => {
                        debug!("Watching path: {}", entry.canonical.display());
                        match entry.mode {
                            PathMode::Recursive => recursive_count += 1,
                            PathMode::Children => children_count += 1,
                        }
                    }
                    Err(e) => error!(
                        "Failed to watch path: {} - {}",
                        entry.canonical.display(),
                        e
                    ),
                }
            }
            PathAction::Ignore => {
                ignore_count += 1;
            }
        }
    }

    (recursive_count, children_count, ignore_count)
}

/// Log a summary of the registered watch paths.
fn log_watch_summary(recursive_count: usize, children_count: usize, ignore_count: usize) {
    if recursive_count == 0 && children_count == 0 && ignore_count == 0 {
        info!(
            "nfd2nfc-watcher v{} started. No paths configured.",
            env!("CARGO_PKG_VERSION")
        );
    } else {
        info!(
            "nfd2nfc-watcher v{} started. Paths: {} recursive, {} children, {} ignored.",
            env!("CARGO_PKG_VERSION"),
            recursive_count,
            children_count,
            ignore_count
        );
    }
}

/// Spawn a background task that writes to the heartbeat file at a regular interval.
fn spawn_heartbeat_task() {
    let heartbeat_dir = HEARTBEAT_PATH.parent().map(Path::to_path_buf);
    spawn(async move {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(e) = std::fs::write(&*HEARTBEAT_PATH, "") {
                if e.kind() == std::io::ErrorKind::NotFound {
                    if let Some(dir) = &heartbeat_dir {
                        let _ = std::fs::create_dir_all(dir);
                    }
                }
            }
        }
    });
}

pub async fn start_watcher(rt_handle: tokio::runtime::Handle, entries: Vec<ActiveEntry>) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, NotifyError>| {
            let tx = tx.clone();
            let rt_handle = rt_handle.clone();
            rt_handle.spawn(async move {
                if let Err(e) = tx.send(res).await {
                    error!("Failed to send file event: {}", e);
                }
            });
        },
        notify::Config::default(),
    ) {
        Ok(w) => {
            debug!("File system event watcher initialized.");
            w
        }
        Err(e) => {
            error!(
                "Failed to initialize file system event watcher: {}. Exiting normally (exit code 0).",
                e
            );
            std::process::exit(0);
        }
    };

    let (recursive_count, children_count, ignore_count) =
        register_watch_paths(&mut watcher, &entries);
    log_watch_summary(recursive_count, children_count, ignore_count);
    spawn_heartbeat_task();

    // Limit the number of concurrently executing tasks using a semaphore.
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));

    // Process events with debouncing to deduplicate rapid events on the same path.
    let mut pending: HashMap<PathBuf, Event> = HashMap::new();
    let mut debounce_timer = tokio::time::interval(DEBOUNCE_DURATION);

    loop {
        tokio::select! {
            Some(res) = rx.recv() => {
                match res {
                    Ok(event) => {
                        let event_path = match event.paths.first() {
                            Some(path) => path,
                            None => continue,
                        };

                        // Use effective_action to determine if this event should be processed.
                        match effective_action(event_path, &entries) {
                            Some(PathAction::Watch) => {}
                            _ => continue,
                        }

                        let file_name = match event_path.file_name().and_then(|s| s.to_str()) {
                            Some(name) => name,
                            None => continue,
                        };
                        if is_nfc(file_name) {
                            continue;
                        }

                        // Same path overwrites previous event (deduplication).
                        pending.insert(event_path.clone(), event);
                    }
                    Err(e) => error!("FS watcher error: {}", e),
                }
            }
            _ = debounce_timer.tick() => {
                for (_path, event) in pending.drain() {
                    let sem_clone = semaphore.clone();
                    spawn(async move {
                        let _permit = sem_clone.acquire_owned().await.unwrap();
                        handler::handle_event(event).await;
                    });
                }
            }
        }
    }
}
