use super::languages::LanguagePickerPopup;
use super::*;

/// Strongly typed user interactions emitted by the Queue workspace views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueAction {
    Select(usize),
    Start(usize),
    Cancel(usize),
    Remove(usize),
    PickDestination(usize),
    UpdateLanguage(usize, String),
    OpenModelsTab,
    ClearAll,
    ClearDone,
    AddFiles,
    AddFolder,
}

struct QueueRowRects {
    card: Rect,
    cy: f32,
    lang: Rect,
    dst: Rect,
    act: Rect,
    rem: Rect,
    select: Rect,
}

impl QueueRowRects {
    fn new(card_rect: Rect) -> Self {
        let cy = card_rect.center().y;
        let rem = Rect::from_center_size(pos2(card_rect.right() - 24.0, cy), vec2(28.0, 28.0));
        let act = Rect::from_center_size(pos2(rem.left() - 18.0, cy), vec2(28.0, 28.0));
        let dst = Rect::from_center_size(pos2(act.left() - 18.0, cy), vec2(28.0, 28.0));
        let lang_w = 148.0;
        let lang = Rect::from_center_size(pos2(dst.left() - 8.0 - lang_w * 0.5, cy), vec2(lang_w, ROW_CONTROL_HEIGHT));
        let select =
            Rect::from_min_max(card_rect.min, pos2((lang.left() - 4.0).max(card_rect.left() + 20.0), card_rect.bottom()));

        Self { card: card_rect, cy, lang, dst, act, rem, select }
    }
}

/// Stateless presentation view for the Empty Queue workspace with drop hero and feature cards.
pub struct EmptyQueueView;

