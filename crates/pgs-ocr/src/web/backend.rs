use std::path::PathBuf;

use wasm_bindgen::JsValue;

use super::bridge;
use super::engine::WebTesseractEngine;
use crate::page_modes::page_modes;
use crate::{catalog, Error, OcrBackend, OcrEngine, OcrFuture, Purpose, Result};

/// tesseract.js in the browser. Models come from the pinned `tessdata_fast` revision and stay in the
/// browser cache; the languages loaded so far are remembered in `localStorage`.
#[derive(Default)]
pub struct WebTesseractBackend;

impl WebTesseractBackend {
    pub fn new() -> Self {
        Self
    }
}

fn stored_languages() -> Vec<String> {
    bridge::stored_languages().split(',').filter(|code| catalog::is_known(code)).map(str::to_owned).collect()
}

/// Starts a worker for `language`, downloading its model on first use.
async fn start_worker(language: &str) -> std::result::Result<JsValue, String> {
    let worker = bridge::create_worker(language, &catalog::models_url()).await.map_err(bridge::message)?;
    let mut languages = stored_languages();
    if catalog::is_known(language) && !languages.iter().any(|code| code == language) {
        languages.push(language.to_string());
        languages.sort();
        bridge::store_languages(&languages.join(","));
    }
    Ok(worker)
}

impl OcrBackend for WebTesseractBackend {
    fn engine<'a>(&'a self, language: &'a str, purpose: Purpose) -> OcrFuture<'a, Box<dyn OcrEngine + 'a>> {
        Box::pin(async move {
            let worker = start_worker(language).await.map_err(Error::Engine)?;
            Ok(Box::new(WebTesseractEngine::new(worker, page_modes(purpose))) as Box<dyn OcrEngine>)
        })
    }

    fn installed_languages(&self) -> Result<Vec<String>> {
        Ok(stored_languages())
    }

    fn install<'a>(&'a self, code: &'a str) -> OcrFuture<'a, ()> {
        Box::pin(async move {
            if !catalog::is_known(code) {
                return Err(Error::UnknownModel(code.to_string()));
            }
            bridge::terminate(&start_worker(code).await.map_err(Error::Download)?);
            Ok(())
        })
    }

    fn models_location(&self) -> Option<PathBuf> {
        None
    }
}
