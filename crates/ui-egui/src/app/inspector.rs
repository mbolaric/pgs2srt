use super::languages::LanguagePickerPopup;
use super::*;

/// Strongly typed user interactions emitted by the right inspector panel view.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectorAction {
    SetSplitterWidth(f32),
    SaveSplitterWidth(f32),
    UpdateLanguage(usize, String),
    PickDestination(usize),
    ResetDestination(usize),
    SetAllowOverwrite(bool),
    StartConversion(usize),
    RequestLanguageProbe(usize),
    DownloadLanguage(String),
    RequestSamplePreview(usize),
    OpenModelsTab,
}

/// Presentation view for the detail / cinema inspector right sidebar.
pub struct InspectorView<'a> {
    pub selected_item: Option<&'a SubtitleItem>,
    pub splitter_position: f32,
    pub installed_langs: &'a [String],
    pub ui_language: UiLanguage,
    pub allow_overwrite: bool,
    pub downloading_lang: Option<&'a str>,
    pub preview_meta: Option<(usize, usize, u32, u32, String)>,
    pub preview_tex: Option<(&'a egui::TextureHandle, egui::Vec2)>,
    pub popup_lang_search: &'a mut String,
    pub popup_highlight_idx: &'a mut usize,
}

impl<'a> InspectorView<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        selected_item: Option<&'a SubtitleItem>,
        splitter_position: f32,
        installed_langs: &'a [String],
        ui_language: UiLanguage,
        allow_overwrite: bool,
        downloading_lang: Option<&'a str>,
        preview_meta: Option<(usize, usize, u32, u32, String)>,
        preview_tex: Option<(&'a egui::TextureHandle, egui::Vec2)>,
        popup_lang_search: &'a mut String,
        popup_highlight_idx: &'a mut usize,
    ) -> Self {
        Self {
            selected_item,
            splitter_position,
            installed_langs,
            ui_language,
            allow_overwrite,
            downloading_lang,
            preview_meta,
            preview_tex,
            popup_lang_search,
            popup_highlight_idx,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, actions: &mut Vec<InspectorAction>) {
        let t = Tokens::get(ui.ctx());

        let panel_resp = egui::Panel::right("inspector_panel")
            .resizable(true)
            .default_size(self.splitter_position)
            .size_range(280.0..=480.0)
            .frame(Frame::NONE.fill(t.panel))
            .show(ui, |ui| {
                let panel_w = ui.available_width();
                ui.set_max_width(panel_w);

                let panel_rect = ui.max_rect();
                ui.painter().vline(panel_rect.left() + 0.5, panel_rect.y_range(), Stroke::new(1.0, t.divider));

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_max_width(panel_w);
                    Frame::NONE.inner_margin(Margin::symmetric(16, 12)).show(ui, |ui| {
                        let inner_w = (panel_w - 32.0).max(80.0);
                        ui.set_max_width(inner_w);
                        ui.set_width(inner_w);

                        if let Some(item) = self.selected_item {
                            let is_converting = matches!(item.status, ItemStatus::Converting { .. });

                            Self::ui_inspector_file_info(ui, item, &t, actions);
                            self.ui_inspector_ocr_section(ui, item, &t, is_converting, actions);
                            self.ui_inspector_output_section(ui, item, &t, is_converting, actions);
                        } else {
                            widgets::section_title(ui, tr("inspector.file_section"));
                            ui.add(
                                egui::Label::new(
                                    RichText::new(tr("inspector.empty")).font(theme::regular(12.5)).color(t.text_muted),
                                )
                                .wrap(),
                            );
                        }

                        let sel_id = self.selected_item.map(|i| i.id);
                        Self::ui_inspector_preview_section(
                            ui,
                            &t,
                            inner_w,
                            self.preview_tex,
                            self.preview_meta.as_ref(),
                            sel_id,
                            actions,
                        );
                    });
                });
            });

        let current_w = panel_resp.response.rect.width().round();
        if (current_w - self.splitter_position).abs() > 0.5 {
            actions.push(InspectorAction::SetSplitterWidth(current_w));
        }

        if ui.ctx().input(|i| i.pointer.any_released()) {
            actions.push(InspectorAction::SaveSplitterWidth(current_w));
        }
    }

    fn ui_inspector_file_info(ui: &mut egui::Ui, item: &SubtitleItem, t: &Tokens, actions: &mut Vec<InspectorAction>) {
        widgets::section_title(ui, tr("inspector.file_section"));
        let file_name = item.input_path.file_name().and_then(|s| s.to_str()).unwrap_or("subtitle.sup");
        ui.add(egui::Label::new(RichText::new(file_name).font(theme::semibold(14.0)).color(t.text)).wrap());

        let cues_str = item.total_cues_hint.map_or_else(|| "—".to_string(), |n| n.to_string());
        ui.label(
            RichText::new(format!(
                "{}: {}  ·  {}: {}",
                tr("inspector.size"),
                human_size(item.file_size),
                tr("inspector.cues"),
                cues_str
            ))
            .font(theme::regular(12.0))
            .color(t.text_muted),
        );
        ui.add_space(6.0);

        Frame::NONE
            .fill(t.hover)
            .stroke(Stroke::new(1.0, t.divider))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(tr("inspector.source")).font(theme::semibold(10.5)).color(t.text_faint));
                ui.add(
                    egui::Label::new(
                        RichText::new(item.input_path.display().to_string()).font(theme::mono(11.0)).color(t.text_muted),
                    )
                    .wrap(),
                );
            });

        // Error Banner (if Failed)
        if let ItemStatus::Failed(err_msg) = &item.status {
            ui.add_space(10.0);
            Frame::NONE
                .fill(t.error_soft)
                .stroke(Stroke::new(1.0, t.error.gamma_multiply(0.6)))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (alert_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                        icons::paint(ui, alert_rect, "alert", 15.0, t.error);
                        ui.label(RichText::new(tr("inspector.error_title")).font(theme::semibold(12.5)).color(t.error));
                    });
                    ui.add_space(4.0);
                    ui.add(egui::Label::new(RichText::new(err_msg).font(theme::regular(12.0)).color(t.text)).wrap());
                    ui.add_space(6.0);
                    if widgets::icon_pill(ui, "refresh", tr("action.retry"), false, 26.0).clicked() {
                        actions.push(InspectorAction::StartConversion(item.id));
                    }
                });
        }
    }

    fn ui_inspector_ocr_section(
        &mut self,
        ui: &mut egui::Ui,
        item: &SubtitleItem,
        t: &Tokens,
        is_converting: bool,
        actions: &mut Vec<InspectorAction>,
    ) {
        ui.add_space(8.0);
        widgets::section_title(ui, tr("inspector.ocr_section"));
        let lang_label = format_language_label(&item.language, self.ui_language);
        let drop_resp = ui
            .add_enabled_ui(!is_converting, |ui| {
                widgets::dropdown_button(ui, Some("globe"), &lang_label, ui.available_width(), TOOLBAR_HEIGHT, "")
            })
            .inner;

        let insp_just_opened = drop_resp.clicked();
        if insp_just_opened {
            self.popup_lang_search.clear();
        }
        let mut new_lang = None;
        let mut jump_to_models = false;
        egui::Popup::menu(&drop_resp)
            .id(drop_resp.id.with("insp_lang_popup"))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(4.0)
            .show(|ui| {
                let mut popup = LanguagePickerPopup::new(
                    self.installed_langs,
                    self.ui_language,
                    self.popup_lang_search,
                    self.popup_highlight_idx,
                );
                let (chosen, open_models) = popup.show(ui, &item.language, insp_just_opened);
                new_lang = chosen;
                jump_to_models = open_models;
            });

        if let Some(code) = new_lang {
            actions.push(InspectorAction::UpdateLanguage(item.id, code));
        }
        if jump_to_models {
            actions.push(InspectorAction::OpenModelsTab);
        }

        let is_model_installed = self.installed_langs.contains(&item.language);
        if !is_model_installed {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::badge(ui, tr("badge.missing_model"), t.warning, t.warning_soft);
                if self.downloading_lang.is_none()
                    && widgets::icon_pill(ui, "download", &format!("{} '{}'", tr("action.download"), item.language), true, 26.0)
                        .clicked()
                {
                    actions.push(InspectorAction::DownloadLanguage(item.language.clone()));
                }
            });
        }

        ui.add_space(6.0);
        let probe_lbl = if item.probing_language { tr("action.detecting") } else { tr("action.detect_image") };
        if ui
            .add_enabled_ui(!is_converting && !item.probing_language, |ui| {
                widgets::icon_pill(ui, "sparkles", probe_lbl, false, ROW_CONTROL_HEIGHT)
            })
            .inner
            .clicked()
        {
            actions.push(InspectorAction::RequestLanguageProbe(item.id));
        }
        ui.add_space(4.0);
        ui.add(egui::Label::new(RichText::new(tr("inspector.lang_hint")).font(theme::regular(11.5)).color(t.text_muted)).wrap());
    }

    fn ui_inspector_output_section(
        &self,
        ui: &mut egui::Ui,
        item: &SubtitleItem,
        t: &Tokens,
        is_converting: bool,
        actions: &mut Vec<InspectorAction>,
    ) {
        ui.add_space(8.0);
        widgets::section_title(ui, tr("inspector.output_section"));
        Frame::NONE
            .fill(t.hover)
            .stroke(Stroke::new(1.0, t.divider))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(
                    egui::Label::new(RichText::new(item.output_path.display().to_string()).font(theme::mono(11.0)).color(t.text))
                        .wrap(),
                );
                if !is_converting
                    && !matches!(item.status, ItemStatus::Completed { .. })
                    && item.output_file_exists()
                    && !self.allow_overwrite
                {
                    ui.add_space(4.0);
                    widgets::badge(ui, tr("badge.exists"), t.warning, t.warning_soft);
                }
            });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled_ui(!is_converting, |ui| {
                    widgets::icon_pill(ui, "folder", tr("action.browse"), false, ROW_CONTROL_HEIGHT)
                })
                .inner
                .clicked()
            {
                actions.push(InspectorAction::PickDestination(item.id));
            }

            if item.custom_output_override
                && !is_converting
                && widgets::ghost_button(ui, "refresh", tr("action.reset_default"), ROW_CONTROL_HEIGHT).clicked()
            {
                actions.push(InspectorAction::ResetDestination(item.id));
            }
        });

        ui.add_space(6.0);
        let mut overwrite = self.allow_overwrite;
        if ui.add(widgets::checkbox(&mut overwrite, tr("inspector.overwrite"))).changed() {
            actions.push(InspectorAction::SetAllowOverwrite(overwrite));
        }
    }

    fn ui_inspector_preview_section(
        ui: &mut egui::Ui,
        t: &Tokens,
        inner_w: f32,
        preview_tex: Option<(&egui::TextureHandle, egui::Vec2)>,
        preview_meta: Option<&(usize, usize, u32, u32, String)>,
        selected_id: Option<usize>,
        actions: &mut Vec<InspectorAction>,
    ) {
        ui.add_space(12.0);
        ui.separator();
        widgets::section_title(ui, tr("inspector.preview_section"));

        Self::ui_inspector_preview_meta_strip(ui, preview_meta, inner_w, t);
        ui.add_space(6.0);

        Self::ui_inspector_cinema_stage(ui, t, inner_w, preview_tex, preview_meta, selected_id, actions);
    }

    fn ui_inspector_preview_meta_strip(
        ui: &mut egui::Ui,
        meta: Option<&(usize, usize, u32, u32, String)>,
        inner_w: f32,
        t: &Tokens,
    ) {
        let (meta_rect, _) = ui.allocate_exact_size(vec2(inner_w, 22.0), Sense::hover());
        if let Some((cue, total, start_ts, stop_ts, _)) = meta {
            ui.scope_builder(egui::UiBuilder::new().max_rect(meta_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
                widgets::badge(ui, &format!("#{cue} / {total}"), t.accent_text, t.accent_soft);
                let ts_str = format!("{} --> {}", presentation_ts_into_string(*start_ts), presentation_ts_into_string(*stop_ts));
                widgets::row_text(
                    ui,
                    pos2(ui.cursor().min.x + 2.0, meta_rect.center().y),
                    ts_str,
                    theme::mono(11.0),
                    t.text_muted,
                    ui.available_width().max(0.0),
                );
            });
            ui.advance_cursor_after_rect(meta_rect);
        }
    }

    fn ui_inspector_cinema_stage(
        ui: &mut egui::Ui,
        t: &Tokens,
        _inner_w: f32,
        preview_tex: Option<(&egui::TextureHandle, egui::Vec2)>,
        meta: Option<&(usize, usize, u32, u32, String)>,
        selected_id: Option<usize>,
        actions: &mut Vec<InspectorAction>,
    ) {
        const BITMAP_STAGE_H: f32 = 64.0;
        const DIVIDER_BAND_H: f32 = 10.0;
        const TEXT_STAGE_H: f32 = 52.0;
        const TOTAL_STAGE_H: f32 = BITMAP_STAGE_H + DIVIDER_BAND_H + TEXT_STAGE_H;

        Frame::NONE
            .fill(t.cinema_bg)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                let cinema_inner_w = ui.available_width();
                ui.set_max_width(cinema_inner_w);
                ui.set_width(cinema_inner_w);
                ui.spacing_mut().item_spacing.y = 0.0;

                if preview_tex.is_some() || meta.is_some() {
                    let (bmp_stage, _) = ui.allocate_exact_size(vec2(cinema_inner_w, BITMAP_STAGE_H), Sense::hover());
                    Self::ui_inspector_cinema_bitmap(ui, bmp_stage, cinema_inner_w, BITMAP_STAGE_H, preview_tex);

                    let (div_rect, _) = ui.allocate_exact_size(vec2(cinema_inner_w, DIVIDER_BAND_H), Sense::hover());
                    ui.painter().hline(div_rect.x_range(), div_rect.center().y, Stroke::new(1.0, Color32::from_white_alpha(24)));

                    let (txt_stage, _) = ui.allocate_exact_size(vec2(cinema_inner_w, TEXT_STAGE_H), Sense::hover());
                    let raw_text = meta.map(|m| m.4.as_str()).unwrap_or("");
                    Self::ui_inspector_cinema_text(ui, txt_stage, cinema_inner_w, raw_text, t);
                } else {
                    let (empty_stage, _) = ui.allocate_exact_size(vec2(cinema_inner_w, TOTAL_STAGE_H), Sense::hover());
                    Self::ui_inspector_cinema_empty(ui, empty_stage, selected_id, actions);
                }
            });
    }

    fn ui_inspector_cinema_bitmap(
        ui: &mut egui::Ui,
        stage: Rect,
        inner_w: f32,
        stage_h: f32,
        preview_tex: Option<(&egui::TextureHandle, egui::Vec2)>,
    ) {
        if let Some((tex, size)) = preview_tex {
            let scale = (inner_w / size.x).min(stage_h / size.y).min(1.0);
            let draw_size = size * scale;
            let img_rect = Rect::from_center_size(stage.center(), draw_size);
            ui.painter().image(tex.id(), img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            ui.painter().text(
                stage.center(),
                Align2::CENTER_CENTER,
                tr("inspector.preview_blank"),
                theme::regular(12.0),
                Color32::from_rgb(0x9C, 0xA3, 0xAF),
            );
        }
    }

    fn ui_inspector_cinema_text(ui: &mut egui::Ui, stage: Rect, inner_w: f32, raw_text: &str, t: &Tokens) {
        let (shown_text, txt_color) =
            if raw_text.is_empty() { ("—", Color32::from_rgb(0x6B, 0x72, 0x80)) } else { (raw_text, t.cinema_text) };
        let mut job = egui::text::LayoutJob::simple(shown_text.to_owned(), theme::medium(13.0), txt_color, inner_w.max(40.0));
        job.halign = Align::Center;
        job.wrap.max_rows = 3;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let text_pos = pos2(stage.center().x, (stage.center().y - galley.size().y * 0.5).max(stage.top()));
        ui.painter().with_clip_rect(stage).galley(text_pos, galley, txt_color);
    }

    fn ui_inspector_cinema_empty(ui: &mut egui::Ui, stage: Rect, selected_id: Option<usize>, actions: &mut Vec<InspectorAction>) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(stage).layout(Layout::top_down(Align::Center)), |ui| {
            ui.add_space(28.0);
            ui.add(
                egui::Label::new(
                    RichText::new(tr("inspector.preview_empty"))
                        .font(theme::regular(12.0))
                        .color(Color32::from_rgb(0x9C, 0xA3, 0xAF)),
                )
                .wrap(),
            );
            if let Some(sel_id) = selected_id {
                ui.add_space(10.0);
                if widgets::icon_pill(ui, "eye", tr("action.preview_cue"), false, 26.0).clicked() {
                    actions.push(InspectorAction::RequestSamplePreview(sel_id));
                }
            }
        });
        ui.advance_cursor_after_rect(stage);
    }
}

