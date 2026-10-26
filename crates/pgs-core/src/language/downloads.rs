use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};

use sha1::{Digest, Sha1};

use crate::error::{Error, Result};
use crate::i18n::I18nService;

const MODELS: &str = include_str!("../../models/models.tsv");
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub code: String,
    pub size_bytes: u64,
}

pub fn catalog() -> Vec<String> {
    MODELS.lines().skip(1).filter_map(|line| line.split('\t').next()).map(str::to_owned).collect()
}

pub fn catalog_models() -> Vec<ModelInfo> {
    MODELS
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let code = parts.next()?.to_owned();
            let size_bytes = parts.next()?.parse::<u64>().ok()?;
            Some(ModelInfo { code, size_bytes })
        })
        .collect()
}

pub fn directory() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("TESSDATA_PREFIX") {
        return Ok(path.into());
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    base.map(|base| base.join("pgs2srt/tessdata"))
        .ok_or_else(|| Error::InvalidInput(I18nService::get("language_downloads.cannot_find_the_user_data").into()))
}

pub fn start(code: String) -> Result<Receiver<Result<String>>> {
    if !catalog().contains(&code) {
        return Err(Error::InvalidInput(I18nService::get("language_downloads.choose_a_language_from_the").into()));
    }
    let directory = directory()?;
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new().name("language-download".into()).spawn(move || {
        let result = install(&code, &directory).map(|()| code);
        let _ = sender.send(result);
    })?;
    Ok(receiver)
}

pub fn install(code: &str, directory: &Path) -> Result<()> {
    let model = MODELS
        .lines()
        .skip(1)
        .find_map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            (fields.first() == Some(&code)).then_some(fields)
        })
        .ok_or_else(|| Error::InvalidInput(I18nService::get("language_downloads.unknown_download_language").into()))?;
    let [_, size, hash] = model.as_slice() else {
        return Err(Error::InvalidInput(I18nService::get("language_downloads.invalid_bundled_model_catalog").into()));
    };
    let size =
        size.parse::<u64>().map_err(|_| Error::InvalidInput(I18nService::get("language_downloads.invalid_model_size").into()))?;
    let destination = directory.join(format!("{code}.traineddata"));
    let vertical = format!("{code}_vert");
    if catalog().contains(&vertical) {
        install(&vertical, directory)?;
    }
    if destination.is_file() {
        return Ok(());
    }
    let revision = MODELS
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().last())
        .ok_or_else(|| Error::InvalidInput(I18nService::get("language_downloads.invalid_model_revision").into()))?;
    let url = format!("https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{revision}/{code}.traineddata");
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).build().into();
    let mut response = agent.get(&url).call().map_err(|error| {
        Error::InvalidInput(format!("{}: {error}", I18nService::get("language_downloads.language_download_failed")))
    })?;
    let mut bytes = Vec::new();
    response.body_mut().as_reader().take(size + 1).read_to_end(&mut bytes)?;
    verify_model(&bytes, size, hash)?;
    publish(&destination, &bytes)
}

pub fn verify_model(bytes: &[u8], size: u64, expected: &str) -> Result<()> {
    let mut digest = Sha1::new();
    digest.update(format!("blob {}\0", bytes.len()).as_bytes());
    digest.update(bytes);
    if bytes.len() as u64 != size || format!("{:x}", digest.finalize()) != expected {
        return Err(Error::InvalidInput(I18nService::get("language_downloads.downloaded_language_failed_its_integrity").into()));
    }
    Ok(())
}

pub fn publish(destination: &Path, bytes: &[u8]) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| Error::InvalidInput(I18nService::get("language_downloads.invalid_language_data_path").into()))?;
    fs::create_dir_all(parent)?;
    let (temporary, mut file) = loop {
        let path = parent.join(format!(".download-{}-{}", std::process::id(), NEXT_ID.fetch_add(1, Ordering::Relaxed)));
        match File::options().write(true).create_new(true).open(&path) {
            Ok(file) => break (path, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    };
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        match fs::hard_link(&temporary, destination) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && destination.is_file() => Ok(()),
            Err(error) => Err(error.into()),
        }
    })();
    let _ = fs::remove_file(temporary);
    result
}
