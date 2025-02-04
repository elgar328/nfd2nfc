mod logger;
mod normalizer;

use crate::logger::*;
use crate::normalizer::*;
use clap::Parser;
use log::{error, info};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "nfd2nfc",
    author = "elgar328",
    version = "1.0.0",
    styles=get_styles(),
    color = clap::ColorChoice::Always,
    about = "\x1b[1;33;4mOverview:\x1b[0m
  nfd2nfc converts filenames between NFD and NFC formats to ensure compatibility across macOS, Windows, and Linux.
  macOS stores filenames in NFD (decomposed form) while other systems typically use NFC (composed form).
  This discrepancy can lead to filename issues when transferring files between operating systems.",
    after_help = "\x1b[1;33;4mAdditional Info:\x1b[0m
  For more details, visit: https://github.com/elgar328/nfd2nfc


\x1b[1;33;4mExamples:\x1b[0m
  Convert a file:
      \x1b[32mnfd2nfc\x1b[0m file.txt

  Convert folder contents (default):
      \x1b[32mnfd2nfc\x1b[0m folder/

  Convert current folder contents:
      \x1b[32mnfd2nfc\x1b[0m .

  Convert folder name only:
      \x1b[32mnfd2nfc -d\x1b[0m folder/

  Convert folder contents recursively:
      \x1b[32mnfd2nfc -r\x1b[0m folder/

  Combined conversion (folder name & all contents):
      \x1b[32mnfd2nfc -dr\x1b[0m folder/

  Reverse conversion (NFC → NFD):
      \x1b[32mnfd2nfc -R\x1b[0m file.txt

  Verbose mode examples:
      \x1b[32mnfd2nfc -v\x1b[0m file.txt         # Warnings only
      \x1b[32mnfd2nfc -vv\x1b[0m folder/         # Detailed info
"
)]

struct Cli {
    /// Path (file or folder)
    #[arg(value_name = "PATH")]
    path: String,

    /// Rename the folder itself (can be combined with -c or -r)
    #[arg(short = 'd', long = "directory")]
    directory: bool,

    /// Rename files and folders inside the directory (non-recursive) [default]
    #[arg(short = 'c', long = "contents", conflicts_with = "recursive")]
    contents: bool,

    /// Rename files and folders recursively inside the directory
    #[arg(short = 'r', long = "recursive")]
    recursive: bool,

    /// Convert from NFC to NFD instead of the default conversion (NFD to NFC)
    #[arg(short = 'R', long = "reverse")]
    reverse: bool,

    /// Increase verbosity (use -v for warnings, -vv for info, -vvv for debug, -vvvv for trace)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() {
    // CLI 인자 파싱
    let cli = Cli::parse();
    let path = Path::new(&cli.path);

    setup_logger(cli.verbose);

    info!("Starting nfd2nfc...");

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
        error!("Error: Not a valid file or directory: {:?}", path);
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
