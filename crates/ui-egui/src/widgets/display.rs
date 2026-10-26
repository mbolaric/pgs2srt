use egui::{pos2, vec2, Align2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke};

use crate::theme::{self, Tokens};

/// Soft hover elevation shadow under interactive cards.
pub fn card_lift(ui: &egui::Ui, t: &Tokens, rect: Rect, press: theme::Press) {
    let lift = press.hover * (1.0 - press.down);
    if lift > 0.04 {
        let shadow = egui::Shadow {
            offset: [0, 3],
            blur: (14.0 * lift) as u8,
            spread: 0,
            color: t.page_shadow.gamma_multiply(0.85 * lift),
        };
        ui.painter().add(shadow.as_shape(rect, CornerRadius::same(8)));
    }
}

/// Subtle vertical divider inside horizontal bars.
pub fn vline_divider(ui: &mut egui::Ui, bar_height: f32) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, bar_height), Sense::hover());
    ui.painter().vline(rect.center().x, rect.y_range().shrink(6.0), Stroke::new(1.0, t.divider));
}

/// Uppercase section header in side panels and settings cards.
pub fn section_title(ui: &mut egui::Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text.to_uppercase()).font(theme::semibold(10.5)).color(t.text_faint).extra_letter_spacing(0.6));
    ui.add_space(2.0);
}

/// Paints one line of `text` left-aligned and vertically centered at `pos`, truncated with `"…"`
/// so it never exceeds `max_width`. Returns `true` if `text` was truncated.
pub fn row_text(ui: &egui::Ui, pos: Pos2, text: impl Into<String>, font: egui::FontId, color: Color32, max_width: f32) -> bool {
    if max_width <= 2.0 {
        return true;
    }
    let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    let galley = ui.painter().layout_job(job);
    let elided = galley.elided;
    let rect = Align2::LEFT_CENTER.anchor_size(pos, galley.size());
    ui.painter().galley(rect.min, galley, color);
    elided
}

/// Paints a right-aligned pill badge centered vertically at `right_center` and returns its [`Rect`].
pub fn paint_badge_right(ui: &egui::Ui, right_center: Pos2, label: &str, fg: Color32, bg: Color32) -> Rect {
    let font = theme::semibold(11.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, fg);
    let w = galley.size().x + 16.0;
    let h = 22.0;
    let rect = Rect::from_min_size(pos2(right_center.x - w, right_center.y - h * 0.5), vec2(w, h));
    ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
    ui.painter().galley(pos2(rect.center().x - galley.size().x * 0.5, rect.center().y - galley.size().y * 0.5), galley, fg);
    rect
}

/// Inline status/tag badge widget.
pub fn badge(ui: &mut egui::Ui, label: &str, fg: Color32, bg: Color32) -> Response {
    let font = theme::semibold(11.0);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), fg).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 16.0, 22.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, fg);
    resp
}
