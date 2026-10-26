use egui::{Color32, CornerRadius, Rect, Response, Vec2};

use super::tokens::Tokens;

/// Hover and press amounts for chrome controls (eased wash + 1px physical press settle).
#[derive(Clone, Copy, Debug)]
pub struct Press {
    pub hover: f32,
    pub down: f32,
}

impl Press {
    pub fn track(ui: &egui::Ui, resp: &Response) -> Self {
        let hot = resp.hovered() || resp.is_pointer_button_down_on();
        let ease = egui::emath::easing::quadratic_out;
        Self {
            hover: ui.ctx().animate_bool_with_time_and_easing(resp.id.with("chrome-hover"), hot, 0.12, ease),
            down: ui.ctx().animate_bool_with_time_and_easing(
                resp.id.with("chrome-down"),
                resp.is_pointer_button_down_on(),
                0.06,
                ease,
            ),
        }
    }

    pub fn fill(self, rest: Color32, hover: Color32, pressed: Color32) -> Color32 {
        rest.lerp_to_gamma(hover, self.hover).lerp_to_gamma(pressed, self.down)
    }

    pub fn offset(self) -> Vec2 {
        Vec2::new(0.0, self.down)
    }

    pub fn wash_color(self, t: &Tokens, selected: bool) -> Color32 {
        if selected {
            self.fill(t.accent_soft, t.accent_soft.lerp_to_gamma(t.accent, 0.16), t.selected)
        } else {
            self.fill(Color32::TRANSPARENT, t.hover, t.pressed)
        }
    }

    pub fn wash(self, ui: &egui::Ui, rect: Rect, radius: impl Into<CornerRadius>, selected: bool) {
        let fill = self.wash_color(&Tokens::get(ui.ctx()), selected);
        if fill.a() > 0 {
            ui.painter().rect_filled(rect, radius, fill);
        }
    }
}

/// Pointing hand cursor for interactive controls.
pub fn hand(resp: Response) -> Response {
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
