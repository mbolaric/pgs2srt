use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use egui::{
    pos2, vec2, Align, Align2, Color32, ColorImage, CornerRadius, Frame, Layout, Margin, Rect, RichText, Sense, Stroke,
    StrokeKind, TextureHandle, TextureOptions,
};
use pgs_core::files::{is_sup, validate_language, validate_output_name};
use pgs_core::helpers::presentation_ts_into_string;
use pgs_core::ocr::{catalog_models, installed_languages, start_download, tessdata_directory, ModelInfo};
use pgs_core::{I18nService, Preferences, UiLanguage};

use crate::i18n::{format_language_label, language_matches_search, tr};
use crate::icons;
use crate::models::{human_size, ItemStatus, ModelFilter, PreviewData, SubtitleItem, WorkspaceTab};
use crate::theme::{self, ThemeKind, ThemePreference, Tokens};
use crate::widgets::{self, ROW_CONTROL_HEIGHT, TOOLBAR_HEIGHT};
use crate::worker::{spawn_conversion, spawn_language_probe, spawn_sample_loader, WorkerMessage};

pub mod chrome;
mod conversions;
mod inspector;
mod languages;
mod layout;
mod preview;
mod queue;
mod runner;
mod state;

pub use chrome::*;
pub use inspector::{InspectorAction, InspectorView};
pub use languages::{LanguagePickerPopup, LanguagesAction, LanguagesView};
pub use layout::*;
pub use queue::{EmptyQueueView, QueueAction, QueueView};
pub use state::PgsApp;