impl EmptyQueueView {
    pub fn show(ui: &mut egui::Ui, actions: &mut Vec<QueueAction>) {
        let t = Tokens::get(ui.ctx());

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let outer_w = ui.available_width();
            let content_w = (outer_w - 56.0).clamp(280.0, 820.0);
            let h_pad = ((outer_w - content_w) * 0.5).max(16.0);
            let default_spacing = ui.spacing().item_spacing;

            ui.add_space(28.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(h_pad);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = default_spacing;
                    ui.set_width(content_w);
                    ui.set_max_width(content_w);

                    ui.label(RichText::new(tr("empty.title")).font(theme::semibold(22.0)).color(t.text));
                    ui.add_space(2.0);
                    ui.label(RichText::new(tr("empty.subtitle")).font(theme::regular(13.5)).color(t.text_muted));
                    ui.add_space(18.0);

                    Self::ui_empty_queue_hero(ui, &t, actions);
                    ui.add_space(18.0);

                    Self::ui_empty_queue_features(ui, &t, actions);
                    ui.add_space(22.0);

                    ui.label(RichText::new(tr("empty.privacy")).font(theme::regular(11.5)).color(t.text_faint));
                });
            });
            ui.add_space(28.0);
        });
    }

    fn ui_empty_queue_hero(ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<QueueAction>) {
        Frame::NONE
            .fill(t.card)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(24, 28))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    let (circle_rect, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                    ui.painter().circle_filled(circle_rect.center(), 24.0, t.accent_soft);
                    icons::paint(ui, circle_rect, "file-sub", 24.0, t.accent);

                    ui.add_space(10.0);
                    ui.label(RichText::new(tr("empty.drop_title")).font(theme::semibold(15.5)).color(t.text));
                    ui.add_space(3.0);
                    ui.label(RichText::new(tr("empty.drop_hint")).font(theme::regular(13.0)).color(t.text_muted));
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let lbl1 = tr("action.add_files");
                        let lbl2 = tr("action.add_folder");
                        let w1 = ui.fonts_mut(|f| f.layout_no_wrap(lbl1.to_owned(), theme::medium(12.5), t.text).size().x) + 42.0;
                        let w2 = ui.fonts_mut(|f| f.layout_no_wrap(lbl2.to_owned(), theme::medium(12.5), t.text).size().x) + 42.0;
                        let total_btns_w = w1 + w2 + 10.0;
                        let pad = ((ui.available_width() - total_btns_w) * 0.5).max(0.0);
                        ui.add_space(pad);
                        ui.spacing_mut().item_spacing.x = 10.0;

                        if widgets::icon_pill(ui, "plus", lbl1, true, 32.0).clicked() {
                            actions.push(QueueAction::AddFiles);
                        }
                        if widgets::icon_pill(ui, "folder-open", lbl2, false, 32.0).clicked() {
                            actions.push(QueueAction::AddFolder);
                        }
                    });
                });
            });
    }

    fn ui_empty_queue_features(ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<QueueAction>) {
        let gap = 12.0;
        let avail = ui.available_width();
        let card_w = if avail >= 480.0 { (avail - 2.0 * gap) / 3.0 } else { avail };
        let card_h = 118.0;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);

            let cards = [
                ("sparkles", t.accent, tr("empty.card1_title"), tr("empty.card1_body"), false),
                ("play", t.success, tr("empty.card2_title"), tr("empty.card2_body"), false),
                ("globe", t.accent, tr("empty.card3_title"), tr("empty.card3_body"), true),
            ];

            for (icon, tint, title, body, clickable) in cards {
                let sense = if clickable { Sense::click() } else { Sense::hover() };
                let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), sense);
                let press = theme::Press::track(ui, &resp);
                if clickable {
                    widgets::card_lift(ui, t, rect, press);
                }
                let fill = if clickable { press.fill(t.card, t.hover, t.pressed) } else { t.card };
                ui.painter().rect(rect, CornerRadius::same(10), fill, Stroke::new(1.0, t.border), StrokeKind::Inside);

                let r = if clickable { rect.translate(press.offset()) } else { rect };
                icons::paint(ui, Rect::from_min_size(r.min + vec2(14.0, 14.0), vec2(20.0, 20.0)), icon, 18.0, tint);
                widgets::row_text(ui, r.min + vec2(40.0, 24.0), title, theme::semibold(13.0), t.text, r.width() - 52.0);

                let mut job = egui::text::LayoutJob::simple(
                    body.to_owned(),
                    theme::regular(11.5),
                    t.text_muted,
                    (r.width() - 28.0).max(40.0),
                );
                job.wrap.max_rows = if clickable { 2 } else { 3 };
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                ui.painter().galley(r.min + vec2(14.0, 44.0), galley, t.text_muted);

                if clickable {
                    widgets::row_text(
                        ui,
                        r.left_bottom() + vec2(14.0, -15.0),
                        tr("action.open_models"),
                        theme::medium(11.5),
                        t.accent_text,
                        r.width() - 28.0,
                    );
                    if theme::hand(resp).clicked() {
                        actions.push(QueueAction::OpenModelsTab);
                    }
                }
            }
        });
    }
}

/// Presentation view for the Populated Queue list workspace.
pub struct QueueView<'a> {
    pub items: &'a [SubtitleItem],
    pub selected_id: Option<usize>,
    pub scroll_to_selected_row: bool,
    pub installed_langs: &'a [String],
    pub ui_language: UiLanguage,
    pub popup_lang_search: &'a mut String,
    pub popup_highlight_idx: &'a mut usize,
}

