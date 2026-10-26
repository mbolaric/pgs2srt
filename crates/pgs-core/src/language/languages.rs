use std::{
    ffi::{CStr, CString, OsStr},
    path::PathBuf,
    process::Command,
    ptr,
};

use tesseract_plumbing::tesseract_sys as tess;

use crate::error::{Error, Result};
use crate::files;
use crate::helpers::TESSERACT_LIFECYCLE_LOCK;
use crate::i18n::I18nService;
use crate::language::downloads;

pub fn discover(program: &OsStr) -> Result<Vec<String>> {
    let output = Command::new(program).arg("--list-languages").output()?;
    if !output.status.success() {
        return Err(Error::InvalidInput(format!(
            "{}: {}",
            I18nService::get("language_languages.cannot_discover_ocr_languages"),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8(output.stdout).map_err(|_| {
        Error::InvalidInput(I18nService::get("language_languages.tesseract_returned_invalid_language_names").into())
    })?;
    let languages: Vec<_> = text.lines().map(str::to_owned).collect();
    for language in &languages {
        files::validate_language(language)?;
    }
    if languages.is_empty() {
        return Err(Error::InvalidInput(I18nService::get("language_languages.no_tesseract_languages_found_install").into()));
    }
    Ok(languages)
}

/// Returns `true` for user-selectable ISO OCR language models (excluding auxiliary Tesseract
/// script classifiers like `Latin` / `Japanese`, `osd`, `equ`, and `*_vert` companion data).
pub fn is_selectable_ocr_language(code: &str) -> bool {
    !code.is_empty()
        && !code.starts_with(char::is_uppercase)
        && !code.ends_with("_vert")
        && !matches!(code, "osd" | "equ" | "snum")
}

pub fn installed() -> Result<Vec<String>> {
    let mut languages = inventory(data_path()?)?.0;
    if std::env::var_os("TESSDATA_PREFIX").is_none() {
        if let Ok(cache) = downloads::directory() {
            if cache.is_dir() {
                languages.extend(inventory(Some(cache.to_string_lossy().into_owned()))?.0);
            }
        }
    }
    languages.retain(|code| is_selectable_ocr_language(code));
    languages.sort_unstable();
    languages.dedup();
    if languages.is_empty() {
        return Err(Error::InvalidInput(I18nService::get("language_languages.no_tesseract_languages_found_press").into()));
    }
    Ok(languages)
}

pub fn inventory(path: Option<String>) -> Result<(Vec<String>, PathBuf)> {
    let data_path = path
        .map(CString::new)
        .transpose()
        .map_err(|_| Error::InvalidInput(I18nService::get("language_languages.ocr_data_path_contains_a").into()))?;
    let _guard = TESSERACT_LIFECYCLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Tesseract owns the handle and null-terminated strings; copy before freeing each allocation once.
    let inventory = unsafe {
        let api = tess::TessBaseAPICreate();
        if api.is_null() {
            return Err(Error::InvalidInput(
                I18nService::get("language_languages.cannot_initialize_tesseract_language_discovery").into(),
            ));
        }
        if tess::TessBaseAPIInit3(api, data_path.as_ref().map_or(ptr::null(), |path| path.as_ptr()), ptr::null()) != 0 {
            tess::TessBaseAPIDelete(api);
            return Err(Error::InvalidInput(
                I18nService::get("language_languages.cannot_initialize_tesseract_language_discovery").into(),
            ));
        }
        let array = tess::TessBaseAPIGetAvailableLanguagesAsVector(api);
        let mut languages = Vec::new();
        if !array.is_null() {
            let mut current = array;
            while !(*current).is_null() {
                languages.push(CStr::from_ptr(*current).to_string_lossy().into_owned());
                current = current.add(1);
            }
            tess::TessDeleteTextArray(array);
        }
        let directory = tess::TessBaseAPIGetDatapath(api);
        let directory = if directory.is_null() {
            PathBuf::new()
        } else {
            PathBuf::from(CStr::from_ptr(directory).to_string_lossy().into_owned())
        };
        tess::TessBaseAPIDelete(api);
        (languages, directory)
    };
    Ok(inventory)
}

pub fn data_path() -> Result<Option<String>> {
    if std::env::var_os("TESSDATA_PREFIX").is_some() {
        return Ok(None);
    }
    let path = std::env::current_exe()?.with_file_name("tessdata");
    if !path.is_dir() {
        return Ok(None);
    }
    path.into_os_string()
        .into_string()
        .map(Some)
        .map_err(|_| Error::InvalidInput(I18nService::get("language_languages.the_ocr_data_folder_requires").into()))
}

pub fn recognition_data_path(language: &str) -> Result<Option<String>> {
    let base = data_path()?;
    if std::env::var_os("TESSDATA_PREFIX").is_some() {
        return Ok(base);
    }
    let Ok(cache) = downloads::directory() else {
        return Ok(base);
    };
    if !cache.join(format!("{language}.traineddata")).is_file() {
        return Ok(base);
    }
    Ok(Some(
        cache
            .into_os_string()
            .into_string()
            .map_err(|_| Error::InvalidInput(I18nService::get("language_languages.the_ocr_data_folder_requires").into()))?,
    ))
}
