use super::*;

/// Reusable dropdown popup component for searching and selecting an OCR language model.
pub struct LanguagePickerPopup<'a> {
    pub installed_langs: &'a [String],
    pub ui_lang: UiLanguage,
    pub search: &'a mut String,
    pub highlight_idx: &'a mut usize,
}

impl<'a> LanguagePickerPopup<'a> {
    pub fn new(installed_langs: &'a [String], ui_lang: UiLanguage, search: &'a mut String, highlight_idx: &'a mut usize) -> Self {
        Self { installed_langs, ui_lang, search, highlight_idx }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, current_lang: &str, just_opened: bool) -> (Option<String>, bool) {
        let t = Tokens::get(ui.ctx());
        let mut open_models_tab = false;

        ui.set_min_width(244.0);
        ui.set_max_width(264.0);

        let filtered = self.filter_languages();
        let (mut scroll_to_highlight, confirm_enter) =
            self.handle_popup_keyboard(ui, filtered.len(), just_opened, current_lang, &filtered);

        let search_changed = self.ui_popup_search(ui, just_opened);
        let filtered = if search_changed {
            *self.highlight_idx = 0;
            scroll_to_highlight = true;
            self.filter_languages()
        } else {
            filtered
        };

        if !filtered.is_empty() {
            *self.highlight_idx = (*self.highlight_idx).min(filtered.len() - 1);
        } else {
            *self.highlight_idx = 0;
        }

        if confirm_enter {
            if let Some((code, _)) = filtered.get(*self.highlight_idx) {
                ui.close();
                return (Some(code.clone()), open_models_tab);
            }
        }

        ui.add_space(4.0);

        let chosen_lang = self.ui_popup_rows(ui, &filtered, current_lang, scroll_to_highlight, &t);

        ui.separator();
        if ui.button(RichText::new(tr("action.download_more")).font(theme::medium(12.0)).color(t.accent_text)).clicked() {
            open_models_tab = true;
            ui.close();
        }

        (chosen_lang, open_models_tab)
    }

    fn filter_languages(&self) -> Vec<(String, String)> {
        let q = self.search.trim().to_lowercase();
        self.installed_langs
            .iter()
            .filter_map(|code| {
                if language_matches_search(code, &q) {
                    Some((code.clone(), format_language_label(code, self.ui_lang)))
                } else {
                    None
                }
            })
            .collect()
    }

    fn handle_popup_keyboard(
        &mut self,
        ui: &egui::Ui,
        filtered_len: usize,
        just_opened: bool,
        current_lang: &str,
        filtered: &[(String, String)],
    ) -> (bool, bool) {
        let mut scroll_to_highlight = false;
        let mut confirm_enter = false;

        if just_opened {
            *self.highlight_idx = filtered.iter().position(|(code, _)| code == current_lang).unwrap_or(0);
            scroll_to_highlight = true;
        }

        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                if filtered_len > 0 {
                    *self.highlight_idx = (*self.highlight_idx + 1).min(filtered_len - 1);
                    scroll_to_highlight = true;
                }
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                if filtered_len > 0 {
                    *self.highlight_idx = (*self.highlight_idx).saturating_sub(1);
                    scroll_to_highlight = true;
                }
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown) {
                if filtered_len > 0 {
                    *self.highlight_idx = (*self.highlight_idx + 8).min(filtered_len - 1);
                    scroll_to_highlight = true;
                }
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp) {
                if filtered_len > 0 {
                    *self.highlight_idx = (*self.highlight_idx).saturating_sub(8);
                    scroll_to_highlight = true;
                }
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                confirm_enter = true;
            }
        });

        (scroll_to_highlight, confirm_enter)
    }

    fn ui_popup_search(&mut self, ui: &mut egui::Ui, just_opened: bool) -> bool {
        let search_resp = widgets::search_input(
            ui,
            "popup_lang_filter",
            self.search,
            tr("models.search_hint"),
            ui.available_width().max(224.0),
            28.0,
        );
        if just_opened {
            search_resp.request_focus();
        }
        search_resp.changed()
    }

    fn ui_popup_rows(
        &mut self,
        ui: &mut egui::Ui,
        filtered: &[(String, String)],
        current_lang: &str,
        scroll_to_highlight: bool,
        t: &Tokens,
    ) -> Option<String> {
        let mut chosen = None;
        let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);

        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            if filtered.is_empty() {
                ui.label(RichText::new(tr("models.no_results")).font(theme::regular(12.0)).color(t.text_muted));
                return;
            }

            let row_w = ui.available_width().max(216.0);
            for (idx, (code, label)) in filtered.iter().enumerate() {
                let is_highlighted = idx == *self.highlight_idx;
                let is_current = code == current_lang;

                let (row_rect, resp) = ui.allocate_exact_size(vec2(row_w, 26.0), Sense::click());
                if pointer_moved && resp.hovered() {
                    *self.highlight_idx = idx;
                }
                if scroll_to_highlight && is_highlighted {
                    resp.scroll_to_me(None);
                }

                if is_highlighted {
                    ui.painter().rect(
                        row_rect,
                        CornerRadius::same(5),
                        t.accent_soft,
                        Stroke::new(1.0, t.accent.gamma_multiply(0.45)),
                        StrokeKind::Inside,
                    );
                } else if is_current {
                    ui.painter().rect_filled(row_rect, CornerRadius::same(5), t.hover);
                }

                let text_col = if is_highlighted { t.accent_text } else { t.text };
                let font = if is_current || is_highlighted { theme::medium(12.5) } else { theme::regular(12.5) };
                let max_lbl_w = if is_current { row_rect.width() - 32.0 } else { row_rect.width() - 16.0 };
                widgets::row_text(ui, pos2(row_rect.left() + 8.0, row_rect.center().y), label, font, text_col, max_lbl_w);

                if is_current {
                    icons::paint(
                        ui,
                        Rect::from_center_size(pos2(row_rect.right() - 14.0, row_rect.center().y), vec2(14.0, 14.0)),
                        "check",
                        13.5,
                        t.accent_text,
                    );
                }

                if theme::hand(resp).clicked() {
                    chosen = Some(code.clone());
                    ui.close();
                }
            }
        });

        chosen
    }
}

