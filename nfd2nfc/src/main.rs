mod daemon_controller;
mod normalizer;

use crate::daemon_controller::WatchMode;
use crate::normalizer::*;
use clap::{CommandFactory, Parser, Subcommand};
use log::{error, info};
use nfd2nfc_common::logger::{init_logger, LogBackend};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "nfd2nfc",
    version = std::env!("CARGO_PKG_VERSION"),
    author = "elgar328",
    styles=get_styles(),
    color = clap::ColorChoice::Always,
    about = "\x1b[1;33;4mConversion Mode:\x1b[0m
  \x1b[32mnfd2nfc\x1b[0m converts filenames between NFD and NFC to ensure \
consistent cross-platform compatibility. For background monitoring, use the \
watch subcommand. See `\x1b[32mnfd2nfc watch --help\x1b[0m` for details.",
    override_usage = "\x1b[32mnfd2nfc\x1b[0m [OPTIONS] <PATH>",
    after_help = "\x1b[1;33;4mAdditional Info:\x1b[0m
  For more details, visit: \x1b[4mhttps://github.com/elgar328/nfd2nfc\x1b[0m

\x1b[1;33;4mExamples:\x1b[0m
  Convert a filename:
      \x1b[32mnfd2nfc\x1b[0m file.txt

  Convert folder contents (default):
      \x1b[32mnfd2nfc\x1b[0m folder

  Convert current folder contents:
      \x1b[32mnfd2nfc\x1b[0m .

  Convert folder name only:
      \x1b[32mnfd2nfc -d\x1b[0m folder

  Convert folder contents recursively:
      \x1b[32mnfd2nfc -r\x1b[0m folder

  Combined conversion (folder name & all contents):
      \x1b[32mnfd2nfc -dr\x1b[0m folder

  Reverse conversion (NFC → NFD):
      \x1b[32mnfd2nfc -R\x1b[0m file.txt

  Verbose mode examples:
      \x1b[32mnfd2nfc -v\x1b[0m file.txt          (Warnings only)
      \x1b[32mnfd2nfc -vv\x1b[0m folder           (Detailed info)
",
    help_template = "\
{about}\n\n{usage-heading} {usage}\n\n\x1b[1;33;4mArguments:\x1b[0m
  <PATH>  Path (file or folder)\n\n\x1b[1;33;4mOptions:\x1b[0m
{options}{after-help}"
)]

struct Cli {
    /// Subcommand for controlling the watcher daemon.
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path (file or folder) for conversion (used when no subcommand is given).
    #[arg(value_name = "PATH")]
    path: Option<String>,

    /// Rename the folder itself (can be combined with -c or -r).
    #[arg(short = 'd', long = "directory")]
    directory: bool,

    /// Rename files and folders inside the directory (non-recursive) [default].
    #[arg(short = 'c', long = "contents", conflicts_with = "recursive")]
    contents: bool,

    /// Rename files and folders recursively inside the directory.
    #[arg(short = 'r', long = "recursive")]
    recursive: bool,

    /// Convert from NFC to NFD instead of the default conversion (NFD to NFC).
    #[arg(short = 'R', long = "reverse")]
    reverse: bool,

    /// Increase verbosity (-v warnings, -vv info, -vvv debug, -vvvv trace).
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        hide = true,
        about = "\x1b[1;33;4mWatch Mode:\x1b[0m
  \x1b[32mnfd2nfc-watcher\x1b[0m operates as a background service that \
continuously monitors specified paths and converts filesystem entry names \
from NFD to NFC. Manage this service with the `\x1b[32mnfd2nfc watch\x1b[0m` subcommand. \
For further details, run `\x1b[32mnfd2nfc watch <COMMAND> --help\x1b[0m`.",
        after_help = "\x1b[1;33;4mAdditional Info:\x1b[0m
  For more details, visit: \x1b[4mhttps://github.com/elgar328/nfd2nfc\x1b[0m

\x1b[1;33;4mExamples:\x1b[0m
  Start the watcher service:
      \x1b[32mnfd2nfc watch start\x1b[0m
  
  Stop the watcher service:
      \x1b[32mnfd2nfc watch stop\x1b[0m
  
  Add the \"Desktop\" folder recursively to the watch list:
      \x1b[32mnfd2nfc watch add\x1b[0m Desktop -r
  
  Add the current directory (non-recursively) to the watch list:
      \x1b[32mnfd2nfc watch add\x1b[0m .
  
  Add the folder \"~/Desktop/folder\" to the ignore list:
      \x1b[32mnfd2nfc watch add\x1b[0m ~/Desktop/folder -i
  
  Remove the \"Desktop\" folder from the watch list:
      \x1b[32mnfd2nfc watch remove\x1b[0m Desktop
  
  List all configured watch paths:
      \x1b[32mnfd2nfc watch list\x1b[0m
  
  Stream the watcher logs in real time:
      \x1b[32mnfd2nfc watch log\x1b[0m
  
  Display the watcher logs from the past 5 minutes:
      \x1b[32mnfd2nfc watch log\x1b[0m --last 5m
"
    )]
    Watch(WatchCommand),
}

#[derive(Parser, Debug)]
struct WatchCommand {
    #[command(subcommand)]
    action: WatchAction,
}

#[derive(Subcommand, Debug)]
enum WatchAction {
    /// Start the nfd2nfc-watcher service.
    ///
    /// This command launches the background watcher that continuously monitors specified paths and converts filenames from NFD to NFC.
    Start,
    /// Stop the nfd2nfc-watcher service.
    Stop,
    /// Restart the nfd2nfc-watcher service.
    Restart,
    /// Display the current status of the nfd2nfc-watcher service.
    Status,
    #[command(
        about = "Add a path to the watch list",
        long_about = "Add a path to the watch list.

Use one of the mutually exclusive options to control watch behavior:
  --recursive: add the path and its subdirectories.
  --nonrecursive: add only the specified path.
  --ignore: mark the path to be ignored (recursively applied)."
    )]
    Add {
        /// The path to add.
        #[arg(value_name = "PATH", help = "The path to be added to the watch list.")]
        path: String,
        /// Add the path recursively.
        #[arg(short = 'r', long, conflicts_with_all = &["nonrecursive", "ignore"])]
        recursive: bool,
        /// Add only the specified path (non-recursive).
        #[arg(short = 'n', long, conflicts_with_all = &["recursive", "ignore"])]
        nonrecursive: bool,
        /// Mark the path to be ignored (recursively).
        #[arg(short = 'i', long, conflicts_with_all = &["recursive", "nonrecursive"])]
        ignore: bool,
    },
    /// Remove a watch path from the configuration.
    ///
    /// Specify a PATH to remove a specific watch path, or use the --all option to delete every watch path.
    Remove {
        /// The path to be removed.
        #[arg(value_name = "PATH", required_unless_present = "all")]
        path: Option<String>,
        /// Remove all watch paths.
        #[arg(long)]
        all: bool,
    },
    /// List all watched paths.
    List,
    /// Display watcher logs.
    ///
    /// By default, streams live logs in real time.
    /// Use `--last <DURATION>` to show logs from a past period (e.g., --last 2h, --last 5m, --last 30s).
    Log {
        /// Specify duration (e.g., 2h, 5m, 30s) for history logs.
        #[arg(long, value_name = "DURATION")]
        last: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    init_logger(LogBackend::Terminal, cli.verbose);

    // If a subcommand is provided, handle it via the daemon_controller module.
    if let Some(Commands::Watch(watch_cmd)) = cli.command {
        match watch_cmd.action {
            WatchAction::Start => {
                daemon_controller::cmd_start_watcher();
            }
            WatchAction::Stop => {
                daemon_controller::cmd_stop_watcher();
            }
            WatchAction::Restart => {
                daemon_controller::cmd_restart_watcher();
            }
            WatchAction::Status => {
                daemon_controller::cmd_status_watcher();
            }
            WatchAction::Log { last } => {
                if let Some(duration) = last {
                    daemon_controller::cmd_log_history(&duration);
                } else {
                    daemon_controller::cmd_stream_logs();
                }
            }
            WatchAction::Add {
                path,
                recursive,
                nonrecursive,
                ignore,
            } => {
                if ignore {
                    daemon_controller::cmd_add_watch_path(&path, WatchMode::Ignore);
                } else if recursive {
                    daemon_controller::cmd_add_watch_path(&path, WatchMode::Recursive);
                } else if nonrecursive {
                    daemon_controller::cmd_add_watch_path(&path, WatchMode::NonRecursive);
                } else {
                    daemon_controller::cmd_add_watch_path(&path, WatchMode::NonRecursive);
                }
            }
            WatchAction::Remove { path, all } => {
                if all {
                    daemon_controller::cmd_remove_watch_path_all();
                } else {
                    let p = path.expect("A path must be provided when --all is not used.");
                    daemon_controller::cmd_remove_watch_path(&p);
                }
            }
            WatchAction::List => {
                daemon_controller::cmd_list_watch_paths();
            }
        }
        return;
    }

    // No subcommand provided: perform the default conversion functionality.
    // Ensure that a PATH is provided for conversion.
    let path_str = match cli.path {
        Some(ref p) => p,
        None => {
            eprintln!("No PATH specified for conversion.\n");
            Cli::command().print_help().unwrap();
            std::process::exit(1);
        }
    };

    let path = Path::new(path_str);

    let reverse_mode = cli.reverse;

    if path.is_file() {
        if reverse_mode {
            heuristic_normalize_name_to_nfd(path);
        } else {
            heuristic_normalize_name_to_nfc(path);
        }
    } else if path.is_dir() {
        let process_directory_name = cli.directory;
        let process_contents = cli.contents || (!cli.directory && !cli.recursive);
        let process_recursive = cli.recursive;

        if process_recursive {
            if reverse_mode {
                normalize_names_to_nfd(path, true);
            } else {
                normalize_names_to_nfc(path, true);
            }
        } else if process_contents {
            if reverse_mode {
                normalize_names_to_nfd(path, false);
            } else {
                normalize_names_to_nfc(path, false);
            }
        }

        if process_directory_name {
            if reverse_mode {
                heuristic_normalize_name_to_nfd(path);
            } else {
                heuristic_normalize_name_to_nfc(path);
            }
        }
    } else {
        error!("Error: Not a valid file or directory: {}", path.display());
        std::process::exit(1);
    }

    info!("nfd2nfc process completed.");
}

pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}
