use std::fmt;

#[derive(Debug)]
pub enum Error {
    Engine(String),
    InvalidImage(String),
    Io(std::io::Error),
    InvalidDataPath,
    Inventory,
    NoLanguages,
    NoDownloadDir,
    UnknownModel(String),
    Download(String),
    Integrity(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Engine(message) => write!(f, "{message}"),
            Error::InvalidImage(message) => write!(f, "Invalid image: {message}"),
            Error::Io(error) => write!(f, "{error}"),
            Error::InvalidDataPath => write!(f, "The OCR data folder requires a UTF-8 path"),
            Error::Inventory => write!(f, "Cannot list the installed OCR languages"),
            Error::NoLanguages => write!(f, "No OCR languages are installed"),
            Error::NoDownloadDir => write!(f, "No folder for downloaded OCR languages"),
            Error::UnknownModel(code) => write!(f, "Unknown OCR language model '{code}'"),
            Error::Download(message) => write!(f, "Language download failed: {message}"),
            Error::Integrity(code) => write!(f, "Downloaded language '{code}' failed its integrity check"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}
