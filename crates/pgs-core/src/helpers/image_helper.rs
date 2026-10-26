use pgs::PgsDisplaySet;

use crate::error::{Error, Result};

/// Extracts the decoded true-color RGBA image data for a PGS display set, tightly cropped around
/// visible (`alpha > 0`) subtitle pixels so full-canvas `1920x1080` ODS frames render crisply.
/// Returns `(width, height, rgba_bytes)` where each pixel has 4 bytes `(R, G, B, A)`.
pub fn get_rgba_image(ds: &PgsDisplaySet) -> Result<(usize, usize, Vec<u8>)> {
    let ods = ds.ods.as_ref().ok_or_else(|| Error::ProcessDisplaySet("Subtitle image segment (ODS) is missing.".to_string()))?;
    let width = ods.width as usize;
    let height = ods.height as usize;

    if width == 0 || height == 0 || width.saturating_mul(height) > 16_777_216 {
        return Err(Error::ProcessDisplaySet(format!("Invalid subtitle image dimensions: {width}x{height}")));
    }

    // `gray = false` decodes 32-bit ARGB (`0xAA_RR_GG_BB`) preserving true Blu-ray colors and alpha.
    // (`gray = true` leaves bits 24..31 as `0x00`, which would produce alpha = 0 invisible pixels!)
    let pixels = ds.get_decoded_image(false)?;
    if pixels.len() < height || pixels.iter().take(height).any(|row| row.len() < width) {
        return Err(Error::ProcessDisplaySet("Decoded subtitle dimensions do not match image header.".to_string()));
    }

    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut has_visible = false;
    let mut peak_opaque_rgb = 0u8;

    for (y, row) in pixels.iter().take(height).enumerate() {
        for (x, &pixel) in row.iter().take(width).enumerate() {
            let a = (pixel >> 24) as u8;
            if a > 0 {
                has_visible = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                if a >= 180 {
                    let r = ((pixel >> 16) & 0xFF) as u8;
                    let g = ((pixel >> 8) & 0xFF) as u8;
                    let b = (pixel & 0xFF) as u8;
                    peak_opaque_rgb = peak_opaque_rgb.max(r).max(g).max(b);
                }
            }
        }
    }

    let (x0, y0, x1, y1) = if has_visible {
        (min_x.saturating_sub(2), min_y.saturating_sub(2), (max_x + 3).min(width), (max_y + 3).min(height))
    } else {
        (0, 0, width, height)
    };

    // Expand BT.709 limited-range studio black (Y=16) and HDR-dimmed Blu-ray palettes (e.g. Y=148)
    // so subtitle glyphs render bright and crisp while preserving hue and dark outlines.
    let luma_scale =
        if peak_opaque_rgb > 48 { (242.0 / f32::from(peak_opaque_rgb.saturating_sub(16).max(1))).clamp(1.0, 2.5) } else { 1.0 };

    let out_w = (x1 - x0).max(1);
    let out_h = (y1 - y0).max(1);
    let mut rgba = Vec::with_capacity(out_w * out_h * 4);

    for row in pixels.iter().skip(y0).take(out_h) {
        for &pixel in row.iter().skip(x0).take(out_w) {
            let a = ((pixel >> 24) & 0xFF) as u8;
            if a == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let raw_r = ((pixel >> 16) & 0xFF) as u8;
                let raw_g = ((pixel >> 8) & 0xFF) as u8;
                let raw_b = (pixel & 0xFF) as u8;
                let map_ch = |c: u8| -> u8 {
                    if peak_opaque_rgb > 48 {
                        (f32::from(c.saturating_sub(16)) * luma_scale).round().clamp(0.0, 255.0) as u8
                    } else {
                        c
                    }
                };
                rgba.extend_from_slice(&[map_ch(raw_r), map_ch(raw_g), map_ch(raw_b), a]);
            }
        }
    }

    Ok((out_w, out_h, rgba))
}