impl<'a> QueueView<'a> {
    pub fn new(
        items: &'a [SubtitleItem],
        selected_id: Option<usize>,
        scroll_to_selected_row: bool,
        installed_langs: &'a [String],
        ui_language: UiLanguage,
        popup_lang_search: &'a mut String,
        popup_highlight_idx: &'a mut usize,
    ) -> Self {
        Self { items, selected_id, scroll_to_selected_row, installed_langs, ui_language, popup_lang_search, popup_highlight_idx }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, actions: &mut Vec<QueueAction>) {
        let t = Tokens::get(ui.ctx());

        Frame::NONE.inner_margin(Margin::symmetric(18, 14)).show(ui, |ui| {
            Self::ui_queue_header_strip(ui, self.items, &t, actions);

            ui.add_space(10.0);

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for item in self.items {
                    self.ui_queue_row(ui, item, &t, actions);
                }
            });
        });
    }

    fn ui_queue_header_strip(ui: &mut egui::Ui, items: &[SubtitleItem], t: &Tokens, actions: &mut Vec<QueueAction>) {
        let total = items.len();
        let done_count = items.iter().filter(|i| matches!(i.status, ItemStatus::Completed { .. })).count();
        let pending_count = items.iter().filter(|i| matches!(i.status, ItemStatus::Pending)).count();

        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("queue.header")).font(theme::semibold(15.0)).color(t.text));
            ui.label(
                RichText::new(format!(
                    "{total}  ·  {done_count} {}  ·  {pending_count} {}",
                    tr("badge.done").to_lowercase(),
                    tr("badge.pending").to_lowercase()
                ))
                .font(theme::regular(12.0))
                .color(t.text_muted),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if widgets::ghost_button(ui, "trash", tr("action.clear_all"), ROW_CONTROL_HEIGHT).clicked() {
                    actions.push(QueueAction::ClearAll);
                }
                if done_count > 0 && widgets::ghost_button(ui, "check", tr("action.clear_done"), ROW_CONTROL_HEIGHT).clicked() {
                    actions.push(QueueAction::ClearDone);
                }
            });
        });
    }

    fn ui_queue_row(&mut self, ui: &mut egui::Ui, item: &SubtitleItem, t: &Tokens, actions: &mut Vec<QueueAction>) {
        let is_selected = self.selected_id == Some(item.id);
        let is_converting = matches!(item.status, ItemStatus::Converting { .. });

        let (card_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), QUEUE_ROW_HEIGHT), Sense::hover());
        if is_selected && self.scroll_to_selected_row {
            ui.scroll_to_rect(card_rect, None);
        }

        let rects = QueueRowRects::new(card_rect);
        let row_resp = Self::ui_queue_row_bg(ui, item.id, &rects, is_selected, is_converting, t, actions);
        Self::ui_queue_row_text_info(ui, item, &rects, t);
        self.ui_queue_row_controls(ui, item, is_converting, &rects, actions);
        Self::ui_queue_row_progress_bar(ui, item, card_rect, t);

        row_resp.on_hover_text(item.input_path.display().to_string());
        ui.advance_cursor_after_rect(card_rect);
    }

    fn ui_queue_row_bg(
        ui: &mut egui::Ui,
        item_id: usize,
        rects: &QueueRowRects,
        is_selected: bool,
        is_converting: bool,
        t: &Tokens,
        actions: &mut Vec<QueueAction>,
    ) -> egui::Response {
        let row_resp = ui.interact(rects.select, ui.scope_id().with(("queue_row", item_id)), Sense::click());
        if theme::hand(row_resp.clone()).clicked() {
            actions.push(QueueAction::Select(item_id));
        }

        let row_hovered = ui.rect_contains_pointer(rects.card);
        let base_fill = if is_selected {
            t.selected
        } else if row_hovered {
            t.card.lerp_to_gamma(t.hover, 0.55)
        } else {
            t.card
        };
        let border_col = if is_converting || is_selected { t.accent } else { t.border };
        ui.painter().rect(rects.card, CornerRadius::same(8), base_fill, Stroke::new(1.0, border_col), StrokeKind::Inside);

        row_resp
    }

    fn ui_queue_row_text_info(ui: &mut egui::Ui, item: &SubtitleItem, rects: &QueueRowRects, t: &Tokens) {
        let (badge_text, badge_fg, badge_bg) = match &item.status {
            ItemStatus::Pending => (tr("badge.pending").to_string(), t.text_muted, t.hover),
            ItemStatus::Converting { cue, total, .. } => {
                let pct = if *total > 0 { (*cue as f32 / *total as f32 * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                (format!("{pct:.0}% · {cue}/{total}"), t.accent_text, t.accent_soft)
            }
            ItemStatus::Completed { cues_count } => (format!("{} · {cues_count}", tr("badge.done")), t.success, t.success_soft),
            ItemStatus::Failed(_) => (tr("badge.failed").to_string(), t.error, t.error_soft),
        };
        let badge_rect =
            widgets::paint_badge_right(ui, pos2(rects.lang.left() - 10.0, rects.cy), &badge_text, badge_fg, badge_bg);

        let icon_tint = match &item.status {
            ItemStatus::Converting { .. } => t.accent,
            ItemStatus::Completed { .. } => t.success,
            ItemStatus::Failed(_) => t.error,
            ItemStatus::Pending => t.icon,
        };
        icons::paint(
            ui,
            Rect::from_center_size(pos2(rects.card.left() + 24.0, rects.cy), vec2(22.0, 22.0)),
            "file-sub",
            20.0,
            icon_tint,
        );

        let text_left = rects.card.left() + 44.0;
        let max_text_w = (badge_rect.left() - 12.0 - text_left).max(0.0);
        let file_name = item.input_path.file_name().and_then(|s| s.to_str()).unwrap_or("subtitle.sup");
        let is_auto = item.detected_language.as_deref() == Some(item.language.as_str());
        let title_str = if is_auto { format!("{file_name}  [{}]", tr("badge.auto")) } else { file_name.to_string() };

        widgets::row_text(ui, pos2(text_left, rects.card.top() + 21.0), title_str, theme::semibold(13.5), t.text, max_text_w);

        let (sub_line, sub_color) = match &item.status {
            ItemStatus::Failed(err) => (err.clone(), t.error),
            _ => (format!("-> {}", item.output_path.display()), t.text_muted),
        };
        widgets::row_text(ui, pos2(text_left, rects.card.top() + 41.0), &sub_line, theme::regular(11.5), sub_color, max_text_w);
    }

    fn ui_queue_row_controls(
        &mut self,
        ui: &mut egui::Ui,
        item: &SubtitleItem,
        is_converting: bool,
        rects: &QueueRowRects,
        actions: &mut Vec<QueueAction>,
    ) {
        let item_id = item.id;
        let ui_lang = self.ui_language;

        // 1. Language Dropdown
        let lang_disp = format_language_label(&item.language, ui_lang);
        let lang_resp = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rects.lang), |ui| {
                ui.add_enabled_ui(!is_converting, |ui| {
                    widgets::dropdown_button(ui, Some("globe"), &lang_disp, rects.lang.width(), ROW_CONTROL_HEIGHT, "")
                })
                .inner
            })
            .inner;

        let row_just_opened = lang_resp.clicked();
        if row_just_opened {
            self.popup_lang_search.clear();
        }
        egui::Popup::menu(&lang_resp)
            .id(lang_resp.id.with(("row_lang_popup", item_id)))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(4.0)
            .show(|ui| {
                let mut popup =
                    LanguagePickerPopup::new(self.installed_langs, ui_lang, self.popup_lang_search, self.popup_highlight_idx);
                let (chosen, jump) = popup.show(ui, &item.language, row_just_opened);
                if let Some(code) = chosen {
                    actions.push(QueueAction::UpdateLanguage(item_id, code));
                }
                if jump {
                    actions.push(QueueAction::OpenModelsTab);
                }
            });

        // 2. Output .srt Destination Button
        let dst_tip = format!("{}\n{}", tr("action.choose_srt"), item.output_path.display());
        let dst_resp = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rects.dst), |ui| {
                ui.add_enabled_ui(!is_converting, |ui| icons::button(ui, "folder", 28.0, item.custom_output_override, &dst_tip))
                    .inner
            })
            .inner;
        if dst_resp.clicked() {
            actions.push(QueueAction::PickDestination(item_id));
        }

        // 3. Action Button (Play / Stop / Retry)
        let (act_icon, act_tip) = if is_converting {
            ("stop", tr("action.cancel"))
        } else if matches!(item.status, ItemStatus::Completed { .. } | ItemStatus::Failed(_)) {
            ("refresh", tr("action.retry"))
        } else {
            ("play", tr("action.convert"))
        };
        let act_resp = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rects.act), |ui| {
                icons::button(ui, act_icon, 28.0, is_converting, act_tip)
            })
            .inner;
        if act_resp.clicked() {
            if is_converting {
                actions.push(QueueAction::Cancel(item_id));
            } else {
                actions.push(QueueAction::Start(item_id));
            }
        }

        // 4. Remove Button
        let rem_resp = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rects.rem), |ui| {
                icons::button(ui, "x", 28.0, false, tr("action.remove"))
            })
            .inner;
        if rem_resp.clicked() {
            actions.push(QueueAction::Remove(item_id));
        }
    }

    fn ui_queue_row_progress_bar(ui: &mut egui::Ui, item: &SubtitleItem, card_rect: Rect, t: &Tokens) {
        if let ItemStatus::Converting { cue, total, .. } = &item.status {
            let frac = if *total > 0 { (*cue as f32 / *total as f32).clamp(0.0, 1.0) } else { 0.04 };
            let shown = ui.ctx().animate_value_with_time(ui.scope_id().with(("row_prog", item.id)), frac, 0.18);
            let track = Rect::from_min_max(
                pos2(card_rect.left() + 6.0, card_rect.bottom() - 4.0),
                pos2(card_rect.right() - 6.0, card_rect.bottom() - 1.0),
            );
            ui.painter().rect_filled(track, CornerRadius::same(2), t.hover);
            let fill_w = (track.width() * shown).max(4.0);
            let done_bar = Rect::from_min_size(track.min, vec2(fill_w.min(track.width()), track.height()));
            ui.painter().rect_filled(done_bar, CornerRadius::same(2), t.accent);
        }
    }
}

