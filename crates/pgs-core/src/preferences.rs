use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_SAVE: AtomicU64 = AtomicU64::new(0);

use crate::{
    dirs,
    error::{Error, Result},
    language::{LanguageService, UiLanguage},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Preferences {
    pub ui_language: UiLanguage,
    pub parallel_files: usize,
    pub use_custom_output_dir: bool,
    pub custom_output_dir: Option<PathBuf>,
    pub theme: String,
    pub allow_overwrite: bool,
    pub auto_detect_language: bool,
    pub splitter_position: f32,
    pub show_inspector: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            ui_language: UiLanguage::English,
            parallel_files: 4,
            use_custom_output_dir: false,
            custom_output_dir: None,
            theme: "system".to_string(),
            allow_overwrite: false,
            auto_detect_language: true,
            splitter_position: 340.0,
            show_inspector: true,
        }
    }
}

impl Preferences {
    pub fn load() -> Result<Self> {
        Self::load_from(&path()?)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(Error::InvalidInput(format!("{}: {error}", path.display())));
            }
        };
        let mut preferences = Self::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let invalid = || {
                Error::InvalidInput(format!(
                    "{}: {} {}",
                    path.display(),
                    LanguageService::get("preferences.invalid_settings_line"),
                    index + 1
                ))
            };
            let (key, value) = line.split_once('=').ok_or_else(invalid)?;
            match key {
                "ui_language" => {
                    preferences.ui_language = UiLanguage::parse_code(value).ok_or_else(invalid)?;
                }
                "parallel_files" => {
                    preferences.parallel_files =
                        value.parse::<usize>().ok().filter(|val| (1..=10).contains(val)).ok_or_else(invalid)?;
                }
                "use_custom_output_dir" => {
                    preferences.use_custom_output_dir = value == "true";
                }
                "custom_output_dir" if !value.is_empty() => {
                    preferences.custom_output_dir = Some(PathBuf::from(value));
                }
                "theme" if matches!(value, "system" | "light" | "dark") => {
                    preferences.theme = value.to_string();
                }
                "allow_overwrite" => {
                    preferences.allow_overwrite = value == "true";
                }
                "auto_detect_language" => {
                    preferences.auto_detect_language = value == "true";
                }
                "splitter_position" => {
                    if let Ok(val) = value.parse::<f32>() {
                        preferences.splitter_position = val.clamp(280.0, 480.0);
                    }
                }
                "show_inspector" => {
                    preferences.show_inspector = value == "true";
                }
                _ => {}
            }
        }
        Ok(preferences)
    }

    pub fn save(&self) -> Result<()> {
        let path = path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let parent =
            path.parent().ok_or_else(|| Error::InvalidInput(LanguageService::get("preferences.invalid_settings_path").into()))?;
        fs::create_dir_all(parent)?;
        let (temporary, mut file) = loop {
            let id = NEXT_SAVE.fetch_add(1, Ordering::Relaxed);
            let temporary = parent.join(format!(".settings-{}-{id}.tmp", std::process::id()));
            match fs::File::options().write(true).create_new(true).open(&temporary) {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        };
        let result = (|| -> std::io::Result<()> {
            writeln!(file, "ui_language={}", self.ui_language.code())?;
            writeln!(file, "parallel_files={}", self.parallel_files)?;
            writeln!(file, "use_custom_output_dir={}", self.use_custom_output_dir)?;
            if let Some(dir) = &self.custom_output_dir {
                writeln!(file, "custom_output_dir={}", dir.display())?;
            }
            writeln!(file, "theme={}", self.theme)?;
            writeln!(file, "allow_overwrite={}", self.allow_overwrite)?;
            writeln!(file, "auto_detect_language={}", self.auto_detect_language)?;
            writeln!(file, "splitter_position={:.0}", self.splitter_position.round())?;
            writeln!(file, "show_inspector={}", self.show_inspector)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        let _ = fs::remove_file(&temporary);
        result.map_err(|error| Error::InvalidInput(format!("{}: {error}", path.display())))
    }
}

pub fn path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("settings.conf"))
        .ok_or_else(|| Error::InvalidInput(LanguageService::get("preferences.cannot_find_the_settings_folder").into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip() -> Result<()> {
        let path = std::env::temp_dir().join(format!("pgs2srt-core-prefs-{}.conf", std::process::id()));
        for &ui_language in UiLanguage::ALL {
            let prefs = Preferences {
                ui_language,
                parallel_files: 6,
                use_custom_output_dir: true,
                custom_output_dir: Some(PathBuf::from("/tmp/subtitles")),
                theme: "light".to_string(),
                allow_overwrite: true,
                auto_detect_language: false,
                splitter_position: 380.0,
                show_inspector: false,
            };
            prefs.save_to(&path)?;
            let loaded = Preferences::load_from(&path)?;
            assert_eq!(loaded, prefs);
        }
        fs::remove_file(path)?;
        Ok(())
    }
}
