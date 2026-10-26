use super::super::*;

/// Strongly typed user interactions emitted by the desktop top titlebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleBarAction {
    SwitchTab(WorkspaceTab),
    ToggleInspector,
    CycleTheme,
    SetTheme(ThemePreference),
    SetUiLanguage(UiLanguage),
    ToggleAutoDetect(bool),
}

/// Stateless presentation view for the desktop top titlebar and tab strip.
#[derive(Debug, Clone, Copy)]
pub struct TitleBarView {
    pub active_tab: WorkspaceTab,
    pub queue_count: usize,
    pub installed_models_count: usize,
    pub show_inspector: bool,
    pub theme_pref: ThemePreference,
    pub ui_language: UiLanguage,
    pub auto_detect_language: bool,
    pub capabilities: Capabilities,
}

impl TitleBarView {
    pub fn new(
        active_tab: WorkspaceTab,
        queue_count: usize,
        installed_models_count: usize,
        show_inspector: bool,
        theme_pref: ThemePreference,
        ui_language: UiLanguage,
        auto_detect_language: bool,
    ) -> Self {
        Self {
            active_tab,
            queue_count,
            installed_models_count,
            show_inspector,
            theme_pref,
            ui_language,
            auto_detect_language,
            capabilities: Capabilities::ALL,
        }
    }

    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Renders the 38pt top titlebar and tab strip matching PdfCraft desktop layout.
    pub fn show(&self, ui: &mut egui::Ui, actions: &mut Vec<TitleBarAction>) {
        let t = Tokens::get(ui.ctx());

        egui::Panel::top("top_titlebar")
            .exact_size(TAB_STRIP_HEIGHT)
            .frame(Frame::NONE.fill(t.titlebar).inner_margin(Margin::symmetric(10, 0)))
            .show(ui, |ui| {
                let bar_rect = ui.max_rect();
                Self::ui_window_drag_and_divider(ui, bar_rect, &t, self.capabilities.window_controls);

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    self.ui_titlebar_tabs(ui, &t, actions);

                    // Right-aligned cluster: Window controls (far right) + Quick actions
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if self.capabilities.window_controls {
                            Self::ui_window_controls(ui, &t);
                            widgets::vline_divider(ui, 18.0);
                        }

                        self.ui_quick_actions(ui, &t, actions);
                    });
                });
            });
    }

    fn ui_window_drag_and_divider(ui: &mut egui::Ui, bar_rect: Rect, t: &Tokens, window_controls: bool) {
        if window_controls {
            Self::ui_window_drag(ui, bar_rect);
        }
        ui.painter().hline(
            bar_rect.left() - 10.0..=bar_rect.right() + 10.0,
            bar_rect.bottom() - 0.5,
            Stroke::new(1.0, t.divider),
        );
    }

    fn ui_window_drag(ui: &mut egui::Ui, bar_rect: Rect) {
        let titlebar_id = ui.scope_id().with("top_titlebar_drag_area");
        let is_max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let drag_rect = if is_max {
            bar_rect
        } else {
            Rect::from_min_max(
                pos2(
                    bar_rect.min.x + WindowResizer::DEFAULT_BORDER_THICKNESS,
                    bar_rect.min.y + WindowResizer::DEFAULT_BORDER_THICKNESS,
                ),
                pos2(bar_rect.max.x - WindowResizer::DEFAULT_BORDER_THICKNESS, bar_rect.max.y),
            )
        };
        let drag_resp = ui.interact(drag_rect, titlebar_id, Sense::click_and_drag());
        if drag_resp.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if drag_resp.double_clicked_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
        }
    }

    fn ui_titlebar_tabs(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<TitleBarAction>) {
        // App brand icon before tabs
        let (icon_rect, icon_resp) = ui.allocate_exact_size(vec2(28.0, TAB_STRIP_HEIGHT), Sense::hover());
        icons::paint(ui, Rect::from_center_size(icon_rect.center(), vec2(18.0, 18.0)), "logo", 17.0, t.accent);
        icon_resp.on_hover_text("pgs2srt");

        ui.add_space(2.0);

        let queue_count = (self.queue_count > 0).then_some(self.queue_count);
        if widgets::mode_tab(
            ui,
            "file-sub",
            tr("tab.queue"),
            queue_count,
            self.active_tab == WorkspaceTab::Queue,
            TAB_STRIP_HEIGHT,
        )
        .clicked()
        {
            actions.push(TitleBarAction::SwitchTab(WorkspaceTab::Queue));
        }

        if widgets::mode_tab(
            ui,
            "globe",
            tr("tab.models"),
            Some(self.installed_models_count),
            self.active_tab == WorkspaceTab::Languages,
            TAB_STRIP_HEIGHT,
        )
        .clicked()
        {
            actions.push(TitleBarAction::SwitchTab(WorkspaceTab::Languages));
        }
    }

    fn ui_window_controls(ui: &mut egui::Ui, t: &Tokens) {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));

        // 1. Close button (washes soft red on hover, error red icon)
        let (close_rect, close_resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
        let close_press = theme::Press::track(ui, &close_resp);
        if close_resp.hovered() {
            ui.painter().rect_filled(close_rect, CornerRadius::same(t.radius), t.error_soft);
        }
        let close_tint = if close_resp.hovered() { t.error } else { t.icon };
        icons::paint(ui, close_rect.translate(close_press.offset()), "x", 13.0, close_tint);
        if theme::hand(close_resp).on_hover_text(tr("window.close")).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // 2. Maximize / Restore button
        let max_icon = if is_maximized { "restore" } else { "maximize" };
        let max_tip = if is_maximized { tr("window.restore") } else { tr("window.maximize") };
        if icons::button(ui, max_icon, 28.0, false, max_tip).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
        }

        // 3. Minimize button
        if icons::button(ui, "minus", 28.0, false, tr("window.minimize")).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn ui_quick_actions(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<TitleBarAction>) {
        // 4. Inspector toggle (square 28x28 icon button)
        if self.active_tab == WorkspaceTab::Queue
            && icons::button(ui, "panel-right", 28.0, self.show_inspector, tr("inspector.toggle_tip")).clicked()
        {
            actions.push(TitleBarAction::ToggleInspector);
        }

        // 5. Theme switcher
        self.ui_theme_button(ui, actions);

        // 6. UI Language switcher
        self.ui_language_button(ui, t, actions);

        // 7. Auto subtitle language discovery toggle
        self.ui_auto_discovery_button(ui, actions);
    }

    fn ui_theme_button(&self, ui: &mut egui::Ui, actions: &mut Vec<TitleBarAction>) {
        let (t_icon, t_label) = match self.theme_pref {
            ThemePreference::System => ("monitor", tr("theme.system")),
            ThemePreference::Light => ("sun", tr("theme.light")),
            ThemePreference::Dark => ("moon", tr("theme.dark")),
        };
        let theme_tip = format!("{}: {t_label}", tr("theme.title"));
        let theme_resp = icons::button(ui, t_icon, 28.0, false, &theme_tip);
        if theme_resp.clicked() {
            actions.push(TitleBarAction::CycleTheme);
        }
        let mut cur_pref = self.theme_pref;
        egui::Popup::context_menu(&theme_resp).id(theme_resp.id.with("theme_menu")).show(|ui| {
            ui.set_min_width(170.0);
            for (pref, lbl) in [
                (ThemePreference::System, tr("theme.system")),
                (ThemePreference::Light, tr("theme.light")),
                (ThemePreference::Dark, tr("theme.dark")),
            ] {
                if ui.add(widgets::radio_value(&mut cur_pref, pref, lbl)).clicked() {
                    actions.push(TitleBarAction::SetTheme(pref));
                    ui.close();
                }
            }
        });
    }

    fn ui_language_button(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<TitleBarAction>) {
        let cur_lang = self.ui_language;
        let lang_tip = format!("{}: {}", tr("ui.language_tip"), cur_lang.native_name());
        let (lang_rect, lang_btn_resp) = ui.allocate_exact_size(vec2(32.0, 28.0), Sense::click());
        let lang_press = theme::Press::track(ui, &lang_btn_resp);
        let lang_fill = lang_press.wash_color(t, false);
        if lang_fill.a() > 0 {
            ui.painter().rect_filled(lang_rect, CornerRadius::same(t.radius), lang_fill);
        }
        ui.painter().text(
            lang_rect.translate(lang_press.offset()).center(),
            Align2::CENTER_CENTER,
            cur_lang.short_label(),
            theme::semibold(12.0),
            t.icon,
        );
        let lang_btn_resp = theme::hand(lang_btn_resp).on_hover_text(lang_tip);

        let ui_menu_row_w = I18nService::supported()
            .iter()
            .map(|l| {
                let lbl = format!("{} ({})", l.native_name(), l.short_label());
                ui.fonts_mut(|f| f.layout_no_wrap(lbl, theme::semibold(12.5), t.text).size().x) + 34.0
            })
            .fold(120.0_f32, f32::max)
            .ceil();

        egui::Popup::menu(&lang_btn_resp)
            .id(lang_btn_resp.id.with("ui_lang_menu"))
            .align(egui::RectAlign::BOTTOM_END)
            .width(ui_menu_row_w)
            .gap(4.0)
            .show(|ui| {
                ui.set_min_width(ui_menu_row_w);
                ui.set_max_width(ui_menu_row_w);
                ui.spacing_mut().item_spacing.y = 2.0;
                for &lang in I18nService::supported() {
                    let is_active = lang == cur_lang;
                    let (r, resp) = ui.allocate_exact_size(vec2(ui_menu_row_w, 24.0), Sense::click());
                    if is_active {
                        ui.painter().rect_filled(r, CornerRadius::same(4), t.accent_soft);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(4), t.hover);
                    }
                    let fg = if is_active { t.accent_text } else { t.text };
                    let font = if is_active { theme::semibold(12.5) } else { theme::regular(12.5) };
                    let label = format!("{} ({})", lang.native_name(), lang.short_label());
                    widgets::row_text(ui, pos2(r.left() + 8.0, r.center().y), label, font, fg, r.width() - 30.0);
                    if is_active {
                        icons::paint(
                            ui,
                            Rect::from_center_size(pos2(r.right() - 14.0, r.center().y), vec2(14.0, 14.0)),
                            "check",
                            13.5,
                            t.accent_text,
                        );
                    }
                    if theme::hand(resp).clicked() {
                        actions.push(TitleBarAction::SetUiLanguage(lang));
                        ui.close();
                    }
                }
            });
    }

    fn ui_auto_discovery_button(&self, ui: &mut egui::Ui, actions: &mut Vec<TitleBarAction>) {
        let auto_active = self.auto_detect_language;
        let auto_tip =
            if auto_active { tr("top_title.auto_lang_discovery_enabled") } else { tr("top_title.auto_lang_discovery_disabled") };
        let auto_resp = icons::button(ui, "sparkles", 28.0, auto_active, auto_tip);
        if auto_resp.clicked() {
            actions.push(TitleBarAction::ToggleAutoDetect(!auto_active));
        }
        let mut cur_auto = self.auto_detect_language;
        egui::Popup::context_menu(&auto_resp).id(auto_resp.id.with("auto_lang_ctx")).show(|ui| {
            ui.set_min_width(260.0);
            let tip_en = tr("top_title.auto_lang_discovery_enabled");
            let tip_dis = tr("top_title.auto_lang_discovery_disabled");
            if ui.add(widgets::radio_value(&mut cur_auto, true, tip_en)).clicked() {
                actions.push(TitleBarAction::ToggleAutoDetect(true));
                ui.close();
            }
            if ui.add(widgets::radio_value(&mut cur_auto, false, tip_dis)).clicked() {
                actions.push(TitleBarAction::ToggleAutoDetect(false));
                ui.close();
            }
        });
    }
}

