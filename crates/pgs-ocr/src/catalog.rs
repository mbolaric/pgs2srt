use std::sync::LazyLock;

/// First line `# tessdata_fast <commit>`, then `<code>\t<size>\t<git blob SHA-1>` per model.
const SOURCE: &str = include_str!("../models/models.tsv");

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::parse(SOURCE));

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelInfo {
    pub code: String,
    pub size_bytes: u64,
    pub sha1: String,
}

struct Catalog {
    revision: &'static str,
    models: Vec<ModelInfo>,
}

impl Catalog {
    fn parse(source: &'static str) -> Self {
        let mut lines = source.lines();
        let revision = lines.next().and_then(|line| line.split_whitespace().last()).unwrap_or_default();
        let models = lines
            .filter_map(|line| {
                let mut fields = line.split('\t');
                Some(ModelInfo {
                    code: fields.next()?.to_owned(),
                    size_bytes: fields.next()?.parse().ok()?,
                    sha1: fields.next()?.to_owned(),
                })
            })
            .collect();
        Self { revision, models }
    }
}

pub fn models() -> Vec<ModelInfo> {
    CATALOG.models.clone()
}

pub fn find(code: &str) -> Option<&'static ModelInfo> {
    CATALOG.models.iter().find(|model| model.code == code)
}

pub fn is_known(code: &str) -> bool {
    find(code).is_some()
}

/// The pinned tessdata_fast folder serving `<code>.traineddata`.
pub fn models_url() -> String {
    format!("https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{}", CATALOG.revision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_parses_completely() {
        assert_eq!(CATALOG.models.len(), SOURCE.lines().count() - 1);
        assert_eq!(CATALOG.revision.len(), 40);
        for model in &CATALOG.models {
            assert!(model.size_bytes > 0 && model.sha1.len() == 40, "{}", model.code);
        }
        assert!(is_known("eng") && is_known("jpn_vert") && !is_known("xxx"));
        assert!(models_url().ends_with(CATALOG.revision));
    }
}
