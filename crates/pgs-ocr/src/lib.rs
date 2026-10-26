pub mod catalog;
mod engine;
mod error;
mod image;
mod language;
#[cfg(any(feature = "tesseract", feature = "web"))]
mod page_modes;
#[cfg(feature = "tesseract")]
pub mod tesseract;
#[cfg(feature = "web")]
pub mod web;

pub use catalog::ModelInfo;
pub use engine::{OcrBackend, OcrEngine, OcrFuture, Purpose, Recognition};
pub use error::{Error, Result};
pub use image::OcrImage;
pub use language::is_selectable_language;
