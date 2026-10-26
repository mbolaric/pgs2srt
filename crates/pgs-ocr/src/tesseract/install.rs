use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sha1::{Digest, Sha1};

use crate::catalog::{self, ModelInfo};
use crate::{Error, Result};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Downloads `<code>.traineddata` (and its `_vert` companion) from the pinned tessdata_fast revision.
pub(super) fn install(code: &str, directory: &Path) -> Result<()> {
    let model = catalog::find(code).ok_or_else(|| Error::UnknownModel(code.to_owned()))?;
    let vertical = format!("{code}_vert");
    if catalog::is_known(&vertical) {
        install(&vertical, directory)?;
    }
    let destination = directory.join(format!("{code}.traineddata"));
    if destination.is_file() {
        return Ok(());
    }

    let url = format!("{}/{code}.traineddata", catalog::models_url());
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).build().into();
    let mut response = agent.get(&url).call().map_err(|error| Error::Download(error.to_string()))?;
    let mut bytes = Vec::new();
    response.body_mut().as_reader().take(model.size_bytes + 1).read_to_end(&mut bytes)?;
    verify(model, &bytes)?;
    publish(directory, &destination, &bytes)
}

fn verify(model: &ModelInfo, bytes: &[u8]) -> Result<()> {
    let mut digest = Sha1::new();
    digest.update(format!("blob {}\0", bytes.len()).as_bytes());
    digest.update(bytes);
    if bytes.len() as u64 != model.size_bytes || format!("{:x}", digest.finalize()) != model.sha1 {
        return Err(Error::Integrity(model.code.clone()));
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
    fn verify_checks_size_and_git_blob_hash() {
        let model = ModelInfo { code: "test".into(), size_bytes: 5, sha1: "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0".into() };
        assert!(verify(&model, b"hello").is_ok());
        assert!(verify(&model, b"hellO").is_err());
        assert!(verify(&ModelInfo { size_bytes: 6, ..model }, b"hello").is_err());
    }
}
