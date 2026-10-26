use egui::{Color32, CornerRadius, Stroke, Visuals};

use super::fonts::{medium, mono, regular, semibold};
use super::tokens::{ThemeKind, Tokens, THEME_DATA_ID};

/// Applies the complete PdfCraft-grade theme to `ctx`, synchronizing active visuals and both
/// `Theme::Light` and `Theme::Dark` style slots so Light and Dark modes render with crisp contrast.
pub fn apply(ctx: &egui::Context, kind: ThemeKind) {
    ctx.set_theme(egui::ThemePreference::System);
    let t = Tokens::for_kind(kind);
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique(THEME_DATA_ID), t));

    let mut v = if t.dark() { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.panel;
    v.window_fill = t.card;
    v.window_stroke = Stroke::new(1.0, t.border);
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.hover;
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent_text;
    v.override_text_color = Some(t.text);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_shadow =
        egui::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(if t.dark() { 110 } else { 38 }) };
    v.popup_shadow =
        egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if t.dark() { 90 } else { 30 }) };

    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(t.radius);
        w.fg_stroke.color = t.text;
    }

    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = t.field;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_fill = t.hover;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = t.pressed;
    v.widgets.active.bg_fill = t.pressed;
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.widgets.open.weak_bg_fill = t.hover;

    // Whole-pixel glyph placement + Light-mode gamma darkening so dark text on white panels is crisp.
    v.text_options.subpixel_binning = false;
    if !t.dark() {
        v.text_options.color_transfer_function = egui::epaint::FontColorTransferFunction::Gamma(0.75);
    }

    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.scroll.bar_width = 8.0;
        s.spacing.scroll.floating = true;
        s.spacing.scroll.floating_width = 3.0;
        s.spacing.scroll.foreground_color = true;
        s.text_styles.insert(egui::TextStyle::Body, regular(13.0));
        s.text_styles.insert(egui::TextStyle::Button, medium(13.0));
        s.text_styles.insert(egui::TextStyle::Small, regular(11.0));
        s.text_styles.insert(egui::TextStyle::Monospace, mono(12.0));
        s.text_styles.insert(egui::TextStyle::Heading, semibold(17.0));
        s.interaction.tooltip_delay = 0.35;
    });

    let style = ctx.global_style();
    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// WCAG 2 contrast ratio between two opaque colors.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lum = |c: Color32| {
        let lin = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    };
    let (x, y) = (lum(a) + 0.05, lum(b) + 0.05);
    x.max(y) / x.min(y)
}

#[cfg(test)]
mod tests {
    use super::super::fonts::install_fonts;
    use super::super::tokens::ThemePreference;
    use super::*;

    #[test]
    fn text_is_wcag_aa_readable_on_every_surface_in_light_and_dark() {
        for kind in [ThemeKind::Light, ThemeKind::Dark] {
            let t = Tokens::for_kind(kind);
            for (name, fg) in [("text", t.text), ("text_muted", t.text_muted), ("text_faint", t.text_faint)] {
                for bg in [t.chrome, t.panel, t.card, t.pasteboard] {
                    let r = contrast(fg, bg);
                    assert!(r >= 4.5, "{kind:?} {name} on {bg:?}: {r:.2}:1, WCAG AA needs >= 4.5:1");
                }
            }
        }
    }

    #[test]
    fn apply_synchronizes_visuals_in_both_themes() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);

        apply(&ctx, ThemeKind::Light);
        assert!(!ctx.global_style().visuals.dark_mode);
        assert_eq!(Tokens::get(&ctx).kind, ThemeKind::Light);
        assert_eq!(ctx.global_style().visuals.panel_fill, Tokens::for_kind(ThemeKind::Light).panel);

        apply(&ctx, ThemeKind::Dark);
        assert!(ctx.global_style().visuals.dark_mode);
        assert_eq!(Tokens::get(&ctx).kind, ThemeKind::Dark);
        assert_eq!(ctx.global_style().visuals.panel_fill, Tokens::for_kind(ThemeKind::Dark).panel);
    }

    #[test]
    fn theme_preference_resolves_system_and_explicit_choices() {
        assert_eq!(ThemePreference::System.resolve(Some(true), ThemeKind::Light), ThemeKind::Dark);
        assert_eq!(ThemePreference::System.resolve(Some(false), ThemeKind::Dark), ThemeKind::Light);
        assert_eq!(ThemePreference::System.resolve(None, ThemeKind::Light), ThemeKind::Light);
        assert_eq!(ThemePreference::Light.resolve(Some(true), ThemeKind::Dark), ThemeKind::Light);
        assert_eq!(ThemePreference::Dark.resolve(Some(false), ThemeKind::Light), ThemeKind::Dark);
    }
}
