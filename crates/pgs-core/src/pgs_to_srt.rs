use std::io::Write;
#[cfg(feature = "native")]
use std::path::Path;

use pgs::PgsDisplaySet;
#[cfg(feature = "native")]
use pgs::PgsParser;
use pgs_ocr::{OcrBackend, Purpose};

#[cfg(feature = "native")]
use crate::cues;
use crate::cues::cues;
use crate::error::{Error, Result};
#[cfg(feature = "native")]
use crate::files;
use crate::helpers::{get_rgba_image, ocr_image, presentation_ts_into_string};
use crate::language::LanguageService;
use crate::progress::ProgressReporter;

pub struct PgsToSrt<'a> {
    reporter: &'a dyn ProgressReporter,
    backend: &'a dyn OcrBackend,
}

fn write_srt_cue(out: &mut impl Write, cue_index: usize, timecode: &str, text: &str) -> Result<()> {
    writeln!(out, "{cue_index}")?;
    writeln!(out, "{timecode}")?;
    writeln!(out, "{}", text.trim_end_matches(['\r', '\n']))?;
    writeln!(out)?;
    Ok(())
}

impl<'a> PgsToSrt<'a> {
    pub fn new(reporter: &'a dyn ProgressReporter, backend: &'a dyn OcrBackend) -> Self {
        PgsToSrt { reporter, backend }
    }

    pub fn get_formatted_presentation_time(&self, start: u32, stop: u32) -> String {
        format!("{} --> {}", presentation_ts_into_string(start), presentation_ts_into_string(stop))
    }

    /// Recognizes every cue of `display_sets` and writes them to `out` as SRT; returns the cue count.
    pub async fn convert(&self, display_sets: &[PgsDisplaySet], language: &str, out: &mut impl Write) -> Result<usize> {
        let cues = cues(display_sets);
        let total_cues = cues.len();
        if total_cues == 0 {
            return Err(Error::ProcessDisplaySet(LanguageService::get("conversion_engine.the_sup_file_contains_no").into()));
        }

        if self.reporter.is_cancelled() {
            return Err(Error::Cancelled);
        }

        let mut engine = self.backend.engine(language, Purpose::Subtitle).await?;

        self.reporter.set_total(total_cues);
        let wants_frames = self.reporter.wants_preview_frames();

        for (index, cue) in cues.iter().enumerate() {
            if self.reporter.is_cancelled() {
                return Err(Error::Cancelled);
            }

            let text = engine.recognize(&ocr_image(cue.display_set)?).await?.text.replace('|', "I");

            let count = index + 1;
            let timecode = self.get_formatted_presentation_time(cue.start_ts, cue.stop_ts);
            write_srt_cue(out, count, &timecode, &text)?;

            if wants_frames {
                let frame = get_rgba_image(cue.display_set).ok();
                self.reporter.on_cue_frame(count, total_cues, cue.start_ts, cue.stop_ts, &text, frame);
            } else {
                self.reporter.on_cue(count, total_cues, cue.start_ts, cue.stop_ts, &text);
            }
            self.reporter.set_position(count);
        }
        Ok(total_cues)
    }
}

#[cfg(feature = "native")]
impl PgsToSrt<'_> {
    /// Returns `output_path`, or `<input dir>/<input stem>.<lang>.srt` when it is empty.
    pub fn get_out_file_name(&self, input_path: &str, output_path: &str, lang: &str) -> String {
        if output_path.is_empty() {
            return files::output_path(Path::new(input_path), lang).to_string_lossy().into_owned();
        }
        output_path.to_string()
    }

    pub fn process_display_set(&self, display_sets: &[PgsDisplaySet], output: &str, language: &str) -> Result<()> {
        files::write_atomically(Path::new(output), |file| pollster::block_on(self.convert(display_sets, language, file)))
            .map(drop)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pgs_ocr::{OcrEngine, OcrFuture, OcrImage, Recognition};

    use super::*;
    use crate::progress::NoOpProgressReporter;
    use crate::test_support::subtitle_display_set;

    struct FixedText;

    impl OcrEngine for FixedText {
        fn recognize<'a>(&'a mut self, _image: &'a OcrImage) -> OcrFuture<'a, Recognition> {
            Box::pin(async { Ok(Recognition { text: "He|lo\n".into(), confidence: 90.0 }) })
        }
    }

    impl OcrBackend for FixedText {
        fn engine<'a>(&'a self, _language: &'a str, _purpose: Purpose) -> OcrFuture<'a, Box<dyn OcrEngine + 'a>> {
            Box::pin(async { Ok(Box::new(FixedText) as Box<dyn OcrEngine>) })
        }

        fn installed_languages(&self) -> pgs_ocr::Result<Vec<String>> {
            Ok(vec!["eng".into()])
        }

        fn install<'a>(&'a self, _code: &'a str) -> OcrFuture<'a, ()> {
            Box::pin(async { Ok(()) })
        }

        fn models_location(&self) -> Option<PathBuf> {
            None
        }
    }

    #[test]
    fn convert_writes_recognized_cues_as_srt() -> Result<()> {
        let converter = PgsToSrt::new(&NoOpProgressReporter, &FixedText);
        let display_sets = [subtitle_display_set(90_000), subtitle_display_set(270_000)];
        let mut srt = Vec::new();
        assert_eq!(pollster::block_on(converter.convert(&display_sets, "eng", &mut srt))?, 2);
        assert_eq!(
            String::from_utf8_lossy(&srt),
            "1\n00:00:01,000 --> 00:00:03,000\nHeIlo\n\n2\n00:00:03,000 --> 00:00:05,000\nHeIlo\n\n"
        );
        assert!(matches!(pollster::block_on(converter.convert(&[], "eng", &mut srt)), Err(Error::ProcessDisplaySet(_))));
        Ok(())
    }
}
