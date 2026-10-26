use super::super::*;

/// PdfCraft-grade desktop status bar view component.
#[derive(Debug, Clone, Copy)]
pub struct StatusBarView<'a> {
    pub active_workers: usize,
    pub parallel_files: usize,
    pub downloading_lang: Option<&'a str>,
    pub installed_models_count: usize,
}

impl<'a> StatusBarView<'a> {
    pub fn new(
        active_workers: usize,
        parallel_files: usize,
        downloading_lang: Option<&'a str>,
        installed_models_count: usize,
    ) -> Self {
        Self { active_workers, parallel_files, downloading_lang, installed_models_count }
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());

        egui::Panel::bottom("status_bar")
            .exact_size(STATUS_BAR_HEIGHT)
            .frame(Frame::NONE.fill(t.titlebar).inner_margin(Margin::symmetric(14, 0)))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().hline(rect.left() - 14.0..=rect.right() + 14.0, rect.top() + 0.5, Stroke::new(1.0, t.divider));

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    let (dot_color, status_text) = if let Some(code) = self.downloading_lang {
                        (t.accent, format!("{} '{code}'…", tr("status.downloading")))
                    } else if self.active_workers > 0 {
                        (t.accent, tr("status.converting").to_string())
                    } else {
                        (t.success, tr("status.ready").to_string())
                    };

                    let (dot_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.5, dot_color);
                    ui.label(RichText::new(status_text).font(theme::regular(11.5)).color(t.text_muted));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        ui.label(
                            RichText::new(format!("{}: {}", tr("status.installed_models"), self.installed_models_count))
                                .font(theme::regular(11.5))
                                .color(t.text_muted),
                        );
                        ui.label(RichText::new("·").color(t.text_faint));
                        ui.label(
                            RichText::new(format!(
                                "{}: {}/{}",
                                tr("status.active_workers"),
                                self.active_workers,
                                self.parallel_files
                            ))
                            .font(theme::regular(11.5))
                            .color(t.text_muted),
                        );
                    });
                });
            });
    }
}

impl PgsApp {
    pub fn ui_status_bar(&self, ui: &mut egui::Ui) {
        let active_workers = self.items.iter().filter(|i| matches!(i.status, ItemStatus::Converting { .. })).count();
        let view = StatusBarView::new(
            active_workers,
            self.preferences.parallel_files,
            self.downloading_lang.as_deref(),
            self.installed_langs.len(),
        );
        view.show(ui);
    }
}
