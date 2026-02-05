mod daemon_controller;
mod log_service;
mod tui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("nfd2nfc {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    tui::run()
}
