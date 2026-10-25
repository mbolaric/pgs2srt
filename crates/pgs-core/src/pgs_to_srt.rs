use std::fs::File;
use std::io::Write;
use std::path::Path;

use pgs::{PgsDisplaySet, PgsDisplaySetState, PgsParser};
use tesseract::Tesseract;

use crate::error::{Error, Result};
use crate::helpers::{get_tiff_stream, presentation_ts_into_string};
use crate::progress::ProgressReporter;

pub struct PgsToSrt<'a> {
    reporter: &'a dyn ProgressReporter,
}

impl<'a> PgsToSrt<'a> {
    pub fn new(reporter: &'a dyn ProgressReporter) -> Self {
        PgsToSrt { reporter }
    }

    pub fn get_formatted_presentation_time(&self, start: u32, stop: u32) -> String {
        format!(
            "{} --> {}",
            presentation_ts_into_string(start),
            presentation_ts_into_string(stop)
        )
    }

    pub fn get_out_file_name(&self, input_path: &str, output_path: &str, lang: &str) -> String {
        if output_path.is_empty() {
            let path = Path::new(input_path);
            if let Some(file_name) = path.file_stem() {
                return format!("{}.{}.srt", file_name.to_str().unwrap(), lang);
            }
        }
        output_path.to_string()
    }

    pub fn process_display_set(
        &self,
        display_sets: &[PgsDisplaySet],
        output: &str,
        language: &str,
    ) -> Result<()> {
        self.reporter.set_total(display_sets.len());
        let mut out = File::create(output)?;

        let mut count = 1;
        let mut api = Tesseract::new(None, Some(language)).unwrap();
        for (i, ds) in display_sets.iter().enumerate() {
            if ds.state() == PgsDisplaySetState::Complete {
                let start = ds;
                if display_sets.len() < i + 1 {
                    return Err(Error::ProcessDisplaySet(
                        "Found Start without stop Display Set.".to_string(),
                    ));
                }
                let stop = &display_sets[i + 1];

                let data = get_tiff_stream(start)?;

                api = api.set_image_from_mem(data.get_ref()).unwrap();
                api = api.recognize().expect("OCR recognition failed");

                let mut text = api.get_text().expect("Failed to get OCR text");
                text = text.replace('|', "I");

                let start_ts = start.pcs.as_ref().unwrap().header.presentation_timestamp;
                let stop_ts = stop.pcs.as_ref().unwrap().header.presentation_timestamp;

                writeln!(out, "{}", count)?;
                writeln!(
                    out,
                    "{}",
                    self.get_formatted_presentation_time(start_ts, stop_ts)
                )?;
                writeln!(out, "{}", text)?;

                count += 1;
            }
            self.reporter.set_position(i);
        }
        Ok(())
    }

    pub fn run(&self, pgs_file_name: &str, language: &str, output: &str) -> Result<()> {
        let path = Path::new(pgs_file_name);
        if !path.exists() {
            return Err(Error::File(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not Exists",
            )));
        }

        match PgsParser::parse(pgs_file_name) {
            Ok(parser) => {
                let output_path = self.get_out_file_name(pgs_file_name, output, language);
                self.process_display_set(parser.get_display_sets(), &output_path, language)
            }
            Err(err) => Err(Error::Pgs(err)),
        }
    }

    pub fn run_from_bytes(
        &self,
        bytes: impl AsRef<[u8]>,
        language: &str,
        output: &str,
    ) -> Result<()> {
        match PgsParser::parse_bytes(bytes) {
            Ok(parser) => self.process_display_set(parser.get_display_sets(), output, language),
            Err(err) => Err(Error::Pgs(err)),
        }
    }
}
