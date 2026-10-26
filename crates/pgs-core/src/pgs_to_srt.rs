use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use pgs::{PgsDisplaySet, PgsDisplaySetState, PgsParser};
use tesseract::plumbing::leptonica_plumbing::Pix;
use tesseract::PageSegMode;

use crate::error::{Error, Result};
use crate::files;
use crate::helpers::{get_rgba_image, get_tiff_stream, presentation_ts_into_string, ManagedTesseract};
use crate::language::{recognition_data_path, LanguageService};
use crate::progress::ProgressReporter;

/// Decoded sample subtitle cue used for quick inspection before or during conversion.
#[derive(Clone, Debug)]
pub struct SampleCue {
    pub cue_index: usize,
    pub total_cues: usize,
    pub start_ts: u32,
    pub stop_ts: u32,
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Extracts a representative subtitle bitmap frame and total cue count from a `.sup` file.
pub fn extract_sample_cue(path: &Path) -> Result<Option<SampleCue>> {
    let path_str = path
        .to_str()
        .ok_or_else(|| Error::InvalidInput(LanguageService::get("conversion_engine.the_pgs_parser_requires_a").into()))?;
    let parser = PgsParser::parse(path_str)?;
    let display_sets = parser.get_display_sets();

    let complete_indices: Vec<usize> = display_sets
        .iter()
        .enumerate()
        .filter_map(|(idx, ds)| (ds.state() == PgsDisplaySetState::Complete).then_some(idx))
        .collect();

    let total_cues = complete_indices.len();
    if total_cues == 0 {
        return Ok(None);
    }

    // Prefer the second cue when available (the first cue is occasionally a studio/title card).
    let chosen_pos = usize::from(total_cues > 1);
    let ds_idx = complete_indices[chosen_pos];
    let start = &display_sets[ds_idx];
    let start_ts = start.pcs.as_ref().map_or(0, |pcs| pcs.header.presentation_timestamp);
    let stop_ts = stop_ts_or_default(display_sets, ds_idx, start_ts);

    let (width, height, rgba) = get_rgba_image(start)?;
    Ok(Some(SampleCue { cue_index: chosen_pos + 1, total_cues, start_ts, stop_ts, width, height, rgba }))
}

pub struct PgsToSrt<'a> {
    reporter: &'a dyn ProgressReporter,
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

/// Cue duration used when no following display set ends the cue.
const DEFAULT_CUE_DURATION: u32 = 180_000;

/// Returns the presentation timestamp of the display set after `index`, or `start_ts` plus
/// [`DEFAULT_CUE_DURATION`] when the stream ends without one.
fn stop_ts_or_default(display_sets: &[PgsDisplaySet], index: usize, start_ts: u32) -> u32 {
    display_sets
        .get(index + 1)
        .and_then(|ds| ds.pcs.as_ref())
        .map_or(start_ts.saturating_add(DEFAULT_CUE_DURATION), |pcs| pcs.header.presentation_timestamp)
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
        PgsToSrt { reporter }
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
        let total_cues = display_sets.iter().filter(|ds| ds.state() == PgsDisplaySetState::Complete).count();
        if total_cues == 0 {
            return Err(Error::ProcessDisplaySet(LanguageService::get("conversion_engine.the_sup_file_contains_no").into()));
        }

        if self.reporter.is_cancelled() {
            return Err(Error::Cancelled);
        }

        // Initialize Tesseract BEFORE creating or truncating any output file
        let data_path = recognition_data_path(language).unwrap_or(None);
        let mut api = ManagedTesseract::new(data_path.as_deref(), language)?;

        self.reporter.set_total(total_cues);
        let (guard, mut out) = TempFileGuard::create(output)?;

        let mut count = 0;
        let wants_frames = self.reporter.wants_preview_frames();

        for (i, ds) in display_sets.iter().enumerate() {
            if self.reporter.is_cancelled() {
                return Err(Error::Cancelled);
            }

            if ds.state() == PgsDisplaySetState::Complete {
                let start = ds;
                let start_ts = start
                    .pcs
                    .as_ref()
                    .ok_or_else(|| Error::ProcessDisplaySet("Display set is missing start presentation timestamp.".to_string()))?
                    .header
                    .presentation_timestamp;
                let stop_ts = stop_ts_or_default(display_sets, i, start_ts);

                let text = self.recognize_cue_text(&mut api, start)?;

                count += 1;
                let timecode = self.get_formatted_presentation_time(start_ts, stop_ts);
                write_srt_cue(&mut out, count, &timecode, &text)?;

                if wants_frames {
                    let frame = get_rgba_image(start).ok();
                    self.reporter.on_cue_frame(count, total_cues, start_ts, stop_ts, &text, frame);
                } else {
                    self.reporter.on_cue(count, total_cues, start_ts, stop_ts, &text);
                }
                self.reporter.set_position(count);
            }
        }
        out.sync_all()?;
        drop(out);

        guard.commit(output)?;
        Ok(())
    }