/// Strongly typed user interactions emitted by the OCR language models workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguagesAction {
    SetFilter(ModelFilter),
    SetSearch(String),
    StartDownload(String),
    RefreshInstalled,
}

/// Stateless presentation view for the OCR Language Models catalog workspace.
#[derive(Debug, Clone)]
pub struct LanguagesView<'a> {
    pub catalog: &'a [ModelInfo],
    pub installed_langs: &'a [String],
    pub downloading_lang: Option<&'a str>,
    pub filter: ModelFilter,
    pub search_query: &'a str,
    pub ui_language: UiLanguage,
    pub tessdata_dir: Option<&'a Path>,
}

impl<'a> LanguagesView<'a> {
    pub fn new(
        catalog: &'a [ModelInfo],
        installed_langs: &'a [String],
        downloading_lang: Option<&'a str>,
        filter: ModelFilter,
        search_query: &'a str,
        ui_language: UiLanguage,
        tessdata_dir: Option<&'a Path>,
    ) -> Self {
        Self { catalog, installed_langs, downloading_lang, filter, search_query, ui_language, tessdata_dir }
    }

    pub fn show(&self, ui: &mut egui::Ui, actions: &mut Vec<LanguagesAction>) {
        let t = Tokens::get(ui.ctx());

        Frame::NONE.inner_margin(Margin::symmetric(24, 18)).show(ui, |ui| {
            self.ui_models_header(ui, &t, actions);
            ui.add_space(12.0);

            let total_models = self.catalog.len();
            let installed_in_cat = self.catalog.iter().filter(|m| self.installed_langs.contains(&m.code)).count();
            self.ui_models_search_and_filters(ui, total_models, installed_in_cat, actions);

            self.ui_models_downloading_banner(ui, &t);
            ui.add_space(12.0);

            self.ui_models_catalog_list(ui, &t, actions);
        });
    }

