use pgs::PgsDisplaySet;
use std::io::Cursor;
use tiff::encoder::{colortype, ImageEncoder, TiffEncoder};

use crate::error::{Error, Result};

fn num_to_bytes(dest: &mut [u8], num: u64, length: usize) {
    for (i, item) in dest.iter_mut().enumerate().take(length) {
        *item = ((num >> (i * 8)) & 0xFF) as u8;
    }
}

pub fn get_tiff_stream(ds: &PgsDisplaySet) -> Result<Cursor<Vec<u8>>> {
    let ods = ds.ods.as_ref().ok_or_else(|| Error::ProcessDisplaySet("Subtitle image segment (ODS) is missing.".to_string()))?;
    let width = (ods.width) as usize;
    let height = (ods.height) as usize;
    if width == 0 || height == 0 || width.saturating_mul(height) > 16_777_216 {
        return Err(Error::ProcessDisplaySet(format!("Invalid subtitle image dimensions: {width}x{height}")));
    }

    // Decode RLE into pixels array
    let pixels = ds.get_decoded_image(true)?;
    if pixels.len() < height || pixels.iter().take(height).any(|row| row.len() < width) {
        return Err(Error::ProcessDisplaySet("Decoded subtitle dimensions do not match image header.".to_string()));
    }

    encode_rgb_tiff(width, height, &pixels)
}

fn encode_rgb_tiff(width: usize, height: usize, pixels: &[Vec<u32>]) -> Result<Cursor<Vec<u8>>> {
    let samples: usize = 3;
    let mut buffer: Vec<u8> = vec![0; width * samples];
    let mut temp: Vec<u8> = vec![0; width * samples];
    let mut image_buffer: Vec<u8> = vec![0; width * height * samples];

    for i in 0..height {
        for j in 0..width {
            num_to_bytes(&mut temp[j * samples..], pixels[i][j] as u64, samples);
        }
        buffer.copy_from_slice(&temp);
        for (k, byte) in buffer.iter().enumerate() {
            image_buffer[i * width * samples + k] = *byte;
        }
    }
    let mut tiff_stream = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut tiff_stream)?;

    let image: ImageEncoder<&mut Cursor<Vec<u8>>, colortype::RGB8, tiff::encoder::TiffKindStandard> =
        encoder.new_image::<colortype::RGB8>(width as u32, height as u32)?;

    image.write_data(&image_buffer)?;

    Ok(tiff_stream)
}

#[cfg(test)]
mod tests {
    use tesseract::plumbing::leptonica_plumbing::{leptonica_sys, Pix};

    use super::*;

    fn pixel_at(pix: &Pix, x: i32, y: i32) -> u32 {
        let mut value = 0;
        let ptr: *mut leptonica_sys::Pix = *pix.as_ref();
        // SAFETY: `ptr` is a live Pix owned by `pix`; coordinates are within its bounds.
        unsafe { leptonica_sys::pixGetPixel(ptr, x, y, &mut value) };
        value
    }

    #[test]
    fn large_multi_strip_bitmap_round_trips_through_leptonica() -> Result<()> {
        // 1920x400 RGB is ~2.3 MB, so the encoder splits it into several strips.
        let (width, height) = (1920, 400);
        let mut pixels = vec![vec![0u32; width]; height];
        pixels[height - 1][width - 1] = 0x00FF_FFFF;

        let tiff = encode_rgb_tiff(width, height, &pixels)?;
        let pix = Pix::read_mem(tiff.get_ref()).expect("leptonica must decode the encoded TIFF");

        assert_eq!((pix.get_width(), pix.get_height()), (width as i32, height as i32));
        assert_ne!(pixel_at(&pix, width as i32 - 1, height as i32 - 1), 0, "last strip must be decoded");
        assert_eq!(pixel_at(&pix, 0, height as i32 - 1), 0);
        Ok(())
    }
}
