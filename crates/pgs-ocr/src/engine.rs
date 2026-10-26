use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use crate::{OcrImage, Result};

pub type OcrFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// Transcribe a subtitle, trying further page layouts until text is found.
    Subtitle,
    /// Quick read used to compare languages by confidence.
    LanguageProbe,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recognition {
    pub text: String,
    /// Mean word confidence from 0 to 100.
    pub confidence: f32,
}

pub trait OcrEngine {
    fn recognize<'a>(&'a mut self, image: &'a OcrImage) -> OcrFuture<'a, Recognition>;
}

pub trait OcrBackend {
    fn engine<'a>(&'a self, language: &'a str, purpose: Purpose) -> OcrFuture<'a, Box<dyn OcrEngine + 'a>>;

    /// Selectable language codes, sorted and without duplicates.
    fn installed_languages(&self) -> Result<Vec<String>>;

    /// Makes the catalog model `code` available to [`OcrBackend::engine`].
    fn install<'a>(&'a self, code: &'a str) -> OcrFuture<'a, ()>;

    /// The folder holding installed models, when there is one.
    fn models_location(&self) -> Option<PathBuf>;
}
