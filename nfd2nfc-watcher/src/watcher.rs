use crate::config::Config;
use crate::handler;
use log::{error, info};
use nfd2nfc_common::constants::WATCHER_LIVE_MESSAGE;
use notify::{Error as NotifyError, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::Semaphore;
use unicode_normalization::is_nfc;

pub async fn start_watcher(rt_handle: tokio::runtime::Handle, config: Config) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

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
            info!(" + File system event watcher initialized.");
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

    // Register recursive watch paths.
    for path in &config.recursive_watch_paths {
        match watcher.watch(&path, RecursiveMode::Recursive) {
            Ok(()) => info!(" + Watching recursive path: {}", path.display()),
            Err(e) => error!("Failed to watch recursive path: {} - {}", path.display(), e),
        }
    }

    // Register non-recursive watch paths.
    for path in &config.non_recursive_watch_paths {
        match watcher.watch(&path, RecursiveMode::NonRecursive) {
            Ok(()) => info!(" + Watching non-recursive path: {}", path.display()),
            Err(e) => error!(
                "Failed to watch non-recursive path: {} - {}",
                path.display(),
                e
            ),
        }
    }

    info!("{}", WATCHER_LIVE_MESSAGE);

    // Limit the number of concurrently executing tasks using a semaphore.
    let semaphore = Arc::new(Semaphore::new(200));

    // Process events in an asynchronous loop.
    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                let event_path = match event.paths.get(0) {
                    Some(path) => path,
                    None => continue,
                };

                // Skip events for paths in the exclusion list.
                if config
                    .recursive_ignore_paths
                    .iter()
                    .any(|ignore| event_path.starts_with(ignore))
                {
                    continue;
                }

                let file_name = match event_path.file_name().and_then(|s| s.to_str()) {
                    Some(name) => name,
                    None => continue,
                };
                if is_nfc(file_name) {
                    continue;
                }
                let sem_clone = semaphore.clone();
                spawn(async move {
                    let _permit = sem_clone.acquire_owned().await.unwrap();
                    handler::handle_event(event).await;
                });
            }
            Err(e) => error!("FS watcher error: {}", e),
        }
    }
}