    fn ui_models_header(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<LanguagesAction>) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(tr("models.title")).font(theme::semibold(18.0)).color(t.text));
                ui.label(RichText::new(tr("models.subtitle")).font(theme::regular(12.5)).color(t.text_muted));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::ghost_button(ui, "refresh", tr("action.refresh"), TOOLBAR_HEIGHT).clicked() {
                    actions.push(LanguagesAction::RefreshInstalled);
                }
            });
        });

        if let Some(dir) = self.tessdata_dir {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{} {}", tr("models.folder_label"), dir.display()))
                    .font(theme::mono(11.0))
                    .color(t.text_faint),
            );
        }
    }

    fn ui_models_search_and_filters(
        &self,
        ui: &mut egui::Ui,
        total_models: usize,
        installed_in_cat: usize,
        actions: &mut Vec<LanguagesAction>,
    ) {
        let avail_in_cat = total_models.saturating_sub(installed_in_cat);
        let all_lbl = format!("{} ({total_models})", tr("filter.all"));
        let inst_lbl = format!("{} ({installed_in_cat})", tr("filter.installed"));
        let avail_lbl = format!("{} ({avail_in_cat})", tr("filter.available"));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let search_w = (ui.available_width() - 340.0).clamp(180.0, 380.0);
            let mut search_buf = self.search_query.to_owned();
            if widgets::search_input(
                ui,
                "models_catalog_search",
                &mut search_buf,
                tr("models.search_hint"),
                search_w,
                TOOLBAR_HEIGHT,
            )
            .changed()
            {
                actions.push(LanguagesAction::SetSearch(search_buf));
            }

            let mut cur_filter = self.filter;
            if widgets::segmented(
                ui,
                "models_filter_seg",
                &mut cur_filter,
                &[(ModelFilter::All, &all_lbl), (ModelFilter::Installed, &inst_lbl), (ModelFilter::Available, &avail_lbl)],
                TOOLBAR_HEIGHT,
            ) {
                actions.push(LanguagesAction::SetFilter(cur_filter));
            }
        });
    }

    fn ui_models_downloading_banner(&self, ui: &mut egui::Ui, t: &Tokens) {
        let Some(code) = self.downloading_lang else {
            return;
        };
        ui.add_space(10.0);
        Frame::NONE
            .fill(t.accent_soft)
            .stroke(Stroke::new(1.0, t.accent.gamma_multiply(0.45)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(format!(
                            "{} '{}' (tessdata_fast)…",
                            tr("status.downloading"),
                            format_language_label(code, self.ui_language)
                        ))
                        .font(theme::medium(12.5))
                        .color(t.accent_text),
                    );
                });
            });
    }

    fn ui_models_catalog_list(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<LanguagesAction>) {
        let query = self.search_query.trim().to_lowercase();
        let filtered: Vec<&ModelInfo> = self
            .catalog
            .iter()
            .filter(|m| {
                let is_inst = self.installed_langs.contains(&m.code);
                match self.filter {
                    ModelFilter::All => {}
                    ModelFilter::Installed if !is_inst => return false,
                    ModelFilter::Available if is_inst => return false,
                    _ => {}
                }
                language_matches_search(&m.code, &query)
            })
            .collect();

        if filtered.is_empty() {
            ui.add_space(24.0);
            ui.label(RichText::new(tr("models.no_results")).font(theme::regular(13.0)).color(t.text_muted));
            return;
        }

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for model in filtered {
                if self.ui_model_row(ui, model, t) {
                    actions.push(LanguagesAction::StartDownload(model.code.clone()));
                }
            }
        });
    }

    fn ui_model_row(&self, ui: &mut egui::Ui, model: &ModelInfo, t: &Tokens) -> bool {
        let mut download_clicked = false;
        let is_installed = self.installed_langs.contains(&model.code);
        let is_downloading = self.downloading_lang == Some(model.code.as_str());

        let (row_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), MODEL_ROW_HEIGHT), Sense::hover());
        let hovered = ui.rect_contains_pointer(row_rect);
        let fill = if hovered { t.card.lerp_to_gamma(t.hover, 0.45) } else { t.card };

        ui.painter().rect(row_rect, CornerRadius::same(8), fill, Stroke::new(1.0, t.border), StrokeKind::Inside);

        let cy = row_rect.center().y;
        icons::paint(
            ui,
            Rect::from_center_size(pos2(row_rect.left() + 22.0, cy), vec2(18.0, 18.0)),
            if is_installed { "check" } else { "globe" },
            16.5,
            if is_installed { t.success } else { t.icon },
        );

        let display = format_language_label(&model.code, self.ui_language);
        let right_reserved = 150.0;
        let max_left_w = (row_rect.width() - right_reserved - 46.0).max(40.0);

        widgets::row_text(
            ui,
            pos2(row_rect.left() + 42.0, row_rect.top() + 17.5),
            display,
            theme::semibold(13.0),
            t.text,
            max_left_w,
        );
        widgets::row_text(
            ui,
            pos2(row_rect.left() + 42.0, row_rect.top() + 34.5),
            format!("{}  ·  {}", model.code, human_size(model.size_bytes)),
            theme::mono(11.0),
            t.text_muted,
            max_left_w,
        );

        // Right-side status badge or Download pill button
        if is_installed {
            widgets::paint_badge_right(ui, pos2(row_rect.right() - 14.0, cy), tr("badge.installed"), t.success, t.success_soft);
        } else if is_downloading {
            widgets::paint_badge_right(
                ui,
                pos2(row_rect.right() - 14.0, cy),
                tr("badge.downloading"),
                t.accent_text,
                t.accent_soft,
            );
        } else {
            let btn_w =
                ui.fonts_mut(|f| f.layout_no_wrap(tr("action.download").to_owned(), theme::medium(12.5), t.text).size().x) + 42.0;
            let btn_rect =
                Rect::from_center_size(pos2(row_rect.right() - 14.0 - btn_w * 0.5, cy), vec2(btn_w, ROW_CONTROL_HEIGHT));
            let dl_resp = ui
                .scope_builder(egui::UiBuilder::new().max_rect(btn_rect), |ui| {
                    ui.add_enabled_ui(self.downloading_lang.is_none(), |ui| {
                        widgets::icon_pill(ui, "download", tr("action.download"), false, ROW_CONTROL_HEIGHT)
                    })
                    .inner
                })
                .inner;
            if dl_resp.clicked() {
                download_clicked = true;
            }
        }

        ui.advance_cursor_after_rect(row_rect);
        download_clicked
    }
}

