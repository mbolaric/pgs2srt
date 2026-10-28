use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

/// Names of the families for the bundled weights besides Regular, which is `FontFamily::Proportional`.
const MEDIUM: &str = "medium";
const SEMIBOLD: &str = "semibold";

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// The bundled fonts.
fn bundled_fonts() -> [(FontFamily, &'static str, &'static [u8]); 4] {
    [
        (FontFamily::Proportional, "Inter-Regular", include_bytes!("../../assets/fonts/inter/Inter-Regular.ttf")),
        (FontFamily::Name(MEDIUM.into()), "Inter-Medium", include_bytes!("../../assets/fonts/inter/Inter-Medium.ttf")),
        (FontFamily::Name(SEMIBOLD.into()), "Inter-SemiBold", include_bytes!("../../assets/fonts/inter/Inter-SemiBold.ttf")),
        (
            FontFamily::Monospace,
            "JetBrainsMonoNL-Regular",
            include_bytes!("../../assets/fonts/jetbrains-mono/JetBrainsMonoNL-Regular.ttf"),
        ),
    ]
}

/// Builds font definitions from the bundled fonts, with egui's built-in fonts of the same kind behind each.
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    let defaults = fonts.families.clone();
    for (family, name, bytes) in bundled_fonts() {
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
        let kind = if family == FontFamily::Monospace { FontFamily::Monospace } else { FontFamily::Proportional };
        let mut stack = vec![name.to_owned()];
        stack.extend(defaults[&kind].iter().cloned());
        fonts.families.insert(family, stack);
    }

    fonts
}

pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_font_helper_starts_with_its_bundled_face() {
        let fonts = font_definitions();
        for (id, face) in [
            (regular(13.0), "Inter-Regular"),
            (medium(13.0), "Inter-Medium"),
            (semibold(13.0), "Inter-SemiBold"),
            (mono(11.0), "JetBrainsMonoNL-Regular"),
        ] {
            let stack = fonts.families.get(&id.family).unwrap_or_else(|| panic!("{:?} is not registered", id.family));
            assert_eq!(stack.first().map(String::as_str), Some(face), "{:?}", id.family);
            assert!(fonts.font_data.contains_key(face));
        }
    }
}
