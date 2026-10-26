//! Pure geometric vector icons drawn on a 24x24 Lucide coordinate grid via `egui::Painter`.
//! Never uses Unicode emojis as icons, ensuring uniform baseline metrics and crisp rendering
//! at every DPI scale across Linux, Windows, macOS, and Web.

use std::f32::consts::TAU;

use egui::{pos2, vec2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind};

use crate::theme::{self, Tokens};

pub const ALL_ICONS: &[&str] = &[
    "logo",
    "plus",
    "minus",
    "folder",
    "folder-open",
    "file-sub",
    "play",
    "stop",
    "check",
    "x",
    "trash",
    "globe",
    "download",
    "search",
    "sun",
    "moon",
    "monitor",
    "panel-right",
    "sparkles",
    "chevron-down",
    "chevron-right",
    "refresh",
    "alert",
    "eye",
    "maximize",
    "restore",
];

pub fn exists(name: &str) -> bool {
    ALL_ICONS.contains(&name)
}

fn arc_points(center: Pos2, rx: f32, ry: f32, start_rad: f32, end_rad: f32, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            let t = start_rad + (end_rad - start_rad) * (i as f32 / steps as f32);
            pos2(center.x + rx * t.cos(), center.y + ry * t.sin())
        })
        .collect()
}

struct IconCtx<'a> {
    p: &'a egui::Painter,
    box_rect: Rect,
    scale: f32,
    stroke: Stroke,
    tint: Color32,
}

impl<'a> IconCtx<'a> {
    #[inline]
    fn map(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.box_rect.min.x + x * self.scale, self.box_rect.min.y + y * self.scale)
    }
}

/// Paints icon `name` centered inside `rect` at logical `size` points with `tint`.
pub fn paint(ui: &egui::Ui, rect: Rect, name: &str, size: f32, tint: Color32) {
    let box_rect = Rect::from_center_size(rect.center(), vec2(size, size));
    let scale = size / 24.0;
    let sw = (1.85 * scale).clamp(1.2, 2.25);
    let stroke = Stroke::new(sw, tint);
    let c = IconCtx { p: ui.painter(), box_rect, scale, stroke, tint };

    if paint_window_icons(&c, name)
        || paint_navigation_icons(&c, name)
        || paint_media_icons(&c, name)
        || paint_system_icons(&c, name)
    {
        return;
    }

    let fallback = Rect::from_min_max(c.map(5.0, 5.0), c.map(19.0, 19.0));
    c.p.rect_stroke(fallback, CornerRadius::same((2.0 * scale).round() as u8), stroke, StrokeKind::Inside);
}

fn paint_window_icons(c: &IconCtx, name: &str) -> bool {
    match name {
        "x" => {
            c.p.line_segment([c.map(6.2, 6.2), c.map(17.8, 17.8)], c.stroke);
            c.p.line_segment([c.map(17.8, 6.2), c.map(6.2, 17.8)], c.stroke);
            true
        }
        "minus" => {
            c.p.line_segment([c.map(5.5, 12.0), c.map(18.5, 12.0)], c.stroke);
            true
        }
        "maximize" => {
            let r = Rect::from_min_max(c.map(5.5, 5.5), c.map(18.5, 18.5));
            c.p.rect_stroke(r, CornerRadius::same((1.5 * c.scale).round() as u8), c.stroke, StrokeKind::Inside);
            true
        }
        "restore" => {
            let back = vec![c.map(8.5, 5.5), c.map(18.5, 5.5), c.map(18.5, 15.5), c.map(15.5, 15.5)];
            c.p.add(egui::Shape::line(back, c.stroke));
            c.p.line_segment([c.map(8.5, 5.5), c.map(8.5, 8.5)], c.stroke);
            let front = Rect::from_min_max(c.map(5.5, 8.5), c.map(15.5, 18.5));
            c.p.rect_stroke(front, CornerRadius::same((1.5 * c.scale).round() as u8), c.stroke, StrokeKind::Inside);
            true
        }
        _ => false,
    }
}

