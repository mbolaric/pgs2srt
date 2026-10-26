pub mod detect;

pub use crate::i18n::{I18nService, LanguageService, UiLanguage};
#[cfg(feature = "native")]
pub use detect::probe_language_from_file;
pub use detect::{detect_language_from_filename, detect_language_from_path, probe_language};
