mod cli;
mod daemon_controller;
mod log_service;
mod tui;
mod version;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about = "NFD/NFC filename converter for macOS.\nRun without arguments to launch TUI."
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show overall status (watcher, paths, FDA, update)
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Control the watcher service
    Watcher {
        #[command(subcommand)]
        action: WatcherAction,
    },
    /// Manage watch/ignore paths
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Convert filenames between NFD and NFC
    Convert {
        /// Target file or directory path
        path: String,
        /// Conversion scope: name, children, recursive
        #[arg(long, default_value = "name")]
        mode: String,
        /// Target form: nfc or nfd
        #[arg(long, default_value = "nfc")]
        target: String,
        /// Preview changes without converting
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// View watcher logs (defaults to 'show --last 30m' if no subcommand)
    Log {
        #[command(subcommand)]
        action: Option<LogAction>,
    },
}

#[derive(Subcommand)]
enum LogAction {
    /// Show past logs for a time period (default if no subcommand)
    Show {
        /// Time range (e.g., 5m, 1h, 30m)
        #[arg(long, default_value = "30m")]
        last: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Stream live logs in real-time (Ctrl+C to stop)
    Stream {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum WatcherAction {
    /// Start the watcher service
    Start,
    /// Stop the watcher service
    Stop,
    /// Restart the watcher service
    Restart,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// List configured paths
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a path to watch or ignore
    Add {
        /// Target directory path
        path: String,
        /// watch or ignore
        #[arg(long, default_value = "watch")]
        action: String,
        /// recursive or children (ignore only supports recursive)
        #[arg(long, default_value = "recursive")]
        mode: String,
        /// Preview the effect without saving
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a path by index (1-based)
    Remove {
        index: usize,
        /// Preview the effect without saving
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Sort paths alphabetically
    Sort,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        None => tui::run(),
        Some(cmd) => {
            cli::run(cmd)?;
            Ok(())
        }
    }
}
