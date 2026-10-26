use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};

/// A queued `.sup` stream: a file on disk, or bytes handed over without a path (browser uploads).
#[derive(Clone, PartialEq, Eq)]
pub enum Input {
    File(PathBuf),
    Memory { name: String, bytes: Arc<[u8]> },
}

impl Input {
    /// The path output names derive from; only a file name for in-memory inputs.
    pub fn path(&self) -> &Path {
        match self {
            Input::File(path) => path,
            Input::Memory { name, .. } => Path::new(name),
        }
    }

    pub fn name(&self) -> &str {
        self.path().file_name().and_then(|name| name.to_str()).unwrap_or("subtitle.sup")
    }

    pub fn size(&self) -> u64 {
        match self {
            Input::File(path) => fs::metadata(path).map_or(0, |metadata| metadata.len()),
            Input::Memory { bytes, .. } => bytes.len() as u64,
        }
    }

    pub fn bytes(&self) -> io::Result<Cow<'_, [u8]>> {
        match self {
            Input::File(path) => fs::read(path).map(Cow::Owned),
            Input::Memory { bytes, .. } => Ok(Cow::Borrowed(bytes)),
        }
    }
}

impl From<PathBuf> for Input {
    fn from(path: PathBuf) -> Self {
        Input::File(path)
    }
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Input::File(path) => f.debug_tuple("File").field(path).finish(),
            Input::Memory { name, bytes } => f.debug_struct("Memory").field("name", name).field("len", &bytes.len()).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_input_names_outputs_and_lends_its_bytes() -> io::Result<()> {
        let input = Input::Memory { name: "Film.hrv.sup".into(), bytes: Arc::from(&b"PG"[..]) };
        assert_eq!((input.name(), input.path(), input.size()), ("Film.hrv.sup", Path::new("Film.hrv.sup"), 2));
        assert!(matches!(input.bytes()?, Cow::Borrowed(b"PG")));
        assert!(Input::from(PathBuf::from("/missing/a.sup")).bytes().is_err());
        Ok(())
    }
}
