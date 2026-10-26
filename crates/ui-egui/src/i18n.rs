//! Desktop UI localization helpers backed by [`pgs_core::I18nService`].
//!
//! All locale strings live in per-language `.lang` files under `crates/pgs-core/locales/`.

use std::fmt::Display;

use pgs_core::{I18nService, UiLanguage};

/// Translates `key` into the currently active [`UiLanguage`] (with English fallback).
#[inline]
pub fn tr(key: &'static str) -> &'static str {
    I18nService::get(key)
}

/// Translates `key` into the specified [`UiLanguage`] (with English fallback).
#[inline]
pub fn tr_for(lang: UiLanguage, key: &'static str) -> &'static str {
    I18nService::tr_for(lang, key)
}

/// Translates `key` into the active [`UiLanguage`] and substitutes `{name}` placeholders.
#[inline]
pub fn tr_fmt(key: &'static str, args: &[(&str, &dyn Display)]) -> String {
    I18nService::format(key, args)
}

/// Returns the localized human-readable display name for a Tesseract OCR language code.
#[inline]
pub fn language_display_name(code: &str, ui_lang: UiLanguage) -> Option<&'static str> {
    I18nService::ocr_language_name_for(ui_lang, code)
}

/// Formats a Tesseract OCR language code as `"Croatian (hrv)"` / `"Hrvatski (hrv)"`.
#[inline]
pub fn format_language_label(code: &str, ui_lang: UiLanguage) -> String {
    I18nService::format_ocr_language(code, ui_lang)
}

/// Returns `true` if `query_lower` matches the OCR ISO `code` or any localized language name.
#[inline]
pub fn language_matches_search(code: &str, query_lower: &str) -> bool {
    I18nService::ocr_language_matches(code, query_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_strings_and_ocr_labels_resolve_across_all_supported_ui_languages() {
        for &lang in I18nService::supported() {
            let add_files = tr_for(lang, "action.add_files");
            assert!(!add_files.is_empty() && add_files != "action.add_files");

            let label = format_language_label("hrv", lang);
            assert!(label.ends_with("(hrv)") && label.len() > 6);
        }

        assert!(language_matches_search("hrv", "croatian"));
        assert!(language_matches_search("hrv", "hrvatski"));
        assert!(language_matches_search("hrv", "kroatisch"));
        assert!(language_matches_search("hrv", "hrv"));
    }
}