fn paint_navigation_icons(c: &IconCtx, name: &str) -> bool {
    match name {
        "chevron-down" => {
            c.p.add(egui::Shape::line(vec![c.map(6.5, 9.5), c.map(12.0, 15.0), c.map(17.5, 9.5)], c.stroke));
            true
        }
        "chevron-right" => {
            c.p.add(egui::Shape::line(vec![c.map(9.5, 6.5), c.map(15.0, 12.0), c.map(9.5, 17.5)], c.stroke));
            true
        }
        "panel-right" => {
            let win = Rect::from_min_max(c.map(3.0, 4.0), c.map(21.0, 20.0));
            c.p.rect_stroke(win, CornerRadius::same((2.5 * c.scale).round() as u8), c.stroke, StrokeKind::Inside);
            c.p.line_segment([c.map(14.8, 4.2), c.map(14.8, 19.8)], c.stroke);
            true
        }
        _ => false,
    }
}

fn paint_media_icons(c: &IconCtx, name: &str) -> bool {
    match name {
        "logo" => {
            let outer = Rect::from_min_max(c.map(2.5, 4.0), c.map(21.5, 20.0));
            c.p.rect_stroke(outer, CornerRadius::same((3.5 * c.scale).round() as u8), c.stroke, StrokeKind::Inside);
            c.p.line_segment([c.map(6.5, 13.5), c.map(17.5, 13.5)], c.stroke);
            c.p.line_segment([c.map(8.5, 16.8), c.map(15.5, 16.8)], c.stroke);
            c.p.circle_filled(c.map(6.5, 7.8), 1.1 * c.scale, c.tint);
            true
        }
        "plus" => {
            c.p.line_segment([c.map(12.0, 5.0), c.map(12.0, 19.0)], c.stroke);
            c.p.line_segment([c.map(5.0, 12.0), c.map(19.0, 12.0)], c.stroke);
            true
        }
        "folder" => {
            let pts = vec![
                c.map(3.0, 6.5),
                c.map(9.2, 6.5),
                c.map(11.4, 9.0),
                c.map(21.0, 9.0),
                c.map(21.0, 19.0),
                c.map(3.0, 19.0),
                c.map(3.0, 6.5),
            ];
            c.p.add(egui::Shape::line(pts, c.stroke));
            true
        }
        "folder-open" => {
            let back =
                vec![c.map(3.0, 18.5), c.map(3.0, 6.5), c.map(9.0, 6.5), c.map(11.2, 8.8), c.map(19.5, 8.8), c.map(19.5, 11.5)];
            c.p.add(egui::Shape::line(back, c.stroke));
            let flap = vec![c.map(3.0, 18.5), c.map(5.6, 11.8), c.map(21.4, 11.8), c.map(18.8, 18.5), c.map(3.0, 18.5)];
            c.p.add(egui::Shape::line(flap, c.stroke));
            true
        }
        "file-sub" => {
            let doc =
                vec![c.map(5.0, 3.0), c.map(14.5, 3.0), c.map(19.0, 7.5), c.map(19.0, 21.0), c.map(5.0, 21.0), c.map(5.0, 3.0)];
            c.p.add(egui::Shape::line(doc, c.stroke));
            c.p.add(egui::Shape::line(vec![c.map(14.2, 3.2), c.map(14.2, 7.8), c.map(18.8, 7.8)], c.stroke));
            c.p.line_segment([c.map(8.0, 13.5), c.map(16.0, 13.5)], c.stroke);
            c.p.line_segment([c.map(8.0, 17.0), c.map(13.8, 17.0)], c.stroke);
            true
        }
        "play" => {
            let tri = vec![c.map(7.0, 5.2), c.map(19.0, 12.0), c.map(7.0, 18.8)];
            c.p.add(egui::Shape::convex_polygon(tri, c.tint, Stroke::new(0.8 * c.scale, c.tint)));
            true
        }
        "stop" => {
            let r = Rect::from_min_max(c.map(6.0, 6.0), c.map(18.0, 18.0));
            c.p.rect_filled(r, CornerRadius::same((2.0 * c.scale).round() as u8), c.tint);
            true
        }
        "trash" => {
            c.p.line_segment([c.map(4.0, 6.5), c.map(20.0, 6.5)], c.stroke);
            c.p.add(egui::Shape::line(vec![c.map(9.0, 6.5), c.map(9.0, 4.0), c.map(15.0, 4.0), c.map(15.0, 6.5)], c.stroke));
            c.p.add(egui::Shape::line(vec![c.map(6.2, 6.5), c.map(7.2, 20.0), c.map(16.8, 20.0), c.map(17.8, 6.5)], c.stroke));
            c.p.line_segment([c.map(10.3, 10.5), c.map(10.3, 16.5)], c.stroke);
            c.p.line_segment([c.map(13.7, 10.5), c.map(13.7, 16.5)], c.stroke);
            true
        }
        _ => false,
    }
}

