use std::fmt;

#[derive(Debug)]
pub enum Error {
    Encoding(tiff::TiffError), 
    Pgs(pgs::Error),
    File(std::io::Error),
    ProcessDisplaySet(String)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
		match self {
            Encoding(e) => write!(f, "Tiff Encoding Error ({e})"),
            Pgs(e) => write!(f, "Parsing Error ({e})"),
            File(e) => write!(f, "File Read/Write Error ({e})"),
            ProcessDisplaySet(e) => write!(f, "Cannot process Display Sets ({e})"),
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

impl From<tiff::TiffError> for Error {
    fn from(value: tiff::TiffError) -> Self {
        Error::Encoding(value)
    }
}

pub type Result<T> = core::result::Result<T, Error>;