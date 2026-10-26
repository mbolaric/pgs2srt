use std::ffi::CString;
use std::future::ready;
use std::os::raw::c_int;
use std::path::Path;

use tesseract_plumbing::tesseract_sys::TessPageSegMode;
use tesseract_plumbing::TessBaseApi;

use super::ffi::{c_path, lifecycle_guard};
use crate::page_modes::{page_modes, read_until_text};
use crate::{Error, OcrEngine, OcrFuture, OcrImage, Purpose, Recognition, Result};

pub(super) struct TesseractEngine {
    api: Option<TessBaseApi>,
    modes: &'static [u32],
}

impl TesseractEngine {
    pub(super) fn new(data_dir: Option<&Path>, language: &str, purpose: Purpose) -> Result<Self> {
        let data_dir = data_dir.map(c_path).transpose()?;
        let language_c = CString::new(language).map_err(|_| Error::Engine(format!("Invalid language code '{language}'")))?;
        let modes = page_modes(purpose);

        let _guard = lifecycle_guard();
        let mut api = TessBaseApi::create();
        api.init_2(data_dir.as_deref(), Some(&language_c))
            .map_err(|error| Error::Engine(format!("Failed to initialize Tesseract with language '{language}': {error}")))?;
        Ok(Self { api: Some(api), modes })
    }
}

impl OcrEngine for TesseractEngine {
    fn recognize<'a>(&'a mut self, image: &'a OcrImage) -> OcrFuture<'a, Recognition> {
        Box::pin(async move {
            let dimension = |value: usize| c_int::try_from(value).map_err(|_| Error::InvalidImage(format!("{value} pixels")));
            let size = (dimension(image.width())?, dimension(image.height())?, dimension(image.width() * 3)?);
            let api = self.api.as_mut().expect("the API lives until drop");
            read_until_text(self.modes, |mode| ready(read(api, image.rgb(), size, mode))).await
        })
    }
}

fn read(api: &mut TessBaseApi, rgb: &[u8], (width, height, line): (c_int, c_int, c_int), mode: u32) -> Result<Recognition> {
    // A C enum: `u32` on Linux and macOS, `i32` on Windows.
    api.set_page_seg_mode(mode as TessPageSegMode);
    api.set_image(rgb, width, height, 3, line).map_err(|error| Error::Engine(format!("Failed to set image: {error}")))?;
    api.set_source_resolution(300);
    api.recognize().map_err(|error| Error::Engine(format!("OCR recognition failed: {error}")))?;
    let confidence = api.mean_text_conf().clamp(0, 100) as f32;
    let text = api.get_utf8_text().map_err(|error| Error::Engine(format!("Failed to get text: {error}")))?;
    Ok(Recognition { text: text.as_ref().to_string_lossy().into_owned(), confidence })
}

impl Drop for TesseractEngine {
    fn drop(&mut self) {
        if let Some(api) = self.api.take() {
            let _guard = lifecycle_guard();
            drop(api);
        }
    }
}
