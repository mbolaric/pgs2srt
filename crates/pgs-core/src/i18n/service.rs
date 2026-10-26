use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::catalog::{interpolate, LocaleCatalog};

macro_rules! define_ui_languages {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => {
                code: $code:literal,
                short: $short:literal,
                native_name: $native:literal,
                source: $source:expr $(,)?
            }
        ),+ $(,)?
    ) => {
        /// Supported user-interface display languages (distinct from OCR subtitle languages).
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub enum UiLanguage {
            $(
                $(#[$meta])*
                $variant,
            )+
        }

        impl UiLanguage {
            /// Every registered UI display language in selector display order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// ISO 639-1 code persisted in `settings.conf` (e.g. `"en"`, `"de"`, `"hr"`).
            #[inline]
            pub const fn code(self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)+
                }
            }

            /// Uppercase badge code for compact UI chrome (e.g. `"EN"`, `"DE"`, `"HR"`).
            #[inline]
            pub const fn short_label(self) -> &'static str {
                match self {
                    $(Self::$variant => $short,)+
                }
            }

            /// Native language endonym (always readable regardless of current UI language,
            /// e.g. `"English"`, `"Deutsch"`, `"Hrvatski"`).
            #[inline]
            pub const fn native_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $native,)+
                }
            }

            /// Alias for [`Self::native_name`].
            #[inline]
            pub const fn name(self) -> &'static str {
                self.native_name()
            }

            /// Parses an ISO code strictly, returning `None` if unrecognized.
            pub fn parse_code(code: &str) -> Option<Self> {
                let normalized = code.trim();
                Self::ALL.iter().copied().find(|lang| lang.code().eq_ignore_ascii_case(normalized))
            }

            /// Parses an ISO code, falling back to [`Self::default`] (`English`) if unrecognized.
            pub fn from_code(code: &str) -> Self {
                Self::parse_code(code).unwrap_or_default()
            }

            /// Cycles to the next registered UI language in [`Self::ALL`].
            pub fn next(self) -> Self {
                let idx = self.index();
                Self::ALL[(idx + 1) % Self::ALL.len()]
            }

            #[inline]
            pub(crate) fn index(self) -> usize {
                Self::ALL.iter().position(|&item| item == self).unwrap_or(0)
            }

            pub(crate) fn raw_source(self) -> &'static str {
                match self {
                    $(Self::$variant => $source,)+
                }
            }
        }
    };
}

define_ui_languages! {
    /// English (canonical fallback UI language).
    #[default]
    English => {
        code: "en",
        short: "EN",
        native_name: "English",
        source: include_str!("../../locales/en.lang"),
    },
    /// German (Deutsch).
    German => {
        code: "de",
        short: "DE",
        native_name: "Deutsch",
        source: include_str!("../../locales/de.lang"),
    },
    /// Croatian (Hrvatski).
    Croatian => {
        code: "hr",
        short: "HR",
        native_name: "Hrvatski",
        source: include_str!("../../locales/hr.lang"),
    },
}

static CURRENT_LANG_IDX: AtomicUsize = AtomicUsize::new(0);
static CATALOGS: OnceLock<Vec<LocaleCatalog>> = OnceLock::new();

fn catalogs() -> &'static [LocaleCatalog] {
    CATALOGS.get_or_init(|| UiLanguage::ALL.iter().map(|lang| LocaleCatalog::parse(lang.raw_source())).collect()).as_slice()
}

/// Thread-safe UI localization service with `O(1)` zero-allocation lookups, automatic English
/// fallback, and named `{placeholder}` formatting.
pub struct I18nService;

/// Backwards-compatible alias for [`I18nService`].
pub type LanguageService = I18nService;

