mod args;
mod srt;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use log4rs::{append::console::ConsoleAppender, config::{Appender, Root}, Config};

use args::Args;
use srt::PgsToSrt;

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

    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    pb.println("[+] Start Processing");
    let pgs_to_srt = PgsToSrt::new(&pb);
    match pgs_to_srt.run(args.pgs_file_name.trim(), &args.language, &args.srt_file_name) {
        Ok(_) => {
            pb.println("[+] Processing Done");  
        },
        Err(err) => {
            pb.println(format!("[-] {:}", err));
            pb.println("[+] Processing Done");  
        }
    }
}