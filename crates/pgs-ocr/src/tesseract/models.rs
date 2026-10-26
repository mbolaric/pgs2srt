use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::Duration;

use sha1::{Digest, Sha1};

use crate::{Error, Result};

/// First line `# tessdata_fast <commit>`, then `<code>\t<size>\t<git blob SHA-1>` per model.
const SOURCE: &str = include_str!("../../models/models.tsv");

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::parse(SOURCE));
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub code: String,
    pub size_bytes: u64,
}

struct Model {
    code: &'static str,
    size: u64,
    sha1: &'static str,
}

struct Catalog {
    revision: &'static str,
    models: Vec<Model>,
}

impl Catalog {
    fn parse(source: &'static str) -> Self {
        let mut lines = source.lines();
        let revision = lines.next().and_then(|line| line.split_whitespace().last()).unwrap_or_default();
        let models = lines
            .filter_map(|line| {
                let mut fields = line.split('\t');
                Some(Model { code: fields.next()?, size: fields.next()?.parse().ok()?, sha1: fields.next()? })
            })
            .collect();
        Self { revision, models }
    }

    fn find(&self, code: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.code == code)
    }
}

pub fn catalog() -> Vec<ModelInfo> {
    CATALOG.models.iter().map(|model| ModelInfo { code: model.code.to_owned(), size_bytes: model.size }).collect()
}

pub fn is_known(code: &str) -> bool {
    CATALOG.find(code).is_some()
}

/// Downloads `<code>.traineddata` (and its `_vert` companion) from the pinned tessdata_fast revision.
pub fn install(code: &str, directory: &Path) -> Result<()> {
    let model = CATALOG.find(code).ok_or_else(|| Error::UnknownModel(code.to_owned()))?;
    let vertical = format!("{code}_vert");
    if is_known(&vertical) {
        install(&vertical, directory)?;
    }
    let destination = directory.join(format!("{code}.traineddata"));
    if destination.is_file() {
        return Ok(());
    }

    let url = format!("https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{}/{code}.traineddata", CATALOG.revision);
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).build().into();
    let mut response = agent.get(&url).call().map_err(|error| Error::Download(error.to_string()))?;
    let mut bytes = Vec::new();
    response.body_mut().as_reader().take(model.size + 1).read_to_end(&mut bytes)?;
    verify(model, &bytes)?;
    publish(directory, &destination, &bytes)
}

fn verify(model: &Model, bytes: &[u8]) -> Result<()> {
    let mut digest = Sha1::new();
    digest.update(format!("blob {}\0", bytes.len()).as_bytes());
    digest.update(bytes);
    if bytes.len() as u64 != model.size || format!("{:x}", digest.finalize()) != model.sha1 {
        return Err(Error::Integrity(model.code.to_owned()));
    }
    Ok(())
}

/// Writes to a temporary file and hard-links it, so readers never see a partial model.
fn publish(directory: &Path, destination: &Path, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(directory)?;
    let (temporary, mut file) = loop {
        let path = directory.join(format!(".download-{}-{}", std::process::id(), NEXT_ID.fetch_add(1, Ordering::Relaxed)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_parses_completely() {
        assert_eq!(CATALOG.models.len(), SOURCE.lines().count() - 1);
        assert_eq!(CATALOG.revision.len(), 40);
        for model in &CATALOG.models {
            assert!(model.size > 0 && model.sha1.len() == 40, "{}", model.code);
        }
        assert!(is_known("eng") && is_known("jpn_vert") && !is_known("xxx"));
    }

    #[test]
    fn verify_checks_size_and_git_blob_hash() {
        let model = Model { code: "test", size: 5, sha1: "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0" };
        assert!(verify(&model, b"hello").is_ok());
        assert!(verify(&model, b"hellO").is_err());
        assert!(verify(&Model { size: 6, ..model }, b"hello").is_err());
    }
}