fn paint_system_icons(c: &IconCtx, name: &str) -> bool {
    match name {
        "check" => {
            c.p.add(egui::Shape::line(vec![c.map(4.8, 12.6), c.map(9.6, 17.4), c.map(19.4, 6.8)], c.stroke));
            true
        }
        "globe" => {
            let center = c.map(12.0, 12.0);
            let r = 8.8 * c.scale;
            c.p.circle_stroke(center, r, c.stroke);
            c.p.line_segment([c.map(3.4, 12.0), c.map(20.6, 12.0)], c.stroke);
            let meridian_r = arc_points(center, 4.2 * c.scale, r, -std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2, 14);
            let meridian_l =
                arc_points(center, 4.2 * c.scale, r, std::f32::consts::FRAC_PI_2, 3.0 * std::f32::consts::FRAC_PI_2, 14);
            c.p.add(egui::Shape::line(meridian_r, c.stroke));
            c.p.add(egui::Shape::line(meridian_l, c.stroke));
            true
        }
        "download" => {
            c.p.line_segment([c.map(12.0, 4.0), c.map(12.0, 15.2)], c.stroke);
            c.p.add(egui::Shape::line(vec![c.map(7.6, 11.0), c.map(12.0, 15.4), c.map(16.4, 11.0)], c.stroke));
            c.p.add(egui::Shape::line(vec![c.map(4.5, 15.5), c.map(4.5, 19.5), c.map(19.5, 19.5), c.map(19.5, 15.5)], c.stroke));
            true
        }
        "search" => {
            c.p.circle_stroke(c.map(10.8, 10.8), 6.2 * c.scale, c.stroke);
            c.p.line_segment([c.map(15.4, 15.4), c.map(20.0, 20.0)], c.stroke);
            true
        }
        "sun" => {
            let center = c.map(12.0, 12.0);
            c.p.circle_stroke(center, 4.2 * c.scale, c.stroke);
            for i in 0..8 {
                let a = (i as f32 / 8.0) * TAU;
                let p0 = pos2(center.x + (6.6 * c.scale) * a.cos(), center.y + (6.6 * c.scale) * a.sin());
                let p1 = pos2(center.x + (9.0 * c.scale) * a.cos(), center.y + (9.0 * c.scale) * a.sin());
                c.p.line_segment([p0, p1], c.stroke);
            }
            true
        }
        "moon" => {
            let outer = arc_points(c.map(12.2, 12.0), 7.8 * c.scale, 7.8 * c.scale, 0.42, 4.35, 18);
            let inner = arc_points(c.map(15.2, 9.0), 6.2 * c.scale, 6.2 * c.scale, 3.92, 1.05, 14);
            let mut pts = outer;
            pts.extend(inner);
            if let Some(&first) = pts.first() {
                pts.push(first);
            }
            c.p.add(egui::Shape::line(pts, c.stroke));
            true
        }
        "monitor" => {
            let scr = Rect::from_min_max(c.map(3.0, 4.0), c.map(21.0, 16.2));
            c.p.rect_stroke(scr, CornerRadius::same((2.0 * c.scale).round() as u8), c.stroke, StrokeKind::Inside);
            c.p.line_segment([c.map(12.0, 16.2), c.map(12.0, 20.0)], c.stroke);
            c.p.line_segment([c.map(8.0, 20.0), c.map(16.0, 20.0)], c.stroke);
            true
        }
        "sparkles" => {
            let star = vec![
                c.map(10.5, 3.5),
                c.map(12.2, 8.3),
                c.map(17.0, 10.0),
                c.map(12.2, 11.7),
                c.map(10.5, 16.5),
                c.map(8.8, 11.7),
                c.map(4.0, 10.0),
                c.map(8.8, 8.3),
                c.map(10.5, 3.5),
            ];
            c.p.add(egui::Shape::line(star, c.stroke));
            c.p.line_segment([c.map(18.5, 14.5), c.map(18.5, 20.5)], c.stroke);
            c.p.line_segment([c.map(15.5, 17.5), c.map(21.5, 17.5)], c.stroke);
            true
        }
        "refresh" => {
            let center = c.map(12.0, 12.0);
            let arc = arc_points(center, 7.6 * c.scale, 7.6 * c.scale, -0.5, 4.9, 22);
            c.p.add(egui::Shape::line(arc, c.stroke));
            c.p.add(egui::Shape::line(vec![c.map(19.5, 4.5), c.map(19.5, 9.2), c.map(14.8, 9.2)], c.stroke));
            true
        }
        "alert" => {
            c.p.circle_stroke(c.map(12.0, 12.0), 8.6 * c.scale, c.stroke);
            c.p.line_segment([c.map(12.0, 7.8), c.map(12.0, 13.0)], c.stroke);
            c.p.circle_filled(c.map(12.0, 16.4), 1.15 * c.scale, c.tint);
            true
        }
        "eye" => {
            let top = arc_points(c.map(12.0, 16.2), 9.4 * c.scale, 7.8 * c.scale, -2.55, -0.59, 16);
            let bot = arc_points(c.map(12.0, 7.8), 9.4 * c.scale, 7.8 * c.scale, 0.59, 2.55, 16);
            c.p.add(egui::Shape::line(top, c.stroke));
            c.p.add(egui::Shape::line(bot, c.stroke));
            c.p.circle_stroke(c.map(12.0, 12.0), 2.6 * c.scale, c.stroke);
            true
        }
        _ => false,
    }
}

