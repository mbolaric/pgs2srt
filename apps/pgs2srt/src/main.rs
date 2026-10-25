mod args;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
    Config,
};

use args::Args;
use pgs_core::{PgsToSrt, ProgressReporter};

struct IndicatifReporter<'a>(&'a ProgressBar);

impl<'a> ProgressReporter for IndicatifReporter<'a> {
    fn set_total(&self, total: usize) {
        self.0.set_length(total as u64);
    }

    fn set_position(&self, position: usize) {
        self.0.set_position(position as u64);
    }

    fn println(&self, msg: &str) {
        self.0.println(msg);
    }
}

pub fn init_logging() {
    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Debug))
        .unwrap();
    let _ = log4rs::init_config(config).unwrap();
}

fn main() {
    init_logging();

    let args = Args::parse();

    #[cfg(debug_assertions)]
    let pb = ProgressBar::hidden();
    #[cfg(not(debug_assertions))]
    let pb = ProgressBar::new(0);

    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let reporter = IndicatifReporter(&pb);
    reporter.println("[+] Start Processing");

    let pgs_to_srt = PgsToSrt::new(&reporter);
    match pgs_to_srt.run(
        args.pgs_file_name.trim(),
        &args.language,
        &args.srt_file_name,
    ) {
        Ok(_) => {
            reporter.println("[+] Processing Done");
        }
        Err(err) => {
            reporter.println(&format!("[-] {:}", err));
            reporter.println("[+] Processing Done");
        }
    }
}
