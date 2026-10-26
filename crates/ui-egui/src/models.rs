use std::path::{Path, PathBuf};
use std::sync::Arc;

use pgs_core::language::detect_language_from_path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WorkspaceTab {
    #[default]
    Queue,
    Languages,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ModelFilter {
    #[default]
    All,
    Installed,
    Available,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemStatus {
    Pending,
    Converting { cue: usize, total: usize, start_ts: u32, stop_ts: u32, last_text: String },
    Completed { cues_count: usize },
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct PreviewData {
    pub item_id: usize,
    pub cue: usize,
    pub total: usize,
    pub start_ts: u32,
    pub stop_ts: u32,
    pub text: String,
    pub rgba: Option<(usize, usize, Arc<[u8]>)>,
    pub seq: u64,
}

#[derive(Clone, Debug)]
pub struct SubtitleItem {
    pub id: usize,
    pub input_path: PathBuf,
    pub file_size: u64,
    pub output_path: PathBuf,
    pub language: String,
    pub detected_language: Option<String>,
    pub custom_output_override: bool,
    pub probing_language: bool,
    pub total_cues_hint: Option<usize>,
    pub status: ItemStatus,
}

impl SubtitleItem {
    pub fn new(id: usize, input_path: PathBuf, default_lang: &str, custom_output_dir: Option<&Path>) -> Self {
        let detected = detect_language_from_path(&input_path).map(str::to_string);
        let language = detected.clone().unwrap_or_else(|| default_lang.to_string());
        let file_size = std::fs::metadata(&input_path).map(|m| m.len()).unwrap_or(0);
        let output_path = compute_default_output(&input_path, &language, custom_output_dir);

        Self {
            id,
            input_path,
            file_size,
            output_path,
            language,
            detected_language: detected,
            custom_output_override: false,
            probing_language: false,
            total_cues_hint: None,
            status: ItemStatus::Pending,
        }
    }

    pub fn new_with_fixed_language(id: usize, input_path: PathBuf, fixed_lang: &str, custom_output_dir: Option<&Path>) -> Self {
        let file_size = std::fs::metadata(&input_path).map(|m| m.len()).unwrap_or(0);
        let output_path = compute_default_output(&input_path, fixed_lang, custom_output_dir);

        Self {
            id,
            input_path,
            file_size,
            output_path,
            language: fixed_lang.to_string(),
            detected_language: None,
            custom_output_override: false,
            probing_language: false,
            total_cues_hint: None,
            status: ItemStatus::Pending,
        }
    }

    pub fn update_language(&mut self, new_lang: String, custom_output_dir: Option<&Path>) {
        self.language = new_lang;
        if !self.custom_output_override {
            self.recompute_output_path(custom_output_dir);
        }
    }

    pub fn recompute_output_path(&mut self, custom_output_dir: Option<&Path>) {
        if self.custom_output_override {
            return;
        }
        self.output_path = compute_default_output(&self.input_path, &self.language, custom_output_dir);
    }

    pub fn reset_output_override(&mut self, custom_output_dir: Option<&Path>) {
        self.custom_output_override = false;
        self.recompute_output_path(custom_output_dir);
    }

    pub fn output_file_exists(&self) -> bool {
        self.output_path.exists()
    }
}

fn compute_default_output(input_path: &Path, language: &str, custom_output_dir: Option<&Path>) -> PathBuf {
    let base_out = pgs_core::output_path(input_path, language);
    if let Some(custom_dir) = custom_output_dir {
        if let Some(file_name) = base_out.file_name() {
            return custom_dir.join(file_name);
        }
    }
    base_out
}

pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}
