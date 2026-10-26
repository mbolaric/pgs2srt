mod args;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use args::Args;
use pgs_core::files::{validate_language, validate_output};
use pgs_core::language::{detect_language_from_path, probe_language_from_file};
use pgs_core::ocr::tesseract_backend;
use pgs_core::{output_path, validate_output_name, Error, LanguageService, PgsToSrt, ProgressReporter, Result};
use pgs_ocr::OcrBackend;

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

/// Resolves `auto` from the file name tag, falling back to an OCR probe over installed models.
fn resolve_language(input: &Path, requested: &str, backend: &dyn OcrBackend) -> Result<String> {
    if !requested.eq_ignore_ascii_case("auto") {
        return Ok(requested.to_string());
    }
    let installed = backend.installed_languages()?;
    let code = match detect_language_from_path(input) {
        Some(code) => code.to_string(),
        None => probe_language_from_file(input, &installed, backend, &AtomicBool::new(false))
            .ok_or_else(|| Error::InvalidInput(LanguageService::get("toast.detect_none").into()))?,
    };
    if !installed.contains(&code) {
        return Err(Error::InvalidInput(format!(
            "{} {code} ({})",
            LanguageService::get("toast.detected_lang"),
            LanguageService::get("badge.missing_model")
        )));
    }
    Ok(code)
}

fn prepare(args: &Args, backend: &dyn OcrBackend) -> Result<(String, PathBuf)> {
    let input = Path::new(args.pgs_file_name.trim());
    if !input.is_file() {
        return Err(Error::File(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", input.display()))));
    }
    let language = resolve_language(input, &args.language, backend)?;
    validate_language(&language)?;

    let output = if args.srt_file_name.is_empty() { output_path(input, &language) } else { PathBuf::from(&args.srt_file_name) };
    let output = if args.force {
        validate_output_name(&output)?;
        output
    } else {
        validate_output(&output).map_err(|error| match error {
            Error::InvalidInput(message) if output.symlink_metadata().is_ok() => {
                Error::InvalidInput(format!("{message} (use --force to overwrite)"))
            }
            other => other,
        })?
    };
    Ok((language, output))
}

fn main() {
    pgs_core::logging::init("pgs2srt", false);

    let args = Args::parse();

    #[cfg(debug_assertions)]
    let pb = ProgressBar::hidden();
    #[cfg(not(debug_assertions))]
    let pb = ProgressBar::new(0);

    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let reporter = IndicatifReporter(&pb);
    reporter.println("[+] Start Processing");

    let backend = tesseract_backend();
    let result = prepare(&args, &backend).and_then(|(language, output)| {
        reporter.println(&format!("[+] Language: {language}"));
        reporter.println(&format!("[+] Output: {}", output.display()));
        PgsToSrt::new(&reporter, &backend).run(args.pgs_file_name.trim(), &language, &output.to_string_lossy())
    });
    match result {
        Ok(_) => {
            reporter.println("[+] Processing Done");
        }
        Err(err) => {
            log::error!("Converting {} failed: {err}", args.pgs_file_name.trim());
            // Not `reporter.println`: the progress bar is hidden in debug builds and would swallow it.
            pb.abandon();
            eprintln!("[-] {err}");
            std::process::exit(1);
        }
    }
}
