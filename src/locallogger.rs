use std::{
    io::{self, Write},
    str::FromStr,
};

use log::{Metadata, Record, warn};

/// A very simple local logger. Basically, just logging to stdout/stderr without much
/// configuration or anything. Its very basic, but it does the job.
pub struct LocalLogger {}

impl LocalLogger {
    /// Create a new logger,and attach it to the `log` crate. Additionally, it
    /// takes the desired log-level from env, checking the `LOG_LEVEL` env-var,
    /// and setting that as loglevel. If no value for `LOG_LEVEL` is found, it
    /// will simply fall back to `Info`.
    pub fn setup() -> Result<(), Box<dyn std::error::Error>> {
        let log_level = match std::env::var("LOG_LEVEL") {
            Ok(value) => match log::LevelFilter::from_str(&value) {
                Ok(level) => level,
                Err(_) => {
                    warn!("Unrecognized value for env var LOG_LEVEL: {value}");
                    log::Level::Info.to_level_filter()
                }
            },
            Err(_) => log::Level::Info.to_level_filter(),
        };
        let logger = Self {};

        log::set_boxed_logger(Box::new(logger))
            .map(move |()| log::set_max_level(log_level))
            .map_err(|e| format!("failed to install local logger: {e}"))?;

        Ok(())
    }
}

impl log::Log for LocalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if record.level() == log::Level::Error {
            eprintln!("[{}] {}", record.level(), record.args());
        } else {
            println!("[{}] {}", record.level(), record.args())
        }
    }

    fn flush(&self) {
        let _ = io::stdout().lock().flush();
        let _ = io::stderr().lock().flush();
    }
}
