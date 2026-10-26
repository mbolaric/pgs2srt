use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::ptr;

use tesseract_plumbing::tesseract_sys as sys;

use super::engine::TesseractEngine;
use super::ffi::{c_path, lifecycle_guard};
use super::install::install;
use crate::{is_selectable_language, Error, OcrBackend, OcrEngine, OcrFuture, Purpose, Result};

/// Finds `<lang>.traineddata` in the download folder first, then in the bundled `tessdata` folder, and
/// otherwise leaves the lookup to Tesseract (`TESSDATA_PREFIX` or its build-time default).
pub struct TesseractBackend {
    default_dir: Option<PathBuf>,
    download_dir: Option<PathBuf>,
}

impl TesseractBackend {
    /// `user_dir` receives downloaded models; `TESSDATA_PREFIX` replaces both it and the bundled folder.
    pub fn new(user_dir: Option<PathBuf>) -> Self {
        match std::env::var_os("TESSDATA_PREFIX") {
            Some(prefix) => Self { default_dir: None, download_dir: Some(prefix.into()) },
            None => Self { default_dir: bundled_dir(), download_dir: user_dir },
        }
    }

    pub fn download_dir(&self) -> Result<&Path> {
        self.download_dir.as_deref().ok_or(Error::NoDownloadDir)
    }

    fn data_dir(&self, language: &str) -> Option<&Path> {
        self.download_dir
            .as_deref()
            .filter(|dir| dir.join(format!("{language}.traineddata")).is_file())
            .or(self.default_dir.as_deref())
    }
}

impl OcrBackend for TesseractBackend {
    fn engine<'a>(&'a self, language: &'a str, purpose: Purpose) -> OcrFuture<'a, Box<dyn OcrEngine + 'a>> {
        Box::pin(async move {
            let engine = TesseractEngine::new(self.data_dir(language), language, purpose)?;
            Ok(Box::new(engine) as Box<dyn OcrEngine>)
        })
    }

    fn installed_languages(&self) -> Result<Vec<String>> {
        let mut languages = inventory(self.default_dir.as_deref())?;
        if let Some(dir) = self.download_dir.as_deref().filter(|dir| dir.is_dir()) {
            languages.extend(inventory(Some(dir))?);
        }
        languages.retain(|code| is_selectable_language(code));
        languages.sort_unstable();
        languages.dedup();
        if languages.is_empty() {
            return Err(Error::NoLanguages);
        }
        Ok(languages)
    }

    fn install<'a>(&'a self, code: &'a str) -> OcrFuture<'a, ()> {
        Box::pin(async move { install(code, self.download_dir()?) })
    }

    fn models_location(&self) -> Option<PathBuf> {
        self.download_dir.clone()
    }
}

/// `tessdata` next to the executable, or in `Contents/Resources` of a macOS app bundle.
fn bundled_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    #[cfg(target_os = "macos")]
    let in_bundle =
        exe.canonicalize().ok().and_then(|exe| exe.parent()?.parent().map(|contents| contents.join("Resources/tessdata")));
    #[cfg(not(target_os = "macos"))]
    let in_bundle = None;
    std::iter::once(exe.with_file_name("tessdata")).chain(in_bundle).find(|dir| dir.is_dir())
}

fn inventory(dir: Option<&Path>) -> Result<Vec<String>> {
    let dir = dir.map(c_path).transpose()?;
    let _guard = lifecycle_guard();
    // SAFETY: the API handle is created and deleted here; the returned array is copied before it is freed.
    unsafe {
        let api = sys::TessBaseAPICreate();
        if api.is_null() {
            return Err(Error::Inventory);
        }
        if sys::TessBaseAPIInit3(api, dir.as_ref().map_or(ptr::null(), |dir| dir.as_ptr()), ptr::null()) != 0 {
            sys::TessBaseAPIDelete(api);
            return Err(Error::Inventory);
        }
        let array = sys::TessBaseAPIGetAvailableLanguagesAsVector(api);
        let mut languages = Vec::new();
        if !array.is_null() {
            let mut current = array;
            while !(*current).is_null() {
                languages.push(CStr::from_ptr(*current).to_string_lossy().into_owned());
                current = current.add(1);
            }
            sys::TessDeleteTextArray(array);
        }
        sys::TessBaseAPIDelete(api);
        Ok(languages)
    }
}
