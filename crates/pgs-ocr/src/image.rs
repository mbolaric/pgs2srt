use crate::error::{Error, Result};

/// 8-bit RGB pixels, row by row without padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcrImage {
    width: usize,
    height: usize,
    rgb: Vec<u8>,
}

impl OcrImage {
    pub fn from_rgb(width: usize, height: usize, rgb: Vec<u8>) -> Result<Self> {
        let expected = width.checked_mul(height).and_then(|pixels| pixels.checked_mul(3));
        if width == 0 || height == 0 || expected != Some(rgb.len()) {
            return Err(Error::InvalidImage(format!("{width}x{height} RGB image with {} bytes", rgb.len())));
        }
        Ok(Self { width, height, rgb })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_must_match_dimensions() {
        assert!(OcrImage::from_rgb(2, 1, vec![0; 6]).is_ok());
        assert!(OcrImage::from_rgb(2, 1, vec![0; 5]).is_err());
        assert!(OcrImage::from_rgb(0, 1, Vec::new()).is_err());
        assert!(OcrImage::from_rgb(usize::MAX, 2, Vec::new()).is_err());
    }
}
