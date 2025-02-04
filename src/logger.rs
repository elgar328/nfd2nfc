use fern::Dispatch;
use log::LevelFilter;
use std::io;

pub fn setup_logger(verbose: u8) {
    let terminal_level = match verbose {
        0 => LevelFilter::Error, // default
        1 => LevelFilter::Warn,  // -v
        2 => LevelFilter::Info,  // -vv
        3 => LevelFilter::Debug, // -vvv
        _ => LevelFilter::Trace, // -vvvv
    };

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(terminal_level)
        .chain(io::stdout())
        .apply()
        .unwrap();
}