impl PgsApp {
    pub fn set_theme_preference(&mut self, ctx: &egui::Context, pref: ThemePreference) {
        self.theme_pref = pref;
        self.preferences.theme = pref.code().to_string();
        let sys_dark = ctx.input(|i| i.raw.system_theme).map(|t| t == egui::Theme::Dark);
        self.theme_kind = pref.resolve(sys_dark, self.theme_kind);
        theme::apply(ctx, self.theme_kind);
        self.save_preferences();
    }

    pub fn set_ui_language(&mut self, lang: UiLanguage) {
        I18nService::set(lang);
        if self.preferences.ui_language != lang {
            self.preferences.ui_language = lang;
            self.save_preferences();
        }
    }

    pub fn set_auto_detect_language(&mut self, enabled: bool) {
        if self.preferences.auto_detect_language == enabled {
            return;
        }
        self.preferences.auto_detect_language = enabled;
        self.save_preferences();
        if !enabled {
            let active_probe_ids: Vec<usize> = self.items.iter().filter(|it| it.probing_language).map(|it| it.id).collect();
            for id in active_probe_ids {
                self.cancel_probe(id);
            }
        }
        let msg = if enabled { tr("toast.auto_lang_enabled") } else { tr("toast.auto_lang_disabled") };
        self.notify(msg, false);
    }

