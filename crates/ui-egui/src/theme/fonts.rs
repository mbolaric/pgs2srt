use std::path::Path;
use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

fn read_font_file(candidates: &[&str]) -> Option<FontData> {
    for path in candidates {
        if let Ok(bytes) = std::fs::read(Path::new(path)) {
            return Some(FontData::from_owned(bytes));
        }
    }
    None
}

const UI_FONTS: [(&str, &str, &[u8]); 3] = [
    ("regular", "Inter-Regular", include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
    ("medium", "Inter-Medium", include_bytes!("../../assets/fonts/Inter-Medium.ttf")),
    ("semibold", "Inter-SemiBold", include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
];

/// Builds font definitions with crisp Regular, Medium and SemiBold weights from the bundled Inter,
/// and Monospace from an installed system font (egui's built-in fonts remain the fallback).
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    let fallback = fonts.families[&FontFamily::Proportional].clone();
    for (weight, name, bytes) in UI_FONTS {
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
        let family = if weight == "regular" { FontFamily::Proportional } else { FontFamily::Name(weight.into()) };
        let mut stack = vec![name.to_owned()];
        stack.extend(fallback.iter().cloned());
        fonts.families.insert(family, stack);
    }

    if let Some(mono_font) = read_font_file(&[
        "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
        "/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Regular.ttf",
        "C:\\Windows\\Fonts\\cascadiamono.ttf",
        "C:\\Windows\\Fonts\\consola.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]) {
        fonts.font_data.insert("SystemMono".to_owned(), Arc::new(mono_font));
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "SystemMono".to_owned());
    }

    fonts
}

pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}
