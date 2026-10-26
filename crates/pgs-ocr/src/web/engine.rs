use wasm_bindgen::{JsCast, JsValue};

use super::bridge::{self, Recognized};
use crate::page_modes::read_until_text;
use crate::{Error, OcrEngine, OcrFuture, OcrImage, Recognition};

/// One tesseract.js worker with its language loaded; terminated on drop.
pub(super) struct WebTesseractEngine {
    worker: JsValue,
    modes: &'static [u32],
}

impl WebTesseractEngine {
    pub(super) fn new(worker: JsValue, modes: &'static [u32]) -> Self {
        Self { worker, modes }
    }
}

impl OcrEngine for WebTesseractEngine {
    fn recognize<'a>(&'a mut self, image: &'a OcrImage) -> OcrFuture<'a, Recognition> {
        Box::pin(async move {
            let dimension = |value: usize| u32::try_from(value).map_err(|_| Error::InvalidImage(format!("{value} pixels")));
            let (width, height) = (dimension(image.width())?, dimension(image.height())?);
            let worker = &self.worker;
            read_until_text(self.modes, |mode| async move {
                let recognized: Recognized = bridge::recognize(worker, width, height, image.rgb(), mode)
                    .await
                    .map_err(|error| Error::Engine(bridge::message(error)))?
                    .unchecked_into();
                Ok(Recognition { text: recognized.text(), confidence: recognized.confidence().clamp(0.0, 100.0) })
            })
            .await
        })
    }
}

impl Drop for WebTesseractEngine {
    fn drop(&mut self) {
        bridge::terminate(&self.worker);
    }
}