    pub fn sync_system_theme(&mut self, ctx: &egui::Context) {
        if self.theme_pref == ThemePreference::System {
            let sys_dark = ctx.input(|i| i.raw.system_theme).map(|t| t == egui::Theme::Dark);
            let resolved = self.theme_pref.resolve(sys_dark, self.theme_kind);
            if resolved != self.theme_kind {
                self.theme_kind = resolved;
                theme::apply(ctx, self.theme_kind);
            }
        }
    }

    pub fn ui_top_title(&mut self, ui: &mut egui::Ui) {
        let view = TitleBarView::new(
            self.tab,
            self.items.len(),
            self.installed_langs.len(),
            self.show_inspector,
            self.theme_pref,
            self.preferences.ui_language,
            self.preferences.auto_detect_language,
        )
        .with_capabilities(self.capabilities);
        let mut actions = Vec::new();
        view.show(ui, &mut actions);
        let ctx = ui.ctx().clone();
        for action in actions {
            self.handle_titlebar_action(action, &ctx);
        }
    }

    pub fn handle_titlebar_action(&mut self, action: TitleBarAction, ctx: &egui::Context) {
        match action {
            TitleBarAction::SwitchTab(tab) => {
                self.tab = tab;
            }
            TitleBarAction::ToggleInspector => {
                self.show_inspector = !self.show_inspector;
                self.preferences.show_inspector = self.show_inspector;
                self.save_preferences();
            }
            TitleBarAction::CycleTheme => {
                self.set_theme_preference(ctx, self.theme_pref.cycle());
            }
            TitleBarAction::SetTheme(pref) => {
                self.set_theme_preference(ctx, pref);
            }
            TitleBarAction::SetUiLanguage(lang) => {
                self.set_ui_language(lang);
            }
            TitleBarAction::ToggleAutoDetect(enabled) => {
                self.set_auto_detect_language(enabled);
            }
        }
    }

    /// Alias for backwards compatibility with tests and callers expecting `ui_tab_strip`.
    pub fn ui_tab_strip(&mut self, ui: &mut egui::Ui) {
        self.ui_top_title(ui);
    }
}