    fn recognize_cue_text(&self, api: &mut ManagedTesseract, ds: &PgsDisplaySet) -> Result<String> {
        let data = get_tiff_stream(ds)?;
        let pix = Pix::read_mem(data.get_ref()).map_err(|e| Error::Tesseract(format!("Failed to set image: {e}")))?;
        let mut text = String::new();

        for mode in [
            PageSegMode::PsmAuto,
            PageSegMode::PsmSingleBlock,
            PageSegMode::PsmSparseText,
            PageSegMode::PsmSingleLine,
            PageSegMode::PsmSingleWord,
        ] {
            if self.reporter.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let tess = api.as_mut();
            tess.set_page_seg_mode(mode.as_tess_page_seg_mode());
            tess.set_image_2(&pix);
            tess.set_source_resolution(300);
            tess.recognize().map_err(|e| Error::Tesseract(format!("OCR recognition failed: {e}")))?;

            text = tess
                .get_utf8_text()
                .map_err(|e| Error::Tesseract(format!("Failed to get text: {e}")))?
                .as_ref()
                .to_string_lossy()
                .into_owned();
            if !text.trim().is_empty() {
                break;
            }
        }
        Ok(text.replace('|', "I"))
    }

    pub fn run(&self, pgs_file_name: &str, language: &str, output: &str) -> Result<()> {
        let path = Path::new(pgs_file_name);
        if !path.exists() {
            return Err(Error::File(std::io::Error::new(std::io::ErrorKind::NotFound, "File not Exists")));
        }

        match PgsParser::parse(pgs_file_name) {
            Ok(parser) => {
                let output_path = self.get_out_file_name(pgs_file_name, output, language);
                self.process_display_set(parser.get_display_sets(), &output_path, language)
            }
            Err(err) => Err(Error::Pgs(err)),
        }
    }

    pub fn run_from_bytes(&self, bytes: impl AsRef<[u8]>, language: &str, output: &str) -> Result<()> {
        match PgsParser::parse_bytes(bytes) {
            Ok(parser) => self.process_display_set(parser.get_display_sets(), output, language),
            Err(err) => Err(Error::Pgs(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use pgs::{PgsPcsSegment, PgsSegmentHeader};

    use super::*;

    fn display_set_at(presentation_timestamp: u32) -> PgsDisplaySet {
        let header = PgsSegmentHeader { presentation_timestamp, ..Default::default() };
        PgsDisplaySet { pcs: Some(Rc::new(PgsPcsSegment { header, ..Default::default() })), ..Default::default() }
    }

    #[test]
    fn stop_timestamp_uses_next_display_set_or_falls_back_for_trailing_cue() {
        let display_sets = [display_set_at(90_000), display_set_at(270_000), PgsDisplaySet::new()];
        assert_eq!(stop_ts_or_default(&display_sets, 0, 90_000), 270_000);
        // Next display set has no PCS, and the last one has no successor at all.
        assert_eq!(stop_ts_or_default(&display_sets, 1, 270_000), 270_000 + DEFAULT_CUE_DURATION);
        assert_eq!(stop_ts_or_default(&display_sets, 2, 450_000), 450_000 + DEFAULT_CUE_DURATION);
        assert_eq!(stop_ts_or_default(&display_sets, 2, u32::MAX), u32::MAX);
    }
}
