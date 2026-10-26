//! The OCR backend shared by conversion and language detection, and its downloadable models.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::LazyLock;

use pgs_ocr::tesseract::{models, TesseractBackend};
use pgs_ocr::OcrBackend;

pub use pgs_ocr::tesseract::models::ModelInfo;

use crate::dirs;
use crate::error::Result;

static TESSERACT: LazyLock<TesseractBackend> =
    LazyLock::new(|| TesseractBackend::new(dirs::data_dir().map(|dir| dir.join("tessdata"))));

pub fn backend() -> &'static dyn OcrBackend {
    &*TESSERACT
}

pub fn installed_languages() -> Result<Vec<String>> {
    Ok(TESSERACT.installed_languages()?)
}

pub fn catalog_models() -> Vec<ModelInfo> {
    models::catalog()
}

pub fn tessdata_directory() -> Result<PathBuf> {
    Ok(TESSERACT.download_dir()?.to_path_buf())
}

/// Downloads a catalog model on a background thread; the receiver gets its code once installed.
pub fn start_download(code: String) -> Result<Receiver<Result<String>>> {
    if !models::is_known(&code) {
        return Err(pgs_ocr::Error::UnknownModel(code).into());
    }
    let directory = tessdata_directory()?;
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new().name("language-download".into()).spawn(move || {
        let _ = sender.send(models::install(&code, &directory).map(|()| code).map_err(Into::into));
    })?;
    Ok(receiver)
}
