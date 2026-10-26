use std::future::Future;

use crate::{Purpose, Recognition, Result};

const AUTO: u32 = 3;
const SINGLE_BLOCK: u32 = 6;
const SINGLE_LINE: u32 = 7;
const SINGLE_WORD: u32 = 8;
const SPARSE_TEXT: u32 = 11;

/// Tesseract page segmentation modes, tried in order until one finds text.
pub(crate) fn page_modes(purpose: Purpose) -> &'static [u32] {
    match purpose {
        Purpose::Subtitle => &[AUTO, SINGLE_BLOCK, SPARSE_TEXT, SINGLE_LINE, SINGLE_WORD],
        Purpose::LanguageProbe => &[SINGLE_BLOCK, AUTO],
    }
}

/// Reads with each of `modes` in turn and returns the first recognition with text, else the last one.
pub(crate) async fn read_until_text<R>(modes: &[u32], mut read: impl FnMut(u32) -> R) -> Result<Recognition>
where
    R: Future<Output = Result<Recognition>>,
{
    let mut recognition = Recognition::default();
    for &mode in modes {
        recognition = read(mode).await?;
        if !recognition.text.trim().is_empty() {
            break;
        }
    }
    Ok(recognition)
}

#[cfg(all(test, feature = "tesseract"))]
mod tests {
    use tesseract_plumbing::tesseract_sys as sys;

    use super::*;

    #[test]
    fn modes_match_libtesseract() {
        assert_eq!(
            [AUTO, SINGLE_BLOCK, SINGLE_LINE, SINGLE_WORD, SPARSE_TEXT].map(|mode| mode as sys::TessPageSegMode),
            [
                sys::TessPageSegMode_PSM_AUTO,
                sys::TessPageSegMode_PSM_SINGLE_BLOCK,
                sys::TessPageSegMode_PSM_SINGLE_LINE,
                sys::TessPageSegMode_PSM_SINGLE_WORD,
                sys::TessPageSegMode_PSM_SPARSE_TEXT,
            ]
        );
    }
}
