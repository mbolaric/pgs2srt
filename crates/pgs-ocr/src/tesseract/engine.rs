use std::ffi::CString;
use std::os::raw::c_int;
use std::path::Path;

use tesseract_plumbing::tesseract_sys::{self as sys, TessPageSegMode};
use tesseract_plumbing::TessBaseApi;

use super::ffi::{c_path, lifecycle_guard};
use crate::{Error, OcrEngine, OcrImage, Purpose, Recognition, Result};

const SUBTITLE_MODES: &[TessPageSegMode] = &[
    sys::TessPageSegMode_PSM_AUTO,
    sys::TessPageSegMode_PSM_SINGLE_BLOCK,
    sys::TessPageSegMode_PSM_SPARSE_TEXT,
    sys::TessPageSegMode_PSM_SINGLE_LINE,
    sys::TessPageSegMode_PSM_SINGLE_WORD,
];
const PROBE_MODES: &[TessPageSegMode] = &[sys::TessPageSegMode_PSM_SINGLE_BLOCK, sys::TessPageSegMode_PSM_AUTO];

pub(super) struct TesseractEngine {
    api: Option<TessBaseApi>,
    modes: &'static [TessPageSegMode],
}

impl TesseractEngine {
    pub(super) fn new(data_dir: Option<&Path>, language: &str, purpose: Purpose) -> Result<Self> {
        let data_dir = data_dir.map(c_path).transpose()?;
        let language_c = CString::new(language).map_err(|_| Error::Engine(format!("Invalid language code '{language}'")))?;
        let modes = match purpose {
            Purpose::Subtitle => SUBTITLE_MODES,
            Purpose::LanguageProbe => PROBE_MODES,
        };

        let _guard = lifecycle_guard();
        let mut api = TessBaseApi::create();
        api.init_2(data_dir.as_deref(), Some(&language_c))
            .map_err(|error| Error::Engine(format!("Failed to initialize Tesseract with language '{language}': {error}")))?;
        Ok(Self { api: Some(api), modes })
    }
}

impl OcrEngine for TesseractEngine {
    fn recognize(&mut self, image: &OcrImage) -> Result<Recognition> {
        let dimension = |value: usize| c_int::try_from(value).map_err(|_| Error::InvalidImage(format!("{value} pixels")));
        let (width, height, line) = (dimension(image.width())?, dimension(image.height())?, dimension(image.width() * 3)?);
        let api = self.api.as_mut().expect("the API lives until drop");

        let mut recognition = Recognition::default();
        for &mode in self.modes {
            api.set_page_seg_mode(mode);
            api.set_image(image.rgb(), width, height, 3, line)
                .map_err(|error| Error::Engine(format!("Failed to set image: {error}")))?;
            api.set_source_resolution(300);
            api.recognize().map_err(|error| Error::Engine(format!("OCR recognition failed: {error}")))?;
            let confidence = api.mean_text_conf().clamp(0, 100) as f32;
            let text = api.get_utf8_text().map_err(|error| Error::Engine(format!("Failed to get text: {error}")))?;
            recognition = Recognition { text: text.as_ref().to_string_lossy().into_owned(), confidence };
            if !recognition.text.trim().is_empty() {
                break;
            }
        }
        Ok(recognition)
    }
}

impl Drop for TesseractEngine {
    fn drop(&mut self) {
        if let Some(api) = self.api.take() {
            let _guard = lifecycle_guard();
            drop(api);
        }
    }
}
