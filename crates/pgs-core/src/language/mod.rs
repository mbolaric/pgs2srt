pub mod detect;
pub mod downloads;
pub mod languages;

pub use crate::i18n::{I18nService, LanguageService, UiLanguage};
pub use detect::{
    detect_language_from_filename, detect_language_from_path, probe_language_from_display_sets,
    probe_language_from_display_sets_cancellable, probe_language_from_file, probe_language_from_file_cancellable,
    probe_language_from_tiff_samples, probe_language_from_tiff_samples_cancellable,
};
pub use downloads::{
    catalog as download_catalog, catalog_models, directory as tessdata_directory, install as install_language,
    start as start_download, ModelInfo,
};
pub use languages::{data_path, installed as installed_languages, is_selectable_ocr_language, recognition_data_path};
