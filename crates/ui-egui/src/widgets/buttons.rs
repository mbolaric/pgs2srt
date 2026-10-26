use egui::{pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind};

use super::display::row_text;
use crate::icons;
use crate::theme::{self, Tokens};

/// Rounded pill button (`primary = true` fills with `t.accent`).
pub fn pill_button(ui: &mut egui::Ui, label: &str, primary: bool, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::medium(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 24.0, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));

    let press = theme::Press::track(ui, &resp);
    let enabled = ui.is_enabled();
    let (fill, stroke, fg) = if primary {
        if enabled {
            (press.fill(t.accent, t.accent_text, t.accent.lerp_to_gamma(Color32::BLACK, 0.22)), Stroke::NONE, Color32::WHITE)
        } else {
            (t.hover, Stroke::new(1.0, t.border), t.text_faint)
        }
    } else if enabled {
        (press.fill(t.card, t.hover, t.pressed), Stroke::new(1.0, t.border), t.text)
    } else {
        (t.card, Stroke::new(1.0, t.divider), t.text_faint)
    };

    let radius = CornerRadius::same(t.radius);
    ui.painter().rect(rect, radius, fill, stroke, StrokeKind::Inside);
    ui.painter().text(rect.translate(press.offset()).center(), Align2::CENTER_CENTER, label, font, fg);
    theme::hand(resp)
}

/// Button with a leading vector icon styled with clean 1px borders (PdfCraft button).
pub fn icon_pill(ui: &mut egui::Ui, icon: &str, label: &str, primary: bool, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::medium(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 36.0, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));

    let press = theme::Press::track(ui, &resp);
    let enabled = ui.is_enabled();
    let (fill, stroke, fg) = if primary {
        if enabled {
            (press.fill(t.accent, t.accent_text, t.accent.lerp_to_gamma(Color32::BLACK, 0.22)), Stroke::NONE, Color32::WHITE)
        } else {
            (t.hover, Stroke::new(1.0, t.border), t.text_faint)
        }
    } else if enabled {
        (press.fill(t.card, t.hover, t.pressed), Stroke::new(1.0, t.border), t.text)
    } else {
        (t.card, Stroke::new(1.0, t.divider), t.text_faint)
    };

    let radius = CornerRadius::same(t.radius);
    ui.painter().rect(rect, radius, fill, stroke, StrokeKind::Inside);
    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(pos2(body.left() + 14.0, body.center().y), vec2(15.0, 15.0)), icon, 14.0, fg);
    ui.painter().text(pos2(body.left() + 27.0, body.center().y), Align2::LEFT_CENTER, label, font, fg);
    theme::hand(resp)
}

/// Outlined destructive/stop button (PdfCraft danger button).
pub fn danger_icon_pill(ui: &mut egui::Ui, icon: &str, label: &str, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::medium(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.error).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 36.0, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));

    let press = theme::Press::track(ui, &resp);
    let fill = press.fill(t.error_soft, t.error_soft.lerp_to_gamma(t.error, 0.16), t.error_soft);
    let stroke = Stroke::new(1.0, t.error.gamma_multiply(0.55));
    let radius = CornerRadius::same(t.radius);
    ui.painter().rect(rect, radius, fill, stroke, StrokeKind::Inside);
    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(pos2(body.left() + 17.0, body.center().y), vec2(15.0, 15.0)), icon, 13.5, t.error);
    ui.painter().text(pos2(body.left() + 30.0, body.center().y), Align2::LEFT_CENTER, label, font, t.error);
    theme::hand(resp)
}

/// Icon + label button, transparent until hovered.
pub fn ghost_button(ui: &mut egui::Ui, icon: &str, label: &str, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::medium(13.0);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 36.0, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));

    let press = theme::Press::track(ui, &resp);
    press.wash(ui, rect, t.radius, false);
    let body = rect.translate(press.offset());
    let fg = if ui.is_enabled() { t.text } else { t.text_faint };
    let ic = if ui.is_enabled() { t.icon } else { t.text_faint };
    icons::paint(ui, Rect::from_center_size(pos2(body.left() + 14.0, body.center().y), vec2(16.0, 16.0)), icon, 15.5, ic);
    ui.painter().text(pos2(body.left() + 27.0, body.center().y), Align2::LEFT_CENTER, label, font, fg);
    theme::hand(resp)
}

/// Outlined dropdown button (`t.card`, optional leading vector icon, ellipsized label, trailing `chevron-down`).
/// Pass the returned [`Response`] to `egui::Popup::menu(&resp)`.
pub fn dropdown_button(ui: &mut egui::Ui, icon: Option<&str>, label: &str, width: f32, height: f32, tooltip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, ui.is_enabled(), label));

    let press = theme::Press::track(ui, &resp);
    let enabled = ui.is_enabled();
    let fill = if enabled { press.fill(t.card, t.hover, t.pressed) } else { t.pasteboard };
    let stroke = Stroke::new(1.0, if resp.hovered() && enabled { t.text_faint } else { t.border });
    ui.painter().rect(rect, CornerRadius::same(t.radius), fill, stroke, StrokeKind::Inside);

    let body = rect.translate(press.offset());
    let fg = if enabled { t.text } else { t.text_faint };
    let text_left = if let Some(ic) = icon {
        icons::paint(
            ui,
            Rect::from_center_size(pos2(body.left() + 14.0, body.center().y), vec2(15.0, 15.0)),
            ic,
            14.0,
            if enabled { t.icon } else { t.text_faint },
        );
        body.left() + 26.0
    } else {
        body.left() + 10.0
    };

    let arrow_x = body.right() - 13.0;
    icons::paint(
        ui,
        Rect::from_center_size(pos2(arrow_x, body.center().y), vec2(14.0, 14.0)),
        "chevron-down",
        13.0,
        if enabled { t.text_muted } else { t.text_faint },
    );

    let max_text_w = (arrow_x - 10.0 - text_left).max(0.0);
    let elided = row_text(ui, pos2(text_left, body.center().y), label, theme::medium(12.5), fg, max_text_w);

    let resp = theme::hand(resp);
    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip)
    } else if elided {
        resp.on_hover_text(label)
    } else {
        resp
    }
}
