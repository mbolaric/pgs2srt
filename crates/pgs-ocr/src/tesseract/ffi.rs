use std::ffi::CString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use crate::{Error, Result};

// libtesseract 5 mutates a process-global dictionary cache while initializing and destroying an API.
static LIFECYCLE_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn lifecycle_guard() -> MutexGuard<'static, ()> {
    LIFECYCLE_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

pub(super) fn c_path(path: &Path) -> Result<CString> {
    path.to_str().and_then(|path| CString::new(path).ok()).ok_or(Error::InvalidDataPath)
}
