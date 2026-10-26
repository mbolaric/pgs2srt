use std::path::PathBuf;
#[cfg(feature = "native")]
use std::{fs, io::Write, path::Path};

#[cfg(feature = "native")]
use crate::{dirs, files};
use crate::{
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
    pub fn parse(text: &str) -> Result<Self> {
        let mut preferences = Self::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let invalid =
                || Error::InvalidInput(format!("{} {}", LanguageService::get("preferences.invalid_settings_line"), index + 1));
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

    pub fn to_text(&self) -> String {
        let custom_output_dir =
            self.custom_output_dir.as_ref().map(|dir| format!("custom_output_dir={}\n", dir.display())).unwrap_or_default();
        format!(
            "ui_language={}\n\
             parallel_files={}\n\
             use_custom_output_dir={}\n\
             {custom_output_dir}\
             theme={}\n\
             allow_overwrite={}\n\
             auto_detect_language={}\n\
             splitter_position={:.0}\n\
             show_inspector={}\n",
            self.ui_language.code(),
            self.parallel_files,
            self.use_custom_output_dir,
            self.theme,
            self.allow_overwrite,
            self.auto_detect_language,
            self.splitter_position.round(),
            self.show_inspector,
        )
    }
}

#[cfg(feature = "native")]
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
        Self::parse(&text).map_err(|error| match error {
            Error::InvalidInput(message) => Error::InvalidInput(format!("{}: {message}", path.display())),
            error => error,
        })
    }

    pub fn save(&self) -> Result<()> {
        let path = path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let parent =
            path.parent().ok_or_else(|| Error::InvalidInput(LanguageService::get("preferences.invalid_settings_path").into()))?;
        fs::create_dir_all(parent)?;
        files::write_atomically(path, |file| Ok(file.write_all(self.to_text().as_bytes())?))
            .map_err(|error| Error::InvalidInput(format!("{}: {error}", path.display())))
    }
}

#[cfg(feature = "native")]
pub fn path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("settings.conf"))
        .ok_or_else(|| Error::InvalidInput(LanguageService::get("preferences.cannot_find_the_settings_folder").into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn customized(ui_language: UiLanguage) -> Preferences {
        Preferences {
            ui_language,
            parallel_files: 6,
            use_custom_output_dir: true,
            custom_output_dir: Some(PathBuf::from("/tmp/subtitles")),
            theme: "light".to_string(),
            allow_overwrite: true,
            auto_detect_language: false,
            splitter_position: 380.0,
            show_inspector: false,
        }
    }

    #[test]
    fn preferences_text_round_trip() -> Result<()> {
        for &ui_language in UiLanguage::ALL {
            let prefs = customized(ui_language);
            assert_eq!(Preferences::parse(&prefs.to_text())?, prefs);
        }
        assert_eq!(Preferences::parse(&Preferences::default().to_text())?, Preferences::default());
        assert!(Preferences::parse("parallel_files=0").is_err());
        Ok(())
    }

    #[cfg(feature = "native")]
    #[test]
    fn preferences_file_round_trip() -> Result<()> {
        let path = std::env::temp_dir().join(format!("pgs2srt-core-prefs-{}.conf", std::process::id()));
        for &ui_language in UiLanguage::ALL {
            let prefs = customized(ui_language);
            prefs.save_to(&path)?;
            assert_eq!(Preferences::load_from(&path)?, prefs);
        }
        fs::remove_file(path)?;
        Ok(())
    }
}
