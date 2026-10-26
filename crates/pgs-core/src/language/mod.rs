pub mod detect;

pub use crate::i18n::{I18nService, LanguageService, UiLanguage};
pub use detect::{
    detect_language_from_filename, detect_language_from_path, probe_language_from_display_sets, probe_language_from_file,
};
