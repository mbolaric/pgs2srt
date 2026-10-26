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

/// Builds font definitions with crisp Regular, Medium, SemiBold, and Monospace weights from
/// installed system fonts (with graceful fallback to egui's built-in fonts).
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    #[cfg(not(target_arch = "wasm32"))]
    {
        type FontFamilySet<'a> = (&'a str, &'a [&'a str], &'a [&'a str], &'a [&'a str]);
        // Matched UI sans-serif families in priority order so Regular/Medium/SemiBold share metrics.
        let matched_sets: &[FontFamilySet<'_>] = &[
            (
                "Inter",
                &[
                    "/usr/share/fonts/inter/Inter-Regular.otf",
                    "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
                    "/usr/share/fonts/opentype/inter/Inter-Regular.otf",
                ],
                &[
                    "/usr/share/fonts/inter/Inter-Medium.otf",
                    "/usr/share/fonts/truetype/inter/Inter-Medium.ttf",
                    "/usr/share/fonts/opentype/inter/Inter-Medium.otf",
                ],
                &[
                    "/usr/share/fonts/inter/Inter-SemiBold.otf",
                    "/usr/share/fonts/truetype/inter/Inter-SemiBold.ttf",
                    "/usr/share/fonts/opentype/inter/Inter-SemiBold.otf",
                ],
            ),
            (
                "SegoeUI",
                &["C:\\Windows\\Fonts\\segoeui.ttf"],
                &["C:\\Windows\\Fonts\\seguisb.ttf", "C:\\Windows\\Fonts\\segoeui.ttf"],
                &["C:\\Windows\\Fonts\\seguisb.ttf", "C:\\Windows\\Fonts\\segoeuib.ttf"],
            ),
            (
                "Lato",
                &["/usr/share/fonts/truetype/lato/Lato-Regular.ttf"],
                &["/usr/share/fonts/truetype/lato/Lato-Medium.ttf"],
                &["/usr/share/fonts/truetype/lato/Lato-Semibold.ttf", "/usr/share/fonts/truetype/lato/Lato-Bold.ttf"],
            ),
            (
                "NotoSans",
                &["/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", "/usr/share/fonts/noto/NotoSans-Regular.ttf"],
                &[
                    "/usr/share/fonts/truetype/noto/NotoSans-Medium.ttf",
                    "/usr/share/fonts/noto/NotoSans-Medium.ttf",
                    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                ],
                &[
                    "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
                    "/usr/share/fonts/noto/NotoSans-SemiBold.ttf",
                    "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
                    "/usr/share/fonts/noto/NotoSans-Bold.ttf",
                ],
            ),
            (
                "UbuntuSans",
                &["/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf"],
                &["/usr/share/fonts/truetype/ubuntu/Ubuntu-M.ttf"],
                &["/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf"],
            ),
            (
                "DejaVu",
                &["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf"],
                &["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf"],
                &["/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf"],
            ),
        ];

        let mut loaded_primary = false;
        for (name, reg_paths, med_paths, semi_paths) in matched_sets {
            if let (Some(reg), Some(med), Some(semi)) =
                (read_font_file(reg_paths), read_font_file(med_paths), read_font_file(semi_paths))
            {
                let reg_key = format!("{name}-Regular");
                let med_key = format!("{name}-Medium");
                let semi_key = format!("{name}-SemiBold");
                fonts.font_data.insert(reg_key.clone(), Arc::new(reg));
                fonts.font_data.insert(med_key.clone(), Arc::new(med));
                fonts.font_data.insert(semi_key.clone(), Arc::new(semi));

                fonts.families.entry(FontFamily::Proportional).or_default().insert(0, reg_key.clone());

                let prop_fallback = fonts.families[&FontFamily::Proportional].clone();
                let mut med_stack = vec![med_key];
                med_stack.extend(prop_fallback.iter().cloned());
                fonts.families.insert(FontFamily::Name("medium".into()), med_stack);

                let mut semi_stack = vec![semi_key];
                semi_stack.extend(prop_fallback);
                fonts.families.insert(FontFamily::Name("semibold".into()), semi_stack);

                loaded_primary = true;
                break;
            }
        }

        // Broad Unicode glyph fallback (NotoSans) if primary wasn't already NotoSans.
        if loaded_primary && !fonts.font_data.contains_key("NotoSans-Regular") {
            if let Some(noto) = read_font_file(&[
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            ]) {
                fonts.font_data.insert("NotoSans-Fallback".to_owned(), Arc::new(noto));
                for family in [FontFamily::Proportional, FontFamily::Name("medium".into()), FontFamily::Name("semibold".into())] {
                    fonts.families.entry(family).or_default().insert(1, "NotoSans-Fallback".to_owned());
                }
            }
        }

        // Crisp monospace font for paths, language codes, and SRT timestamps.
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
    }

    // Ensure "medium" and "semibold" font families always exist even without system fonts.
    let prop_stack = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    fonts.families.entry(FontFamily::Name("medium".into())).or_insert_with(|| prop_stack.clone());
    fonts.families.entry(FontFamily::Name("semibold".into())).or_insert(prop_stack);

    fonts
}

pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}
