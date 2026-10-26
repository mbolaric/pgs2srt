use pgs_ocr::tesseract::TesseractBackend;

use crate::dirs;

/// The libtesseract backend with downloaded models in the user data folder.
pub fn tesseract_backend() -> TesseractBackend {
    TesseractBackend::new(dirs::data_dir().map(|dir| dir.join("tessdata")))
}
