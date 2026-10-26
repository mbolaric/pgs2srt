use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::path::Path;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use zip::result::ZipResult;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

/// Downloads one SRT as itself and several as one ZIP archive.
pub fn download_srts(srts: &[(&Path, &[u8])]) -> Result<(), String> {
    let (name, mime, bytes) = match srts {
        [(output, srt)] => (file_name(output), "application/x-subrip", Cow::Borrowed(*srt)),
        _ => ("subtitles.zip", "application/zip", Cow::Owned(zip(srts).map_err(|e| e.to_string())?)),
    };
    download(name, mime, &bytes).map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|name| name.to_str()).unwrap_or("subtitle.srt")
}

fn zip(files: &[(&Path, &[u8])]) -> ZipResult<Vec<u8>> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).last_modified_time(now());
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let mut names = HashSet::new();
    for (path, bytes) in files {
        let mut name = file_name(path).to_string();
        let mut copy = 1;
        while !names.insert(name.clone()) {
            copy += 1;
            name = numbered(file_name(path), copy);
        }
        archive.start_file(name, options)?;
        archive.write_all(bytes)?;
    }
    Ok(archive.finish()?.into_inner())
}

/// `name.srt` as `name (2).srt`, for outputs that share a file name.
fn numbered(name: &str, copy: u32) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem} ({copy}).{extension}"),
        None => format!("{name} ({copy})"),
    }
}

/// Local time, as ZIP stores no time zone.
fn now() -> DateTime {
    let now = js_sys::Date::new_0();
    DateTime::from_date_and_time(
        now.get_full_year() as u16,
        now.get_month() as u8 + 1,
        now.get_date() as u8,
        now.get_hours() as u8,
        now.get_minutes() as u8,
        now.get_seconds() as u8,
    )
    .unwrap_or_default()
}

/// Hands `bytes` to the browser as a download named `name`.
fn download(name: &str, mime: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let anchor: web_sys::HtmlAnchorElement = document.create_element("a")?.unchecked_into();
    anchor.set_href(&url);
    anchor.set_download(name);
    anchor.click();
    // Some browsers still read the URL after `click` returns.
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    window.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 60_000)?;
    Ok(())
}
