use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use pgs::{PgsDisplaySet, PgsParser};
use pgs_ocr::{OcrBackend, Purpose};

use crate::cues::{self, cues};
use crate::error::{Error, Result};
use crate::helpers::{get_rgba_image, ocr_image, presentation_ts_into_string};
use crate::language::LanguageService;
use crate::progress::ProgressReporter;
use crate::{files, ocr};

pub struct PgsToSrt<'a> {
    reporter: &'a dyn ProgressReporter,
    backend: &'a dyn OcrBackend,
}

struct TempFileGuard {
    path: PathBuf,
    active: bool,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

impl TempFileGuard {
    fn create(output: &str) -> Result<(Self, File)> {
        static TEMP_NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let out_path = Path::new(output);
        let file_name = out_path.file_name().and_then(|s| s.to_str()).unwrap_or("subtitles.srt");
        let parent = out_path.parent().unwrap_or_else(|| Path::new(""));
        let temp_path = if parent.as_os_str().is_empty() {
            PathBuf::from(format!(".{file_name}.tmp.{}.{nonce}", std::process::id()))
        } else {
            parent.join(format!(".{file_name}.tmp.{}.{nonce}", std::process::id()))
        };
        let file = File::create(&temp_path)?;
        Ok((Self { path: temp_path, active: true }, file))
    }

    fn commit(mut self, target: &str) -> Result<()> {
        std::fs::rename(&self.path, target)?;
        self.active = false;
        Ok(())
    }
}

fn write_srt_cue(out: &mut File, cue_index: usize, timecode: &str, text: &str) -> Result<()> {
    writeln!(out, "{cue_index}")?;
    writeln!(out, "{timecode}")?;
    writeln!(out, "{}", text.trim_end_matches(['\r', '\n']))?;
    writeln!(out)?;
    Ok(())
}

impl<'a> PgsToSrt<'a> {
    pub fn new(reporter: &'a dyn ProgressReporter) -> Self {
        Self::with_backend(reporter, ocr::backend())
    }

    pub fn with_backend(reporter: &'a dyn ProgressReporter, backend: &'a dyn OcrBackend) -> Self {
        PgsToSrt { reporter, backend }
    }

    pub fn get_formatted_presentation_time(&self, start: u32, stop: u32) -> String {
        format!("{} --> {}", presentation_ts_into_string(start), presentation_ts_into_string(stop))
    }

    /// Returns `output_path`, or `<input dir>/<input stem>.<lang>.srt` when it is empty.
    pub fn get_out_file_name(&self, input_path: &str, output_path: &str, lang: &str) -> String {
        if output_path.is_empty() {
            return files::output_path(Path::new(input_path), lang).to_string_lossy().into_owned();
        }
        output_path.to_string()
    }

    pub fn process_display_set(&self, display_sets: &[PgsDisplaySet], output: &str, language: &str) -> Result<()> {
        let cues = cues(display_sets);
        let total_cues = cues.len();
        if total_cues == 0 {
            return Err(Error::ProcessDisplaySet(LanguageService::get("conversion_engine.the_sup_file_contains_no").into()));
        }

        if self.reporter.is_cancelled() {
            return Err(Error::Cancelled);
        }

        let mut engine = self.backend.engine(language, Purpose::Subtitle)?;

        self.reporter.set_total(total_cues);
        let (guard, mut out) = TempFileGuard::create(output)?;
        let wants_frames = self.reporter.wants_preview_frames();

        for (index, cue) in cues.iter().enumerate() {
            if self.reporter.is_cancelled() {
                return Err(Error::Cancelled);
            }

            let text = engine.recognize(&ocr_image(cue.display_set)?)?.text.replace('|', "I");

            let count = index + 1;
            let timecode = self.get_formatted_presentation_time(cue.start_ts, cue.stop_ts);
            write_srt_cue(&mut out, count, &timecode, &text)?;

            if wants_frames {
                let frame = get_rgba_image(cue.display_set).ok();
                self.reporter.on_cue_frame(count, total_cues, cue.start_ts, cue.stop_ts, &text, frame);
            } else {
                self.reporter.on_cue(count, total_cues, cue.start_ts, cue.stop_ts, &text);
            }
            self.reporter.set_position(count);
        }
        out.sync_all()?;
        drop(out);

        guard.commit(output)?;
        Ok(())
    }

    pub fn run(&self, pgs_file_name: &str, language: &str, output: &str) -> Result<()> {
        let parser = cues::parse_file(Path::new(pgs_file_name))?;
        let output_path = self.get_out_file_name(pgs_file_name, output, language);
        self.process_display_set(parser.get_display_sets(), &output_path, language)
    }

    pub fn run_from_bytes(&self, bytes: impl AsRef<[u8]>, language: &str, output: &str) -> Result<()> {
        let parser = PgsParser::parse_bytes(bytes)?;
        self.process_display_set(parser.get_display_sets(), output, language)
    }
}
