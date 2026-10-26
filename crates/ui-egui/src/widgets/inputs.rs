use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::{pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Rect, Response, Sense, Stroke, StrokeKind};

use super::CHECKBOX_RADIUS;
use crate::icons;
use crate::theme::{self, Tokens};

/// Unified numeric stepper (`[-  4  +]`) in a single exact-height container.
pub fn stepper(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    value: &mut usize,
    range: RangeInclusive<usize>,
    height: f32,
    tooltip: &str,
) -> bool {
    let t = Tokens::get(ui.ctx());
    let btn_w = 26.0;
    let val_w = 28.0;
    let total_w = btn_w * 2.0 + val_w;
    let (rect, outer_resp) = ui.allocate_exact_size(vec2(total_w, height), Sense::hover());
    if !tooltip.is_empty() {
        outer_resp.on_hover_text(tooltip);
    }

    ui.painter().rect(rect, CornerRadius::same(t.radius), t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);

    let left_rect = Rect::from_min_size(rect.min, vec2(btn_w, height));
    let mid_rect = Rect::from_min_size(pos2(rect.left() + btn_w, rect.top()), vec2(val_w, height));
    let right_rect = Rect::from_min_size(pos2(rect.left() + btn_w + val_w, rect.top()), vec2(btn_w, height));

    let base_id = ui.scope_id().with(id_salt);
    let can_dec = ui.is_enabled() && *value > *range.start();
    let can_inc = ui.is_enabled() && *value < *range.end();

    let dec_resp = ui.interact(left_rect, base_id.with("dec"), if can_dec { Sense::click() } else { Sense::hover() });
    let inc_resp = ui.interact(right_rect, base_id.with("inc"), if can_inc { Sense::click() } else { Sense::hover() });

    let dec_press = theme::Press::track(ui, &dec_resp);
    let inc_press = theme::Press::track(ui, &inc_resp);

    if can_dec {
        let fill = dec_press.fill(Color32::TRANSPARENT, t.hover, t.pressed);
        if fill.a() > 0 {
            ui.painter().rect_filled(
                left_rect.shrink(1.0),
                CornerRadius { nw: t.radius.saturating_sub(1), sw: t.radius.saturating_sub(1), ne: 0, se: 0 },
                fill,
            );
        }
    }
    if can_inc {
        let fill = inc_press.fill(Color32::TRANSPARENT, t.hover, t.pressed);
        if fill.a() > 0 {
            ui.painter().rect_filled(
                right_rect.shrink(1.0),
                CornerRadius { nw: 0, sw: 0, ne: t.radius.saturating_sub(1), se: t.radius.saturating_sub(1) },
                fill,
            );
        }
    }

    ui.painter().vline(mid_rect.left(), rect.y_range().shrink(5.0), Stroke::new(1.0, t.divider));
    ui.painter().vline(mid_rect.right(), rect.y_range().shrink(5.0), Stroke::new(1.0, t.divider));

    icons::paint(
        ui,
        left_rect.translate(dec_press.offset()),
        "minus",
        13.0,
        if can_dec { t.icon } else { t.text_faint.gamma_multiply(0.5) },
    );
    ui.painter().text(mid_rect.center(), Align2::CENTER_CENTER, value.to_string(), theme::semibold(12.5), t.text);
    icons::paint(
        ui,
        right_rect.translate(inc_press.offset()),
        "plus",
        13.0,
        if can_inc { t.icon } else { t.text_faint.gamma_multiply(0.5) },
    );

    let mut changed = false;
    if can_dec && theme::hand(dec_resp).clicked() {
        *value -= 1;
        changed = true;
    }
    if can_inc && theme::hand(inc_resp).clicked() {
        *value += 1;
        changed = true;
    }
    changed
}

/// Outlined search text input with leading `search` icon and trailing `x` clear button.
pub fn search_input(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    query: &mut String,
    placeholder: &str,
    width: f32,
    height: f32,
) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let edit_id = ui.scope_id().with(id_salt);
    let has_focus = ui.ctx().memory(|m| m.has_focus(edit_id));

    let border_col = if has_focus { t.accent } else { t.border };
    let stroke_w = if has_focus { 1.5 } else { 1.0 };
    ui.painter().rect(rect, CornerRadius::same(t.radius), t.field, Stroke::new(stroke_w, border_col), StrokeKind::Inside);

    icons::paint(
        ui,
        Rect::from_center_size(pos2(rect.left() + 15.0, rect.center().y), vec2(15.0, 15.0)),
        "search",
        14.5,
        if has_focus { t.accent } else { t.text_muted },
    );

    let right_pad = if query.is_empty() { 8.0 } else { 28.0 };
    let inner_rect = Rect::from_min_max(
        pos2(rect.left() + 28.0, rect.top() + 2.0),
        pos2((rect.right() - right_pad).max(rect.left() + 32.0), rect.bottom() - 2.0),
    );

    let mut clear_clicked = false;
    if !query.is_empty() {
        let x_rect = Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(20.0, 20.0));
        let x_resp = ui.interact(x_rect, edit_id.with("clear"), Sense::click());
        let x_press = theme::Press::track(ui, &x_resp);
        x_press.wash(ui, x_rect, 4, false);
        icons::paint(ui, x_rect.translate(x_press.offset()), "x", 12.5, t.text_muted);
        if theme::hand(x_resp).clicked() {
            clear_clicked = true;
        }
    }

    let mut edit_resp = ui
        .scope_builder(egui::UiBuilder::new().max_rect(inner_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.add(
                egui::TextEdit::singleline(query)
                    .id(edit_id)
                    .hint_text(egui::RichText::new(placeholder).color(t.text_faint).font(theme::regular(12.5)))
                    .font(theme::regular(12.5))
                    .text_color(t.text)
                    .desired_width(inner_rect.width())
                    .frame(egui::Frame::NONE)
                    .margin(vec2(2.0, 4.0)),
            )
        })
        .inner;
    ui.advance_cursor_after_rect(rect);

    if clear_clicked {
        query.clear();
        edit_resp.mark_changed();
    }
    edit_resp
}