impl PgsApp {
    pub fn default_ocr_language(&self) -> &str {
        if self.installed_langs.iter().any(|l| l == "eng") {
            "eng"
        } else {
            self.installed_langs.first().map_or("eng", String::as_str)
        }
    }

    pub fn refresh_installed_languages(&mut self) {
        match installed_languages() {
            Ok(langs) if !langs.is_empty() => self.installed_langs = langs,
            Ok(_) => {}
            Err(e) => log::warn!("Cannot refresh installed OCR languages: {e}"),
        }
    }

    pub fn request_language_probe(&mut self, id: usize, ctx: Option<&egui::Context>) {
        self.start_language_probe(id, true, ctx);
    }

    pub(crate) fn start_language_probe(&mut self, id: usize, manual: bool, ctx: Option<&egui::Context>) {
        let candidates = self.installed_langs.clone();
        let input_path = if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if item.probing_language {
                return;
            }
            item.probing_language = true;
            item.input_path.clone()
        } else {
            return;
        };

        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(prev) = self.workers.probe_cancellations.insert(id, cancel.clone()) {
            prev.store(true, Ordering::Relaxed);
        }
        let handle = spawn_language_probe(id, input_path, candidates, manual, cancel, self.tx.clone(), ctx.cloned());
        self.track_worker(handle);
    }

    pub fn cancel_probe(&mut self, id: usize) {
        if let Some(cancel) = self.workers.probe_cancellations.remove(&id) {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.probing_language = false;
        }
    }

    pub(super) fn start_model_download(&mut self, code: String) {
        if self.downloading_lang.is_some() {
            return;
        }
        match start_download(code.clone()) {
            Ok(rx) => {
                self.downloading_lang = Some(code);
                self.download_rx = Some(rx);
            }
            Err(e) => {
                log::error!("Cannot start download of OCR language {code}: {e}");
                self.notify(format!("{} {e}", tr("toast.lang_failed")), true);
            }
        }
    }

    pub(super) fn handle_language_downloads(&mut self, _ctx: &egui::Context) {
        let Some(rx) = &self.download_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(code)) => {
                self.downloading_lang = None;
                self.download_rx = None;
                self.refresh_installed_languages();
                let label = format_language_label(&code, self.preferences.ui_language);
                self.notify(format!("{} {label}", tr("toast.lang_installed")), false);
            }
            Ok(Err(e)) => {
                log::error!("Download of OCR language {} failed: {e}", self.downloading_lang.as_deref().unwrap_or("?"));
                self.downloading_lang = None;
                self.download_rx = None;
                self.notify(format!("{} {e}", tr("toast.lang_failed")), true);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::error!("Download thread for {} stopped without a result", self.downloading_lang.as_deref().unwrap_or("?"));
                self.downloading_lang = None;
                self.download_rx = None;
                self.notify(I18nService::get("app.language_download_stopped_unexpectedly_try"), true);
            }
        }
    }

    pub fn ui_language_popup_contents(
        &mut self,
        ui: &mut egui::Ui,
        current_lang: &str,
        just_opened: bool,
    ) -> (Option<String>, bool) {
        let mut popup = LanguagePickerPopup::new(
            &self.installed_langs,
            self.preferences.ui_language,
            &mut self.popup_lang_search,
            &mut self.popup_highlight_idx,
        );
        popup.show(ui, current_lang, just_opened)
    }

    pub fn ui_models_workspace(&mut self, ui: &mut egui::Ui) {
        let view = LanguagesView::new(
            &self.catalog,
            &self.installed_langs,
            self.downloading_lang.as_deref(),
            self.model_filter,
            &self.language_search,
            self.preferences.ui_language,
            self.tessdata_dir.as_deref(),
        );
        let mut actions = Vec::new();
        view.show(ui, &mut actions);
        for action in actions {
            self.handle_languages_action(action);
        }
    }

    pub fn handle_languages_action(&mut self, action: LanguagesAction) {
        match action {
            LanguagesAction::SetFilter(f) => self.model_filter = f,
            LanguagesAction::SetSearch(s) => self.language_search = s,
            LanguagesAction::StartDownload(code) => self.start_model_download(code),
            LanguagesAction::RefreshInstalled => self.refresh_installed_languages(),
        }
    }
}
