use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::language::LanguageService;

pub fn is_sup(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("sup"))
}

pub fn validate_language(language: &str) -> Result<()> {
    if language.is_empty()
        || language
            .split('/')
            .any(|segment| segment.is_empty() || !segment.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
    {
        return Err(Error::InvalidInput(LanguageService::get("files.use_an_ocr_language_such").into()));
    }
    Ok(())
}

pub fn output_path(input: &Path, language: &str) -> PathBuf {
    input.with_extension(format!("{}.srt", language.replace('/', "_")))
}

pub fn resolve_path(value: &str, base: &Path) -> Result<PathBuf> {
    if value.is_empty() {
        return Err(Error::InvalidInput(LanguageService::get("files.enter_an_output_path").into()));
    }
    let path = if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME");
        #[cfg(windows)]
        let home = home.or_else(|| std::env::var_os("USERPROFILE"));
        let home = home.ok_or_else(|| Error::InvalidInput(LanguageService::get("files.use_an_absolute_path_the").into()))?;
        PathBuf::from(home).join(value.strip_prefix("~/").unwrap_or(""))
    } else {
        PathBuf::from(value)
    };
    Ok(if path.is_absolute() { path } else { base.join(path) })
}

pub fn validate_output_name(output: &Path) -> Result<()> {
    if !output.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("srt")) {
        return Err(Error::InvalidInput(LanguageService::get("files.the_output_filename_must_end").into()));
    }
    Ok(())
}

pub fn validate_output(output: &Path) -> Result<PathBuf> {
    validate_output_name(output)?;
    let parent = output.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|error| {
        Error::InvalidInput(format!("{} {}: {error}", LanguageService::get("files.cannot_open_output_folder"), parent.display()))
    })?;
    if !parent.is_dir() {
        return Err(Error::InvalidInput(LanguageService::get("files.choose_an_output_folder").into()));
    }
    let name =
        output.file_name().ok_or_else(|| Error::InvalidInput(LanguageService::get("files.enter_an_output_filename").into()))?;
    let output = parent.join(name);
    match output.symlink_metadata() {
        Ok(_) => {
            Err(Error::InvalidInput(format!("{}: {}", LanguageService::get("files.output_already_exists"), output.display())))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(output),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_cannot_escape_output_directory() {
        for language in ["", "../eng", "/eng", "eng/", "eng//deu", "eng+", "+eng", " eng", "eng+deu", "script/Latin+deu"] {
            assert!(validate_language(language).is_err());
        }
        assert!(validate_language("eng").is_ok());
        assert!(validate_language("script/Latin").is_ok());
        assert_eq!(output_path(Path::new("/films/a.sup"), "script/Latin"), Path::new("/films/a.script_Latin.srt"));
        assert_eq!(output_path(Path::new("/films/a.SUP"), "eng"), Path::new("/films/a.eng.srt"));
        assert!(is_sup(Path::new("a.SUP")));
    }
}