impl I18nService {
    /// Returns all supported UI languages.
    #[inline]
    pub const fn supported() -> &'static [UiLanguage] {
        UiLanguage::ALL
    }

    /// Sets the active global UI language.
    #[inline]
    pub fn set(language: UiLanguage) {
        CURRENT_LANG_IDX.store(language.index(), Ordering::Relaxed);
    }

    /// Returns the active global UI language.
    #[inline]
    pub fn current() -> UiLanguage {
        let idx = CURRENT_LANG_IDX.load(Ordering::Relaxed);
        UiLanguage::ALL.get(idx).copied().unwrap_or_default()
    }

    /// Looks up a UI string in `lang` without fallback.
    #[inline]
    pub fn try_get_for(lang: UiLanguage, key: &str) -> Option<&'static str> {
        catalogs().get(lang.index()).and_then(|cat| cat.get(key))
    }

    /// Translates `key` for `lang`, falling back to [`UiLanguage::English`] and finally `key` itself.
    #[inline]
    pub fn tr_for(lang: UiLanguage, key: &'static str) -> &'static str {
        Self::try_get_for(lang, key)
            .or_else(|| (lang != UiLanguage::English).then(|| Self::try_get_for(UiLanguage::English, key)).flatten())
            .unwrap_or(key)
    }

    /// Translates `key` for the currently active [`UiLanguage`] (with English fallback).
    #[inline]
    pub fn get(key: &'static str) -> &'static str {
        Self::tr_for(Self::current(), key)
    }

    /// Translates `key` for `lang` and substitutes `{name}` placeholders.
    pub fn format_for(lang: UiLanguage, key: &'static str, args: &[(&str, &dyn Display)]) -> String {
        interpolate(Self::tr_for(lang, key), args)
    }

    /// Translates `key` for the active [`UiLanguage`] and substitutes `{name}` placeholders.
    pub fn format(key: &'static str, args: &[(&str, &dyn Display)]) -> String {
        Self::format_for(Self::current(), key, args)
    }

    /// Returns the localized display name of a Tesseract OCR language code (e.g. `"hrv"` -> `"Hrvatski"`),
    /// falling back to [`UiLanguage::English`] if not translated in `lang`.
    #[inline]
    pub fn ocr_language_name_for(lang: UiLanguage, code: &str) -> Option<&'static str> {
        let stores = catalogs();
        stores
            .get(lang.index())
            .and_then(|cat| cat.ocr_name(code))
            .or_else(|| (lang != UiLanguage::English).then(|| stores[UiLanguage::English.index()].ocr_name(code)).flatten())
    }

    /// Returns the localized display name of an OCR language code in the active [`UiLanguage`].
    #[inline]
    pub fn ocr_language_name(code: &str) -> Option<&'static str> {
        Self::ocr_language_name_for(Self::current(), code)
    }

    /// Formats an OCR language code as `"Croatian (hrv)"` / `"Hrvatski (hrv)"` / `"Kroatisch (hrv)"`.
    pub fn format_ocr_language(code: &str, lang: UiLanguage) -> String {
        if let Some(name) = Self::ocr_language_name_for(lang, code) {
            format!("{name} ({code})")
        } else {
            code.to_owned()
        }
    }

    /// Returns `true` if `query_lower` (already lowercase) matches the OCR ISO `code` or its
    /// localized display name in the active UI language, English, or any registered UI locale.
    pub fn ocr_language_matches(code: &str, query_lower: &str) -> bool {
        let q = query_lower.trim();
        if q.is_empty() {
            return true;
        }
        if code.to_lowercase().contains(q) {
            return true;
        }
        let stores = catalogs();
        stores.iter().any(|cat| cat.ocr_name(code).is_some_and(|name| name.to_lowercase().contains(q)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::catalog::extract_placeholders;
    use super::*;
    use crate::ocr::catalog_models;

    #[test]
    fn locale_files_have_valid_syntax_no_duplicate_keys_and_complete_parity() {
        for &lang in UiLanguage::ALL {
            let mut seen = HashSet::new();
            for (line_no, raw_line) in lang.raw_source().lines().enumerate() {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some((k, v)) = line.split_once('=') else {
                    panic!("{lang:?} line {} missing '=': {raw_line}", line_no + 1);
                };
                let key = k.trim();
                let val = v.trim();
                assert!(!key.is_empty(), "{lang:?} line {} has empty key", line_no + 1);
                assert!(!val.is_empty(), "{lang:?} line {} key '{key}' has empty translation", line_no + 1);
                assert!(seen.insert(key), "{lang:?} line {} has duplicate key '{key}'", line_no + 1);
            }
        }

        let stores = catalogs();
        let en = &stores[UiLanguage::English.index()];
        let en_keys: HashSet<&str> = en.keys().collect();

        for &lang in UiLanguage::ALL.iter().skip(1) {
            let cat = &stores[lang.index()];
            let lang_keys: HashSet<&str> = cat.keys().collect();

            // 1. Every key in non-English locale must exist in English reference locale
            for &k in &lang_keys {
                assert!(en_keys.contains(k), "{lang:?} contains unknown key '{k}' not present in en.lang");
            }

            // 2. Every UI/core key in en.lang must be translated in every built-in locale, with matching {placeholders}
            for &k in &en_keys {
                let Some(translated) = cat.get(k) else {
                    panic!("{lang:?} is missing translation for key '{k}'");
                };
                let en_text = en.get(k).unwrap();
                assert_eq!(
                    extract_placeholders(en_text),
                    extract_placeholders(translated),
                    "{lang:?} placeholder mismatch for key '{k}'"
                );
            }
        }
    }

    #[test]
    fn every_bundled_ocr_model_has_display_name_in_reference_locale() {
        for model in catalog_models() {
            assert!(
                I18nService::ocr_language_name_for(UiLanguage::English, &model.code).is_some(),
                "Missing ocr.{} display name in en.lang",
                model.code
            );
        }
    }

    #[test]
    fn placeholder_formatting_and_language_cycling_work_across_all_locales() {
        let visited: Vec<UiLanguage> = {
            let mut cur = UiLanguage::default();
            let mut v = Vec::new();
            for _ in 0..UiLanguage::ALL.len() {
                v.push(cur);
                cur = cur.next();
            }
            assert_eq!(cur, UiLanguage::default(), "UiLanguage::next() must cycle back to start");
            v
        };
        assert_eq!(visited.as_slice(), UiLanguage::ALL);

        let msg_en = I18nService::format_for(UiLanguage::English, "conversion_engine.ocr_no_text_in_subtitle", &[("cue", &42)]);
        assert!(msg_en.contains("42") && !msg_en.contains("{cue}"));

        let msg_hr = I18nService::format_for(UiLanguage::Croatian, "conversion_engine.ocr_no_text_in_subtitle", &[("cue", &42)]);
        assert!(msg_hr.contains("42") && msg_hr.contains("podnaslovu"));
    }
}
