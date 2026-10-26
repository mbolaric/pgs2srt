use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use egui::DroppedFileHandle;
use pgs_core::Preferences;
use pgs_ocr::{OcrBackend, OcrEngine, OcrFuture, Purpose};

use crate::worker::Mailbox;

pub type JobFuture<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

/// Background work; the platform runs it with its OCR backend.
pub type Job = Box<dyn for<'a> FnOnce(&'a dyn OcrBackend) -> JobFuture<'a> + Send>;

/// The controls a platform supports; the UI hides the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub folders: bool,
    pub output_folder: bool,
    pub save_as: bool,
    pub overwrite: bool,
    /// Saves each SRT as it finishes; otherwise the queue keeps them until the user downloads them.
    pub writes_files: bool,
    pub window_controls: bool,
}

impl Capabilities {
    pub const ALL: Self =
        Self { folders: true, output_folder: true, save_as: true, overwrite: true, writes_files: true, window_controls: true };
    pub const NONE: Self = Self {
        folders: false,
        output_folder: false,
        save_as: false,
        overwrite: false,
        writes_files: false,
        window_controls: false,
    };
}

/// What the UI needs from the system it runs on.
pub trait Platform {
    fn capabilities(&self) -> Capabilities;

    fn backend(&self) -> &dyn OcrBackend;

    /// Runs `job` in the background; the app waits for it on exit.
    fn spawn(&self, job: Job) -> io::Result<()>;

    /// Runs `job` in the background without waiting for it on exit.
    fn spawn_detached(&self, job: Job) -> io::Result<()> {
        self.spawn(job)
    }

    /// Lets the user choose `.sup` files, or a folder of them, and mails them as `InputsPicked`.
    fn pick_inputs(&self, folder: bool, mailbox: Mailbox);

    /// Reads dropped files and folders and mails their `.sup` inputs as `InputsPicked`.
    fn take_dropped_files(&self, files: Vec<DroppedFileHandle>, mailbox: Mailbox);

    fn pick_output_folder(&self) -> Option<PathBuf> {
        None
    }

    fn pick_output_file(&self, _current: &Path) -> Option<PathBuf> {
        None
    }

    /// Checks before a conversion that its SRT can be delivered to `output`.
    fn check_output(&self, _output: &Path, _allow_overwrite: bool) -> Result<(), String> {
        Ok(())
    }

    /// Hands finished SRTs to the user, written to their outputs or downloaded.
    fn deliver_srts(&self, srts: &[(&Path, &[u8])]) -> Result<(), String>;

    /// Tells the platform whether finished SRTs would be lost on exit.
    fn set_unsaved_srts(&self, _unsaved: bool) {}

    fn load_preferences(&self) -> Preferences;

    fn save_preferences(&self, preferences: &Preferences);

    /// Waits for the background work started with [`Platform::spawn`].
    fn shutdown(&self) {}
}

/// A platform without OCR, files or background work, for tests.
pub struct Headless;

impl OcrBackend for Headless {
    fn engine<'a>(&'a self, _language: &'a str, _purpose: Purpose) -> OcrFuture<'a, Box<dyn OcrEngine + 'a>> {
        Box::pin(async { Err(pgs_ocr::Error::NoLanguages) })
    }

    fn installed_languages(&self) -> pgs_ocr::Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn install<'a>(&'a self, code: &'a str) -> OcrFuture<'a, ()> {
        Box::pin(async move { Err(pgs_ocr::Error::UnknownModel(code.to_string())) })
    }

    fn models_location(&self) -> Option<PathBuf> {
        None
    }
}

impl Platform for Headless {
    fn capabilities(&self) -> Capabilities {
        Capabilities::ALL
    }

    fn backend(&self) -> &dyn OcrBackend {
        self
    }

    fn spawn(&self, _job: Job) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    fn pick_inputs(&self, _folder: bool, _mailbox: Mailbox) {}

    fn take_dropped_files(&self, _files: Vec<DroppedFileHandle>, _mailbox: Mailbox) {}

    fn deliver_srts(&self, _srts: &[(&Path, &[u8])]) -> Result<(), String> {
        Ok(())
    }

    fn load_preferences(&self) -> Preferences {
        Preferences::default()
    }

    fn save_preferences(&self, _preferences: &Preferences) {}
}