/// Outlined square checkbox (`CornerRadius::same(3)`), filled with `t.hover` in Light mode so it
/// never vanishes into white panels or cards (PdfCraft `widgets::checkbox`).
pub fn checkbox<'a>(checked: &'a mut bool, label: impl egui::IntoAtoms<'a>) -> impl egui::Widget + 'a {
    let widget = egui::Checkbox::new(checked, label);
    move |ui: &mut egui::Ui| {
        ui.scope(|ui| {
            let t = Tokens::get(ui.ctx());
            let w = &mut ui.visuals_mut().widgets;
            for v in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
                v.corner_radius = CornerRadius::same(CHECKBOX_RADIUS);
            }
            if !t.dark() && w.inactive.bg_fill == t.field {
                w.inactive.bg_fill = t.hover;
            }
            if w.inactive.bg_stroke.is_empty() {
                w.inactive.bg_stroke = Stroke::new(1.0, t.border);
            }
            if w.hovered.bg_stroke.is_empty() {
                w.hovered.bg_stroke = Stroke::new(1.0, t.text_muted);
            }
            ui.add(widget)
        })
        .inner
    }
}

/// Themed radio button wrapper matching PdfCraft `widgets::radio`.
pub fn radio<'a>(selected: bool, text: impl egui::IntoAtoms<'a>) -> impl egui::Widget + 'a {
    egui::RadioButton::new(selected, text)
}

/// Themed radio value button wrapper matching PdfCraft `widgets::radio_value`.
pub fn radio_value<'a, Value: PartialEq + Clone>(
    current_value: &'a mut Value,
    selected_value: Value,
    text: impl egui::IntoAtoms<'a>,
) -> impl egui::Widget + 'a {
    let checked = *current_value == selected_value;
    let widget = egui::RadioButton::new(checked, text);
    move |ui: &mut egui::Ui| {
        let resp = ui.add(widget);
        if resp.clicked() {
            *current_value = selected_value;
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkboxes_are_outlined_and_distinct_from_white_cards_in_both_themes() {
        for kind in [theme::ThemeKind::Light, theme::ThemeKind::Dark] {
            let ctx = egui::Context::default();
            theme::apply(&ctx, kind);
            let t = Tokens::for_kind(kind);
            let side = ctx.global_style().spacing.icon_width;
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.add(checkbox(&mut false, "Overwrite existing files"));
            });
            let boxes: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if (r.rect.width() - side).abs() < 0.5 && (r.rect.height() - side).abs() < 0.5 => {
                        Some(r)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(boxes.len(), 1, "{kind:?}: expected one checkbox box");
            assert!(boxes[0].fill != t.panel && boxes[0].fill != t.card, "{kind:?}: checkbox box vanished into card");
            assert!(boxes[0].stroke.width >= 1.0, "{kind:?}: checkbox box must have outline");
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn search_input_keeps_sibling_x_position_steady_when_query_is_typed() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, theme::ThemeKind::Light);

        let measure_sibling_left = |text: &str| -> f32 {
            let mut q = text.to_string();
            let mut sibling_left = 0.0;
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    search_input(ui, "test_search", &mut q, "Search...", 240.0, 30.0);
                    let r = crate::widgets::pill_button(ui, "Filter", false, 30.0);
                    sibling_left = r.rect.left();
                });
            });
            out.drop_without_applying_deltas();
            sibling_left
        };

        let left_empty = measure_sibling_left("");
        let left_typed = measure_sibling_left("hrv");
        assert!(
            (left_empty - left_typed).abs() < 0.01,
            "Sibling shifted horizontally when search query changed: empty={left_empty}, typed={left_typed}"
        );
    }

    #[test]
    fn search_input_clear_button_empties_query_and_reports_change() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, theme::ThemeKind::Light);

        let mut query = "hrv".to_string();
        let mut frame = |events: Vec<egui::Event>| {
            let mut response = None;
            let out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                response = Some(search_input(ui, "test_search", &mut query, "Search...", 240.0, 30.0));
            });
            out.drop_without_applying_deltas();
            response.expect("search input was shown")
        };

        let edit = frame(Vec::new()).rect;
        let x_button = pos2(edit.right() + 14.0, edit.center().y);
        let button = |pressed| egui::Event::PointerButton {
            pos: x_button,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frame(vec![egui::Event::PointerMoved(x_button), button(true)]);
        let released = frame(vec![button(false)]);

        assert!(released.changed(), "clearing must report a change so callers apply the empty filter");
        assert!(query.is_empty());
    }
}
