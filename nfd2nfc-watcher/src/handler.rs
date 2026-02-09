use log::{error, info};
use nfd2nfc_core::normalizer::{get_actual_file_name, NormalizationTarget};
use nfd2nfc_core::utils::abbreviate_home_path;
use notify::Event;

pub async fn handle_event(event: Event) {
    let path = match event.paths.first() {
        Some(p) => p,
        None => return,
    };

    let path_clone = path.to_path_buf();
    let actual_name =
        match tokio::task::spawn_blocking(move || get_actual_file_name(&path_clone)).await {
            Ok(Ok(name)) => name,
            Ok(Err(ref e)) => {
                if !e.is_not_found() {
                    error!(
                        "Failed to get file name: {} — {}",
                        abbreviate_home_path(path),
                        e
                    );
                }
                return;
            }
            Err(e) => {
                error!("Task join error: {}", e);
                return;
            }
        };

    let target = NormalizationTarget::NFC;
    if !target.needs_conversion(&actual_name) {
        return;
    }

    let nfc_file_name = target.convert(&actual_name);
    let new_path = path.with_file_name(&nfc_file_name);

    match tokio::fs::rename(path, &new_path).await {
        Ok(()) => info!("NFD→NFC: {}", abbreviate_home_path(&new_path)),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                error!(
                    "Failed to convert {} to NFC: {}",
                    abbreviate_home_path(&new_path),
                    e
                );
            }
        }
    }
}
