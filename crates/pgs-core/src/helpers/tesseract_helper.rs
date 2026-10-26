use std::ffi::CString;
use std::sync::Mutex;

use tesseract::plumbing::TessBaseApi;

use crate::error::{Error, Result};

/// Global mutex serializing `TessBaseAPI` creation/initialization and destruction.
///
/// In `libtesseract` 5, `TessBaseAPI::Init` and `TessBaseAPI::~TessBaseAPI` mutate the
/// process-global `tesseract::Dict::GlobalDawgCache()` (`ObjectCache`). Serializing
/// init and teardown prevents data races across parallel worker and probe threads while
/// allowing `Recognize()` to run concurrently on separate instances.
pub(crate) static TESSERACT_LIFECYCLE_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct ManagedTesseract(Option<TessBaseApi>);

impl ManagedTesseract {
    pub(crate) fn new(datapath: Option<&str>, language: &str) -> Result<Self> {
        let datapath_c = match datapath {
            Some(p) => Some(CString::new(p).map_err(|e| Error::Tesseract(format!("Invalid tessdata path '{p}': {e}")))?),
            None => None,
        };
        let lang_c = CString::new(language).map_err(|e| Error::Tesseract(format!("Invalid language code '{language}': {e}")))?;

        let _guard = TESSERACT_LIFECYCLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut api = TessBaseApi::create();
        if let Err(e) = api.init_2(datapath_c.as_deref(), Some(lang_c.as_c_str())) {
            drop(api);
            return Err(Error::Tesseract(format!("Failed to initialize Tesseract with language '{language}': {e}")));
        }
        Ok(Self(Some(api)))
    }

    #[inline]
    pub(crate) fn as_mut(&mut self) -> &mut TessBaseApi {
        self.0.as_mut().expect("ManagedTesseract inner instance is always present until Drop")
    }
}

impl Drop for ManagedTesseract {
    fn drop(&mut self) {
        if let Some(api) = self.0.take() {
            let _guard = TESSERACT_LIFECYCLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            drop(api);
        }
    }
}
