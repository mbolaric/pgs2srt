use std::fmt;

use crate::i18n::LanguageService;

#[derive(Debug)]
pub enum Error {
    Pgs(pgs::Error),
    File(std::io::Error),
    ProcessDisplaySet(String),
    InvalidInput(String),
    Ocr(String),
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        match self {
            Pgs(e) => write!(f, "Parsing Error ({e})"),
            File(e) => write!(f, "File Read/Write Error ({e})"),
            ProcessDisplaySet(e) => write!(f, "Cannot process Display Sets ({e})"),
            InvalidInput(e) => write!(f, "Invalid input: {e}"),
            Ocr(e) => write!(f, "OCR Error: {e}"),
            Cancelled => write!(f, "Conversion was cancelled"),
        }
    }
}

impl std::error::Error for Error {}

impl From<pgs::Error> for Error {
    fn from(value: pgs::Error) -> Self {
        Error::Pgs(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::File(value)
    }
}

impl From<pgs_ocr::Error> for Error {
    fn from(value: pgs_ocr::Error) -> Self {
        use pgs_ocr::Error as Ocr;
        let localized = |key| Error::InvalidInput(LanguageService::get(key).into());
        match value {
            Ocr::Io(e) => Error::File(e),
            Ocr::Engine(e) => Error::Ocr(e),
            Ocr::InvalidImage(e) => Error::ProcessDisplaySet(e),
            Ocr::InvalidDataPath => localized("language_languages.the_ocr_data_folder_requires"),
            Ocr::Inventory => localized("language_languages.cannot_initialize_tesseract_language_discovery"),
            Ocr::NoLanguages => localized("language_languages.no_tesseract_languages_found_press"),
            Ocr::NoDownloadDir => localized("language_downloads.cannot_find_the_user_data"),
            Ocr::UnknownModel(_) => localized("language_downloads.choose_a_language_from_the"),
            Ocr::Download(e) => {
                Error::InvalidInput(format!("{}: {e}", LanguageService::get("language_downloads.language_download_failed")))
            }
            Ocr::Integrity(_) => localized("language_downloads.downloaded_language_failed_its_integrity"),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
