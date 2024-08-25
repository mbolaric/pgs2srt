use std::io::Cursor;
use tiff::{encoder::{colortype, ImageEncoder, TiffEncoder}, tags};
use pgs::PgsDisplaySet;

use crate::srt::error::Result;

fn num_to_bytes(dest: &mut [u8], num: u64, length: usize) {
    for (i, item) in dest.iter_mut().enumerate().take(length) {
        *item = ((num >> (i * 8)) & 0xFF) as u8;
    }
}

fn add_tiff_tag(
    image: &mut ImageEncoder<&mut Cursor<Vec<u8>>, colortype::RGB8, tiff::encoder::TiffKindStandard>,
    tag: tags::Tag,
    value: u32
) -> Result<()> {
    image.encoder().write_tag(tag, value)?;
    Ok(())
}

pub fn get_tiff_stream(ds: &PgsDisplaySet) -> Result<Cursor<Vec<u8>>> {
    let samples = 3;

    let ods = ds.ods.as_ref().unwrap();
    let width = (ods.width) as usize;
    let height = (ods.height) as usize;

    // Decode RLE into pixels array
    let pixels = ds.get_decoded_image(true)?;

    let mut buffer: Vec<u8> = vec![0; width * samples as usize];
    let mut temp: Vec<u8> = vec![0; width * samples as usize];
    let mut image_buffer: Vec<u8> = vec![0; width * height * samples as usize];

    for i in 0..height {
        for j in 0..width {
            num_to_bytes(&mut temp[j * samples as usize..], pixels[i][j] as u64, samples as usize);
        }
        buffer.copy_from_slice(&temp);
        for (k, byte) in buffer.iter().enumerate() {
            image_buffer[i * width * samples as usize + k] = *byte;
        }
    }
    let mut tiff_stream = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut tiff_stream)?;

    let mut image: ImageEncoder<&mut Cursor<Vec<u8>>, colortype::RGB8, tiff::encoder::TiffKindStandard>
        = encoder.new_image::<colortype::RGB8>(width as u32, height as u32,)?;

    add_tiff_tag(&mut image, tags::Tag::PlanarConfiguration, 1)?;
    add_tiff_tag(&mut image, tags::Tag::PhotometricInterpretation, 2)?;
    add_tiff_tag(&mut image, tags::Tag::XResolution, 300)?;
    add_tiff_tag(&mut image, tags::Tag::YResolution, 300)?;
    add_tiff_tag(&mut image, tags::Tag::RowsPerStrip, width as u32 * samples)?;

    image.write_data(&image_buffer)?;

    Ok(tiff_stream)
}