use std::cell::RefCell;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use egui::DroppedFileHandle;
use pgs_core::files::{is_sup, write_atomically};
use pgs_core::ocr::tesseract_backend;
use pgs_core::{I18nService, Preferences};
use pgs_ocr::tesseract::TesseractBackend;
use pgs_ocr::OcrBackend;
use ui_egui::{Capabilities, Input, Job, Mailbox, Platform, WorkerMessage};

/// Worker threads, native dialogs, files next to the inputs and `settings.conf`, with libtesseract for OCR.
pub struct DesktopPlatform {
    backend: Arc<TesseractBackend>,
    workers: RefCell<Vec<JoinHandle<()>>>,
    /// The app draws its own window buttons and resize edges (the window has no system frame).
    window_controls: bool,
}

impl DesktopPlatform {
    pub fn new(window_controls: bool) -> Self {
        Self { backend: Arc::new(tesseract_backend()), workers: RefCell::default(), window_controls }
    }

    fn start(&self, job: Job) -> io::Result<JoinHandle<()>> {
        let backend = Arc::clone(&self.backend);
        std::thread::Builder::new().name("pgs2srt-worker".into()).spawn(move || pollster::block_on(job(&*backend)))
    }
}

fn file_input(path: &Path) -> Input {
    Input::File(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

/// The file itself, or the `.sup` files of a folder.
fn sup_inputs(path: &Path) -> Vec<Input> {
    if !path.is_dir() {
        return vec![file_input(path)];
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && is_sup(p)).collect();
    paths.sort();
    paths.iter().map(|path| file_input(path)).collect()
}

impl Platform for DesktopPlatform {
    fn capabilities(&self) -> Capabilities {
        Capabilities { window_controls: self.window_controls, ..Capabilities::ALL }
    }

    fn backend(&self) -> &dyn OcrBackend {
        &*self.backend
    }

    fn spawn(&self, job: Job) -> io::Result<()> {
        let worker = self.start(job)?;
        let mut workers = self.workers.borrow_mut();
        workers.retain(|worker| !worker.is_finished());
        workers.push(worker);
        Ok(())
    }

    fn spawn_detached(&self, job: Job) -> io::Result<()> {
        self.start(job).map(drop)
    }

    fn pick_inputs(&self, folder: bool, mailbox: Mailbox) {
        let inputs = if folder {
            rfd::FileDialog::new().pick_folder().map(|folder| sup_inputs(&folder))
        } else {
            rfd::FileDialog::new()
                .add_filter("Blu-ray PGS Subtitle (.sup)", &["sup", "SUP"])
                .pick_files()
                .map(|files| files.iter().map(|file| file_input(file)).collect())
        };
        if let Some(inputs) = inputs {
            mailbox.send(WorkerMessage::InputsPicked { inputs, folder });
        }
    }

    fn take_dropped_files(&self, files: Vec<DroppedFileHandle>, mailbox: Mailbox) {
        let inputs = files.iter().flat_map(|file| sup_inputs(file.path())).collect();
        mailbox.send(WorkerMessage::InputsPicked { inputs, folder: false });
    }

    fn pick_output_folder(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    fn pick_output_file(&self, current: &Path) -> Option<PathBuf> {
        let name = current.file_name().and_then(|s| s.to_str()).unwrap_or("subtitle.srt");
        let mut dialog = rfd::FileDialog::new().add_filter("SubRip Subtitle (.srt)", &["srt"]).set_file_name(name);
        if let Some(parent) = current.parent() {
            dialog = dialog.set_directory(parent);
        }
        dialog.save_file()
    }

    fn check_output(&self, output: &Path, allow_overwrite: bool) -> Result<(), String> {
        let parent = output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("{} {}: {e}", I18nService::get("app.cannot_create_output_folder"), parent.display()))?;
        }
        if !allow_overwrite && output.exists() {
            return Err(format!("{}: {}", I18nService::get("files.output_already_exists"), output.display()));
        }
        Ok(())
    }

    fn deliver_srts(&self, srts: &[(&Path, &[u8])]) -> Result<(), String> {
        srts.iter()
            .try_for_each(|(output, srt)| write_atomically(output, |file| Ok(file.write_all(srt)?)).map_err(|e| e.to_string()))
    }

    fn load_preferences(&self) -> Preferences {
        Preferences::load().unwrap_or_else(|e| {
            log::error!("Cannot load settings, using defaults: {e}");
            Preferences::default()
        })
    }

    fn save_preferences(&self, preferences: &Preferences) {
        if let Err(e) = preferences.save() {
            log::error!("Cannot save settings: {e}");
        }
    }

    /// Joins the workers so they release their `TessBaseAPI` before process teardown (`~ObjectCache()`).
    fn shutdown(&self) {
        for worker in self.workers.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for DesktopPlatform {
    fn drop(&mut self) {
        self.shutdown();
    }
}
