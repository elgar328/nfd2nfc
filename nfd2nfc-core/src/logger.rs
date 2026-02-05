use crate::constants::NFD2NFC_SERVICE_LABEL;
use fern;
use log::LevelFilter;
use oslog::OsLogger;
use std::sync::Once;

#[derive(Debug, Clone, Copy)]
pub enum LogBackend {
    Terminal,
    OSLog,
}

static INIT: Once = Once::new();

/// Initializes the logger with the specified backend and verbosity level.
///
/// # Arguments
///
/// * `backend` - Selects which logging backend to use.
/// * `verbose` - Verbosity level:
///     - 0 => Error,
///     - 1 => Warn,
///     - 2 => Info,
///     - 3 => Debug,
///     - 4 or more => Trace.
///
/// This function must be called only once at program startup.
pub fn init_logger(backend: LogBackend, verbose: u8) {
    // Determine log level based on verbosity.
    let log_level = match verbose {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    INIT.call_once(|| {
        match backend {
            LogBackend::Terminal => {
                fern::Dispatch::new()
                    .format(|out, message, _record| {
                        // Only output the message without time or level.
                        out.finish(format_args!("{}", message))
                    })
                    .level(log_level)
                    .chain(std::io::stdout())
                    .apply()
                    .expect("Failed to initialize terminal logger");
            }
            LogBackend::OSLog => {
                // OSLog backend using the oslog crate.
                OsLogger::new(NFD2NFC_SERVICE_LABEL)
                    .level_filter(log_level)
                    .init()
                    .expect("Failed to initialize OSLog logger");
            }
        }
    });
}
