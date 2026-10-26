mod engine;
mod error;
mod image;
mod language;
#[cfg(feature = "tesseract")]
pub mod tesseract;

pub use engine::{OcrBackend, OcrEngine, Purpose, Recognition};
pub use error::{Error, Result};
pub use image::OcrImage;
pub use language::is_selectable_language;
