use std::cell::Cell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use egui::DroppedFileHandle;
use pgs_core::Preferences;
use pgs_ocr::web::WebTesseractBackend;
use pgs_ocr::OcrBackend;
use ui_egui::{Capabilities, Input, Job, Mailbox, Platform, WorkerMessage};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::download::download_srts;

const SETTINGS: &str = "pgs2srt.settings";

/// Tasks on the page's event loop, uploads and downloads instead of files, settings in `localStorage`
/// and tesseract.js for OCR.
pub struct WebPlatform {
    backend: Rc<WebTesseractBackend>,
    unsaved: Rc<Cell<bool>>,
}

impl WebPlatform {
    pub fn new() -> Self {
        let unsaved = Rc::default();
        warn_before_leaving(Rc::clone(&unsaved));
        Self { backend: Rc::default(), unsaved }
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Asks the browser to confirm leaving the page while `unsaved` is set.
fn warn_before_leaving(unsaved: Rc<Cell<bool>>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let listener = Closure::<dyn Fn(web_sys::Event)>::new(move |event: web_sys::Event| {
        if unsaved.get() {
            event.prevent_default();
        }
    });
    if window.add_event_listener_with_callback("beforeunload", listener.as_ref().unchecked_ref()).is_ok() {
        listener.forget();
    }
}

impl Platform for WebPlatform {
    fn capabilities(&self) -> Capabilities {
        Capabilities::NONE
    }

    fn backend(&self) -> &dyn OcrBackend {
        &*self.backend
    }

    fn spawn(&self, job: Job) -> io::Result<()> {
        let backend = Rc::clone(&self.backend);
        spawn_local(async move { job(&*backend).await });
        Ok(())
    }

    fn pick_inputs(&self, _folder: bool, mailbox: Mailbox) {
        spawn_local(async move {
            let Some(files) =
                rfd::AsyncFileDialog::new().add_filter("Blu-ray PGS Subtitle (.sup)", &["sup", "SUP"]).pick_files().await
            else {
                return;
            };
            let mut inputs = Vec::with_capacity(files.len());
            for file in files {
                inputs.push(Input::Memory { name: file.file_name(), bytes: Arc::from(file.read().await) });
            }
            mailbox.send(WorkerMessage::InputsPicked { inputs, folder: false });
        });
    }

    fn take_dropped_files(&self, files: Vec<DroppedFileHandle>, mailbox: Mailbox) {
        spawn_local(async move {
            let mut inputs = Vec::with_capacity(files.len());
            for file in &files {
                let name = file.path().file_name().map_or_else(|| "subtitle.sup".into(), |name| name.to_string_lossy());
                match file.bytes_async().await {
                    Ok(bytes) => inputs.push(Input::Memory { name: name.into_owned(), bytes: Arc::from(bytes) }),
                    Err(e) => log::warn!("Cannot read the dropped file {name}: {e}"),
                }
            }
            mailbox.send(WorkerMessage::InputsPicked { inputs, folder: false });
        });
    }

    fn deliver_srts(&self, srts: &[(&Path, &[u8])]) -> Result<(), String> {
        download_srts(srts)
    }

    fn set_unsaved_srts(&self, unsaved: bool) {
        self.unsaved.set(unsaved);
    }

    fn load_preferences(&self) -> Preferences {
        let text = storage().and_then(|storage| storage.get_item(SETTINGS).ok().flatten()).unwrap_or_default();
        Preferences::parse(&text).unwrap_or_else(|e| {
            log::warn!("Cannot read settings, using defaults: {e}");
            Preferences::default()
        })
    }

    fn save_preferences(&self, preferences: &Preferences) {
        if storage().is_none_or(|storage| storage.set_item(SETTINGS, &preferences.to_text()).is_err()) {
            log::warn!("Cannot save settings");
        }
    }
}
