use crate::config::Config;
use crate::handler;
use log::{error, info};
use notify::{Error as NotifyError, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::spawn;
use unicode_normalization::is_nfc;

pub async fn start_watcher(rt_handle: tokio::runtime::Handle, config: Config) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, NotifyError>| {
            let tx = tx.clone();
            let rt_handle = rt_handle.clone();
            rt_handle.spawn(async move {
                if let Err(e) = tx.send(res).await {
                    error!("Failed to send file event: {:?}", e);
                }
            });
        },
        notify::Config::default(),
    ) {
        Ok(w) => {
            info!("File system event watcher initialized successfully.");
            w
        }
        Err(e) => {
            error!(
                "Failed to initialize file system event watcher: {:?}. Exiting normally (exit code 0).",
                e
            );
            std::process::exit(0);
        }
    };

    // Register recursive watch paths.
    for path in &config.recursive_watch_paths {
        match watcher.watch(&path, RecursiveMode::Recursive) {
            Ok(()) => info!("Monitoring recursive path: {:?}", path),
            Err(e) => error!("Failed to watch recursive path: {:?} - {:?}", path, e),
        }
    }

    // Register non-recursive watch paths.
    for path in &config.non_recursive_watch_paths {
        match watcher.watch(&path, RecursiveMode::NonRecursive) {
            Ok(()) => info!("Monitoring non-recursive path: {:?}", path),
            Err(e) => error!("Failed to watch non-recursive path: {:?} - {:?}", path, e),
        }
    }

    info!("File system monitoring started.");

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
                    .recursive_exclude_paths
                    .iter()
                    .any(|exclude| event_path.starts_with(exclude))
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
                spawn(async move {
                    handler::handle_event(event).await;
                });
            }
            Err(e) => error!("FS watcher error: {:?}", e),
        }
    }
}
