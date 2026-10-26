use super::*;

impl eframe::App for PgsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.sync_system_theme(&ctx);
        self.handle_incoming_messages();
        self.handle_batch_concurrency();
        self.handle_queue_keyboard(&ctx);

        // Handle Drag & Drop of .sup files or folders containing .sup files
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            self.platform.take_dropped_files(dropped, self.mailbox());
        }

        // Render PdfCraft-grade 3-tier desktop chrome + workspace
        self.ui_top_title(ui);
        self.ui_command_toolbar(ui);
        self.ui_status_bar(ui);
        self.ui_inspector_panel(ui);

        let t = Tokens::get(&ctx);
        let dragging_files = ctx.input(|i| !i.raw.hovered_files.is_empty());

        egui::CentralPanel::default().frame(Frame::NONE.fill(t.pasteboard)).show(ui, |ui| {
            match self.tab {
                WorkspaceTab::Queue => {
                    if self.items.is_empty() {
                        self.ui_empty_queue(ui);
                    } else {
                        self.ui_populated_queue(ui);
                    }
                }
                WorkspaceTab::Languages => {
                    self.ui_models_workspace(ui);
                }
            }

            if dragging_files {
                let drop_rect = ui.max_rect().shrink(8.0);
                ui.painter().rect(
                    drop_rect,
                    CornerRadius::same(10),
                    t.accent_soft.gamma_multiply(0.45),
                    Stroke::new(2.0, t.accent),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    drop_rect.center(),
                    Align2::CENTER_CENTER,
                    tr("empty.drop_title"),
                    theme::semibold(16.0),
                    t.accent_text,
                );
            }
        });

        self.ui_toast(&ctx);
        self.platform.set_unsaved_srts(self.items.iter().any(SubtitleItem::awaits_download));

        // Borderless window edge & corner resizing modeled after PdfCraft
        if self.capabilities.window_controls {
            WindowResizer::default().handle(&ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown_workers();
        self.save_preferences();
    }
}
