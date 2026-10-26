use std::hash::Hash;

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind};

use crate::icons;
use crate::theme::{self, Tokens};

/// Top strip / mode-bar tab with icon, label, optional badge count, and active bottom accent bar,
/// styled after PdfCraft's document & workspace tabs.
pub fn mode_tab(ui: &mut egui::Ui, icon: &str, label: &str, count: Option<usize>, active: bool, strip_height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = if active { theme::semibold(13.0) } else { theme::medium(13.0) };
    let label_w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);

    let badge_str = count.map(|n| n.to_string());
    let badge_w = badge_str.as_ref().map_or(0.0, |s| {
        ui.fonts_mut(|f| f.layout_no_wrap(s.clone(), theme::medium(11.0), t.text).size().x).max(12.0) + 12.0 + 6.0
    });

    let total_w = 12.0 + 16.0 + 7.0 + label_w + badge_w + 12.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(total_w, strip_height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), active, label));

    let press = theme::Press::track(ui, &resp);
    if active {
        let top_tab = Rect::from_min_max(rect.min + vec2(0.0, 3.0), rect.max + vec2(0.0, 1.0));
        ui.painter().rect(
            top_tab,
            CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
            t.chrome,
            Stroke::new(1.0, t.border),
            StrokeKind::Inside,
        );
        // Overwrite the bottom stroke with t.chrome so the active tab seamlessly merges into the toolbar below
        ui.painter().hline(top_tab.left() + 1.0..=top_tab.right() - 1.0, rect.bottom(), Stroke::new(1.5, t.chrome));
        // Clean accent indicator bar along the top edge of the tab
        let bar = Rect::from_min_max(top_tab.left_top() + vec2(4.0, 0.0), top_tab.right_top() + vec2(-4.0, 2.0));
        ui.painter().rect_filled(bar, CornerRadius::same(1), t.accent);
    } else {
        let well = rect.shrink2(vec2(2.0, 4.0));
        press.wash(ui, well, 4, false);
    }

    let body = rect.translate(press.offset());
    let fg = if active { t.text } else { t.text_muted };
    let icon_tint = if active { t.accent } else { t.text_muted };
    icons::paint(
        ui,
        Rect::from_center_size(pos2(body.left() + 18.0, body.center().y + 1.0), vec2(15.0, 15.0)),
        icon,
        14.5,
        icon_tint,
    );
    ui.painter().text(pos2(body.left() + 32.0, body.center().y + 1.0), Align2::LEFT_CENTER, label, font, fg);

    if let Some(b_text) = badge_str {
        let b_font = theme::medium(11.0);
        let tw = ui.fonts_mut(|f| f.layout_no_wrap(b_text.clone(), b_font.clone(), t.text).size().x).max(12.0);
        let bw = tw + 10.0;
        let b_rect = Rect::from_center_size(pos2(body.right() - 10.0 - bw * 0.5, body.center().y + 1.0), vec2(bw, 18.0));
        let (b_fill, b_fg) = if active { (t.accent_soft, t.accent_text) } else { (t.hover, t.text_muted) };
        ui.painter().rect_filled(b_rect, CornerRadius::same(9), b_fill);
        ui.painter().text(b_rect.center(), Align2::CENTER_CENTER, b_text, b_font, b_fg);
    }

    theme::hand(resp)
}

/// Unified segmented switch (`[EN | DE]`, `[Source folder | Custom folder]`, `[All | Installed | Available]`)
/// sharing one exact `height` and crisp inner selection pill.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id_salt: impl Hash + std::fmt::Debug,
    current: &mut T,
    options: &[(T, &str)],
    height: f32,
) -> bool {
    let t = Tokens::get(ui.ctx());
    let font = theme::medium(12.0);
    let seg_widths: Vec<f32> = options
        .iter()
        .map(|(_, label)| ui.fonts_mut(|f| f.layout_no_wrap((*label).to_owned(), font.clone(), t.text).size().x) + 20.0)
        .collect();
    let total_w: f32 = seg_widths.iter().sum::<f32>() + 4.0;
    let (outer_rect, _) = ui.allocate_exact_size(vec2(total_w, height), Sense::hover());

    ui.painter().rect(outer_rect, CornerRadius::same(t.radius), t.pasteboard, Stroke::new(1.0, t.border), StrokeKind::Inside);

    let mut changed = false;
    let mut x = outer_rect.left() + 2.0;
    let base_id = ui.scope_id().with(id_salt);

    for (idx, ((val, label), &w)) in options.iter().zip(seg_widths.iter()).enumerate() {
        let seg_rect = Rect::from_min_size(pos2(x, outer_rect.top() + 2.0), vec2(w, height - 4.0));
        let resp = ui.interact(seg_rect, base_id.with(idx), Sense::click());
        let active = *current == *val;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, ui.is_enabled(), active, *label));

        let press = theme::Press::track(ui, &resp);
        if active {
            ui.painter().rect(
                seg_rect,
                CornerRadius::same(t.radius.saturating_sub(2)),
                t.card,
                Stroke::new(1.0, t.border),
                StrokeKind::Inside,
            );
        } else {
            let hover_fill = press.fill(Color32::TRANSPARENT, t.hover, t.pressed);
            if hover_fill.a() > 0 {
                ui.painter().rect_filled(seg_rect, CornerRadius::same(t.radius.saturating_sub(2)), hover_fill);
            }
        }

        let fg = if active { t.text } else { t.text_muted };
        let label_font = if active { theme::semibold(12.0) } else { font.clone() };
        ui.painter().text(seg_rect.translate(press.offset()).center(), Align2::CENTER_CENTER, *label, label_font, fg);

        if theme::hand(resp).clicked() && !active {
            *current = *val;
            changed = true;
        }
        x += w;
    }

    changed
}
