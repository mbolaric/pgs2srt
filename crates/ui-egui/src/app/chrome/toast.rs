use super::super::*;

/// PdfCraft-grade floating toast notification component.
#[derive(Debug, Clone, Copy)]
pub struct ToastView<'a> {
    pub message: &'a str,
    pub is_error: bool,
}

impl<'a> ToastView<'a> {
    pub fn new(message: &'a str, is_error: bool) -> Self {
        Self { message, is_error }
    }

    pub fn show(&self, ctx: &egui::Context) {
        let t = Tokens::get(ctx);
        let screen = ctx.content_rect();
        let (fill, fg, border) = if self.is_error {
            (t.error_soft, t.error, t.error)
        } else if t.dark() {
            (Color32::from_rgb(0xEC, 0xEC, 0xEF), Color32::from_rgb(0x22, 0x22, 0x26), t.border)
        } else {
            (Color32::from_rgb(0x26, 0x26, 0x2B), Color32::WHITE, Color32::TRANSPARENT)
        };

        egui::Area::new(egui::Id::unique("pgs2srt_toast"))
            .order(egui::Order::Tooltip)
            .pivot(Align2::CENTER_BOTTOM)
            .fixed_pos(screen.center_bottom() - vec2(0.0, 38.0))
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(fill)
                    .stroke(Stroke::new(1.0, border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(16, 9))
                    .show(ui, |ui| {
                        ui.set_max_width((screen.width() - 64.0).clamp(120.0, 520.0));
                        ui.add(egui::Label::new(RichText::new(self.message).font(theme::medium(12.5)).color(fg)).wrap());
                    });
            });
    }
}

impl PgsApp {
    pub fn notify(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some((message.into(), is_error, 0.0));
    }

    pub fn ui_toast(&mut self, ctx: &egui::Context) {
        let Some((msg, is_err, start)) = self.toast.clone() else {
            return;
        };
        let now = ctx.input(|i| i.time);
        let start = if start == 0.0 { now } else { start };
        self.toast = Some((msg.clone(), is_err, start));
        if now - start > 3.4 {
            self.toast = None;
            return;
        }

        ToastView::new(&msg, is_err).show(ctx);
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
