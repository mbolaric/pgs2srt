//! Process-wide logging shared by the CLI and desktop apps: a size-rolled log file in the
//! platform log folder, an optional console mirror, and a panic hook that records panics.

use std::path::{Path, PathBuf};

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;

use crate::dirs;

const FILE_PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S%.3f)} {l:<5} [{T}] {t} - {m}{n}";
const CONSOLE_PATTERN: &str = "{l:<5} {t} - {m}{n}";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const ROLLED_LOG_FILES: u32 = 3;

const OWN_CRATES: &[&str] = &["pgs_core", "ui_egui", "pgs2srt", "pgs2srt_desktop"];

pub fn init(app: &str, console: bool) -> Option<PathBuf> {
    let file = dirs::log_dir().and_then(|dir| match file_appender(&dir, app) {
        Ok(appender) => Some((dir.join(format!("{app}.log")), appender)),
        Err(error) => {
            eprintln!("pgs2srt: cannot open log file in {}: {error}", dir.display());
            None
        }
    });

    let mut builder = Config::builder();
    let mut root = Root::builder();
    let mut log_path = None;
    if let Some((path, appender)) = file {
        builder = builder.appender(Appender::builder().build("file", Box::new(appender)));
        root = root.appender("file");
        log_path = Some(path);
    }
    if console || log_path.is_none() {
        let stderr =
            ConsoleAppender::builder().target(Target::Stderr).encoder(Box::new(PatternEncoder::new(CONSOLE_PATTERN))).build();
        builder = builder.appender(Appender::builder().build("console", Box::new(stderr)));
        root = root.appender("console");
    }
    for name in OWN_CRATES {
        builder = builder.logger(Logger::builder().build(*name, LevelFilter::Debug));
    }

    let config = builder.build(root.build(LevelFilter::Warn)).ok()?;
    log4rs::init_config(config).ok()?;
    install_panic_hook();

    if let Some(path) = &log_path {
        log::info!("{app} {} logging to {}", env!("CARGO_PKG_VERSION"), path.display());
    }
    log_path
}

fn file_appender(dir: &Path, app: &str) -> Result<RollingFileAppender, String> {
    let roller = FixedWindowRoller::builder()
        .build(&dir.join(format!("{app}.{{}}.log")).to_string_lossy(), ROLLED_LOG_FILES)
        .map_err(|error| error.to_string())?;
    let policy = CompoundPolicy::new(Box::new(SizeTrigger::new(MAX_LOG_BYTES)), Box::new(roller));
    RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(FILE_PATTERN)))
        .build(dir.join(format!("{app}.log")), Box::new(policy))
        .map_err(|error| error.to_string())
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic: {info}\n{}", std::backtrace::Backtrace::force_capture());
        previous(info);
    }));
}