/// Square icon button: transparent until hovered; `selected` gets PdfCraft's soft accent wash and border.
pub fn button(ui: &mut egui::Ui, name: &str, box_size: f32, selected: bool, tooltip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(box_size, box_size), Sense::click());
    let label = if tooltip.is_empty() { name } else { tooltip };
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), selected, label));

    let press = theme::Press::track(ui, &resp);
    let fill = press.wash_color(&t, selected);
    if fill.a() > 0 {
        ui.painter().rect_filled(rect, CornerRadius::same(t.radius), fill);
    }
    if selected {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            CornerRadius::same(t.radius),
            Stroke::new(1.0, t.accent.gamma_multiply(0.45)),
            StrokeKind::Inside,
        );
    }

    let tint = if !ui.is_enabled() {
        t.text_faint.gamma_multiply(0.6)
    } else if selected {
        t.accent_text
    } else {
        t.icon
    };
    let icon_size = (box_size * 0.56).round() * (1.0 - 0.06 * press.down);
    paint(ui, rect.translate(press.offset()), name, icon_size, tint);

    let resp = theme::hand(resp);
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_icon_paints_shapes_in_both_themes() {
        for kind in [theme::ThemeKind::Light, theme::ThemeKind::Dark] {
            let ctx = egui::Context::default();
            theme::apply(&ctx, kind);
            for &icon in ALL_ICONS {
                assert!(exists(icon), "icon {icon} must exist");
                let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    button(ui, icon, 28.0, false, icon);
                });
                assert!(!out.shapes.is_empty(), "icon {icon} produced no shapes in {kind:?}");
                out.drop_without_applying_deltas();
            }
        }
    }
}
