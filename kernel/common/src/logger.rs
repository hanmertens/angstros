//! Simple logger implementation

use crate::println;
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use owo_colors::{AnsiColors, OwoColorize};
use spin::Once;

static LOGGER: Once<Logger> = Once::new();

struct Logger {
    level: LevelFilter,
}

impl Logger {
    fn new(level: LevelFilter) -> Self {
        Self { level }
    }

    fn init(&'static self) -> Result<(), SetLoggerError> {
        log::set_logger(self)?;
        log::set_max_level(self.level);
        Ok(())
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();
            let level = level.color(match level {
                Level::Error => AnsiColors::Red,
                Level::Warn => AnsiColors::Yellow,
                Level::Info => AnsiColors::Green,
                Level::Debug => AnsiColors::Cyan,
                Level::Trace => AnsiColors::Magenta,
            });
            println!("{} {}", level, record.args());
        }
    }

    fn flush(&self) {}
}

// Should be called only once; subsequent calls will panic
pub fn init(level: LevelFilter) -> Result<(), SetLoggerError> {
    LOGGER.call_once(|| Logger::new(level)).init()
}