impl PgsApp {
    pub fn ui_inspector_panel(&mut self, ui: &mut egui::Ui) {
        if !self.show_inspector || self.tab != WorkspaceTab::Queue {
            return;
        }

        let ctx = ui.ctx().clone();
        let preview_tex = self.ensure_preview_texture(&ctx);
        let preview_tex_ref = preview_tex.as_ref().map(|(th, sz)| (th, *sz));
        let preview_meta = self.active_preview().map(|p| (p.cue, p.total, p.start_ts, p.stop_ts, p.text.trim().to_string()));

        let selected_item = self.selected_id.and_then(|id| self.items.iter().find(|i| i.id == id));

        let mut view = InspectorView::new(
            selected_item,
            self.preferences.splitter_position,
            &self.installed_langs,
            self.preferences.ui_language,
            self.preferences.allow_overwrite,
            self.downloading_lang.as_deref(),
            preview_meta,
            preview_tex_ref,
            &mut self.popup_lang_search,
            &mut self.popup_highlight_idx,
        );

        let mut actions = Vec::new();
        view.show(ui, &mut actions);

        for action in actions {
            self.handle_inspector_action(action, &ctx);
        }
    }

    pub fn handle_inspector_action(&mut self, action: InspectorAction, ctx: &egui::Context) {
        match action {
            InspectorAction::SetSplitterWidth(w) => {
                self.preferences.splitter_position = w;
            }
            InspectorAction::SaveSplitterWidth(w) => {
                if (w - self.last_saved_splitter).abs() > 0.5 {
                    self.last_saved_splitter = w;
                    self.save_preferences();
                }
            }
            InspectorAction::UpdateLanguage(id, code) => {
                self.cancel_probe(id);
                let custom_dir = self.get_effective_custom_output_dir().map(Path::to_path_buf);
                if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
                    it.update_language(code, custom_dir.as_deref());
                }
            }
            InspectorAction::PickDestination(id) => {
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
            InspectorAction::ResetDestination(id) => {
                let custom_dir = self.get_effective_custom_output_dir().map(Path::to_path_buf);
                if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
                    it.reset_output_override(custom_dir.as_deref());
                }
            }
            InspectorAction::SetAllowOverwrite(overwrite) => {
                self.preferences.allow_overwrite = overwrite;
                self.save_preferences();
            }
            InspectorAction::StartConversion(id) => {
                self.start_item(id, Some(ctx));
            }
            InspectorAction::RequestLanguageProbe(id) => {
                self.request_language_probe(id, Some(ctx));
            }
            InspectorAction::DownloadLanguage(code) => {
                self.start_model_download(code);
            }
            InspectorAction::RequestSamplePreview(id) => {
                self.request_sample_preview(id, Some(ctx));
            }
            InspectorAction::OpenModelsTab => {
                self.tab = WorkspaceTab::Languages;
            }
        }
    }
}
