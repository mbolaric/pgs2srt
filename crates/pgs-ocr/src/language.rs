/// Excludes auxiliary models that are not a spoken language: script classifiers (`Latin`), `osd`, `equ`,
/// `snum` and the `*_vert` companions of vertical-text languages.
pub fn is_selectable_language(code: &str) -> bool {
    !code.is_empty()
        && !code.starts_with(char::is_uppercase)
        && !code.ends_with("_vert")
        && !matches!(code, "osd" | "equ" | "snum")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auxiliary_models_are_not_selectable() {
        for code in ["eng", "chi_sim", "srp_latn", "jpn"] {
            assert!(is_selectable_language(code), "{code}");
        }
        for code in ["", "Latin", "osd", "equ", "snum", "jpn_vert"] {
            assert!(!is_selectable_language(code), "{code}");
        }
    }
}
