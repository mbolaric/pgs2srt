use crate::{OcrImage, Result};

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

pub trait OcrEngine: Send {
    fn recognize(&mut self, image: &OcrImage) -> Result<Recognition>;
}

pub trait OcrBackend: Send + Sync {
    fn engine(&self, language: &str, purpose: Purpose) -> Result<Box<dyn OcrEngine>>;

    /// Selectable language codes, sorted and without duplicates.
    fn installed_languages(&self) -> Result<Vec<String>>;
}
