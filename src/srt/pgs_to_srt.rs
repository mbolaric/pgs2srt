use std::{fs::File, path::Path};
use std::io::Write;

use tesseract::Tesseract;
use indicatif::ProgressBar;

use pgs::{PgsDisplaySet, PgsDisplaySetState, PgsParser};
use crate::srt::helpers::{get_tiff_stream, presentation_ts_into_string};
use crate::srt::error::{Error, Result};

pub struct PgsToSrt<'a> {
    pb: &'a ProgressBar
}

impl<'a> PgsToSrt<'a> {
    pub fn new(pb: &'a ProgressBar) -> Self {
        PgsToSrt { pb }
    }

    fn get_formatted_presentation_time(&self, start: u32, stop: u32) -> String {
        format!("{} --> {}", presentation_ts_into_string(start), presentation_ts_into_string(stop))
    }

    fn get_out_file_name(&self, input_path: &str, output_path: &str, lang: &str) -> String {
        if output_path.is_empty() {
            let path = Path::new(input_path);
            if let Some(file_name) = path.file_stem() {
                return format!("{}.{}.srt", file_name.to_str().unwrap(), lang)
            }
        }
        output_path.to_string()
    }

    fn process_display_set(&self, display_sets: &[PgsDisplaySet], output: &str, language: &str) -> Result<()> {
        self.pb.set_length(display_sets.len() as u64);
        let mut out = File::create(output)?;

        let mut count = 1;
        let mut api = Tesseract::new(None, Some(language)).unwrap();
        for (i, ds) in display_sets.iter().enumerate()  {
            if ds.state() == PgsDisplaySetState::Complete {
                let start = &display_sets[i];
                let stop = &display_sets[i + 1];

                let data = get_tiff_stream(start)?;

                api = api.set_image_from_mem(data.get_ref()).unwrap();
                api = api.recognize().expect("OCR recognition failed");

                let mut text = api.get_text().expect("Failed to get OCR text");
                text = text.replace('|', "I");

                writeln!(out, "{}", count)?;
                writeln!(out, "{}", self.get_formatted_presentation_time(start.pcs.as_ref().unwrap().header.presentation_timestamp, stop.pcs.as_ref().unwrap().header.presentation_timestamp))?;
                writeln!(out, "{}", text)?;

                count += 1;
            }   
            self.pb.set_position(i as u64);
        }
        Ok(())
    }

    pub fn run(&self, pgs_file_name: &str, language: &str, output: &str) -> Result<()> {
        let path = Path::new(pgs_file_name);
        if !path.exists() {
            return Err(Error::File(std::io::Error::new(std::io::ErrorKind::NotFound, "File not Exists")));
        }

        match PgsParser::parse(pgs_file_name) {
            Ok(parser) => {
                let output_path = self.get_out_file_name(pgs_file_name, output, language);
                return self.process_display_set(parser.get_display_sets(), &output_path, language);
            },
            Err(err) => Err(Error::Pgs(err))
        }
    }
}