impl PgsApp {
    /// Handles keyboard navigation (`Up`, `Down`, `Home`, `End`, `Enter`, `Delete`) in the Queue list
    /// whenever the Queue workspace is active and no popup menu is open.
    pub fn handle_queue_keyboard(&mut self, ctx: &egui::Context) {
        if self.tab != WorkspaceTab::Queue || self.items.is_empty() || egui::Popup::is_any_open(ctx) {
            return;
        }

        let mut move_dir: Option<isize> = None;
        let mut jump_home = false;
        let mut jump_end = false;
        let mut trigger_start = false;
        let mut trigger_delete = false;

        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                move_dir = Some(1);
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                move_dir = Some(-1);
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
                jump_home = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::End) {
                jump_end = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                trigger_start = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Delete) {
                trigger_delete = true;
            }
        });

        let len = self.items.len();
        let current_idx = self.selected_id.and_then(|id| self.items.iter().position(|item| item.id == id));

        let next_idx = if jump_home {
            Some(0)
        } else if jump_end {
            Some(len - 1)
        } else {
            move_dir.map(|delta| match current_idx {
                Some(idx) if delta > 0 => (idx + 1).min(len - 1),
                Some(idx) => idx.saturating_sub(1),
                None if delta > 0 => 0,
                None => len - 1,
            })
        };

        if let Some(idx) = next_idx {
            let new_id = self.items[idx].id;
            if self.selected_id != Some(new_id) {
                self.selected_id = Some(new_id);
                self.scroll_to_selected_row = true;
                if !self.sample_previews.contains_key(&new_id) {
                    self.request_sample_preview(new_id, Some(ctx));
                }
            }
        } else if trigger_delete {
            if let Some(sel_id) = self.selected_id {
                self.remove_item(sel_id);
                self.scroll_to_selected_row = true;
            }
        } else if trigger_start {
            if let Some(sel_id) = self.selected_id {
                self.start_item(sel_id, Some(ctx));
            }
        }
    }

    pub fn ui_empty_queue(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        EmptyQueueView::show(ui, &mut actions);
        let ctx = ui.ctx().clone();
        for action in actions {
            self.handle_queue_action(action, &ctx);
        }
    }

    pub fn ui_populated_queue(&mut self, ui: &mut egui::Ui) {
        let mut view = QueueView::new(
            &self.items,
            self.selected_id,
            self.scroll_to_selected_row,
            &self.installed_langs,
            self.preferences.ui_language,
            &mut self.popup_lang_search,
            &mut self.popup_highlight_idx,
        );
        let mut actions = Vec::new();
        view.show(ui, &mut actions);
        let ctx = ui.ctx().clone();
        for action in actions {
            self.handle_queue_action(action, &ctx);
        }
    }

    pub fn handle_queue_action(&mut self, action: QueueAction, ctx: &egui::Context) {
        match action {
            QueueAction::Select(id) => {
                self.selected_id = Some(id);
                if !self.sample_previews.contains_key(&id) {
                    self.request_sample_preview(id, Some(ctx));
                }
            }
            QueueAction::Start(id) => {
                self.selected_id = Some(id);
                self.start_item(id, Some(ctx));
            }
            QueueAction::Cancel(id) => {
                self.cancel_item(id);
            }
            QueueAction::Remove(id) => {
                self.remove_item(id);
            }
            QueueAction::PickDestination(id) => {
                self.selected_id = Some(id);
                if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                    let default_name = item.output_path.file_name().and_then(|s| s.to_str()).unwrap_or("subtitle.srt");
                    let mut dialog =
                        rfd::FileDialog::new().add_filter("SubRip Subtitle (.srt)", &["srt"]).set_file_name(default_name);
                    if let Some(parent) = item.output_path.parent() {
                        dialog = dialog.set_directory(parent);
                    }
                    if let Some(chosen) = dialog.save_file() {
                        item.output_path = chosen;
                        item.custom_output_override = true;
                    }
                }
            }
            QueueAction::UpdateLanguage(id, code) => {
                self.cancel_probe(id);
                let custom_dir = self.get_effective_custom_output_dir().map(Path::to_path_buf);
                if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                    item.update_language(code, custom_dir.as_deref());
                }
            }
            QueueAction::OpenModelsTab => {
                self.tab = WorkspaceTab::Languages;
            }
            QueueAction::ClearAll => {
                self.cancel_batch();
                let ids: Vec<usize> = self.items.iter().map(|i| i.id).collect();
                for id in ids {
                    self.remove_item(id);
                }
            }
            QueueAction::ClearDone => {
                let done_ids: Vec<usize> =
                    self.items.iter().filter(|i| matches!(i.status, ItemStatus::Completed { .. })).map(|i| i.id).collect();
                for id in done_ids {
                    self.remove_item(id);
                }
            }
            QueueAction::AddFiles => {
                if let Some(files) =
                    rfd::FileDialog::new().add_filter("Blu-ray PGS Subtitle (.sup)", &["sup", "SUP"]).pick_files()
                {
                    let mut count = 0;
                    for path in files {
                        if self.add_file(path, Some(ctx)) {
                            count += 1;
                        }
                    }
                    if count > 0 {
                        self.tab = WorkspaceTab::Queue;
                        self.notify(format!("{} ({count})", tr("toast.added_files")), false);
                    }
                }
            }
            QueueAction::AddFolder => {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    let added = self.add_folder(&folder, Some(ctx));
                    if added > 0 {
                        self.tab = WorkspaceTab::Queue;
                        self.notify(format!("{} ({added})", tr("toast.added_files")), false);
                    } else {
                        self.notify(tr("toast.no_sup_in_folder"), true);
                    }
                }
            }
        }
        self.scroll_to_selected_row = false;
    }
}
