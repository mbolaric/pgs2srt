use std::path::Path;

use super::super::*;

/// Strongly typed user interactions emitted by the desktop command toolbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolbarAction {
    AddFiles,
    AddFolder,
    ToggleCustomOutputDir(bool),
    PickCustomFolder,
    DownloadAll,
    StartBatch,
    CancelBatch,
    SetParallelWorkers(usize),
}

/// Stateless presentation view for the command toolbar.
#[derive(Debug, Clone, Copy)]
pub struct ToolbarView<'a> {
    pub is_batch_running: bool,
    pub runnable_count: usize,
    pub use_custom_output_dir: bool,
    pub custom_output_dir: Option<&'a Path>,
    pub parallel_files: usize,
    pub downloadable_count: usize,
    pub capabilities: Capabilities,
}

impl<'a> ToolbarView<'a> {
    pub fn new(
        is_batch_running: bool,
        runnable_count: usize,
        use_custom_output_dir: bool,
        custom_output_dir: Option<&'a Path>,
        parallel_files: usize,
    ) -> Self {
        Self {
            is_batch_running,
            runnable_count,
            use_custom_output_dir,
            custom_output_dir,
            parallel_files,
            downloadable_count: 0,
            capabilities: Capabilities::ALL,
        }
    }

    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_downloadable_count(mut self, downloadable_count: usize) -> Self {
        self.downloadable_count = downloadable_count;
        self
    }

    pub fn show(&self, ui: &mut egui::Ui, actions: &mut Vec<ToolbarAction>) {
        let t = Tokens::get(ui.ctx());

        egui::Panel::top("command_toolbar")
            .exact_size(COMMAND_BAR_HEIGHT)
            .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(12, 0)))
            .show(ui, |ui| {
                let bar_rect = ui.max_rect();
                ui.painter().hline(
                    bar_rect.left() - 12.0..=bar_rect.right() + 12.0,
                    bar_rect.bottom() - 0.5,
                    Stroke::new(1.0, t.divider),
                );

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    self.ui_toolbar_file_actions(ui, actions);

                    if self.capabilities.output_folder {
                        widgets::vline_divider(ui, TOOLBAR_HEIGHT);
                        self.ui_toolbar_output_options(ui, &t, actions);
                    }

                    if !self.capabilities.writes_files {
                        widgets::vline_divider(ui, TOOLBAR_HEIGHT);
                        self.ui_toolbar_download_all(ui, actions);
                    }

                    // Right-aligned Workers stepper + Primary Start/Cancel Batch button
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        self.ui_toolbar_batch_and_workers(ui, &t, actions);
                    });
                });
            });
    }

    fn ui_toolbar_file_actions(&self, ui: &mut egui::Ui, actions: &mut Vec<ToolbarAction>) {
        if widgets::ghost_button(ui, "plus", tr("action.add_files"), TOOLBAR_HEIGHT)
            .on_hover_text(tr("action.add_files_tip"))
            .clicked()
        {
            actions.push(ToolbarAction::AddFiles);
        }

        if self.capabilities.folders
            && widgets::ghost_button(ui, "folder-open", tr("action.add_folder"), TOOLBAR_HEIGHT)
                .on_hover_text(tr("action.add_folder_tip"))
                .clicked()
        {
            actions.push(ToolbarAction::AddFolder);
        }
    }

    fn ui_toolbar_download_all(&self, ui: &mut egui::Ui, actions: &mut Vec<ToolbarAction>) {
        let label = format!("{} ({})", tr("action.download_all"), self.downloadable_count);
        if ui
            .add_enabled_ui(self.downloadable_count > 0, |ui| widgets::ghost_button(ui, "download", &label, TOOLBAR_HEIGHT))
            .inner
            .on_hover_text(tr("action.download_all_tip"))
            .clicked()
        {
            actions.push(ToolbarAction::DownloadAll);
        }
    }

    fn ui_toolbar_output_options(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<ToolbarAction>) {
        ui.label(RichText::new(tr("output.label")).font(theme::medium(12.5)).color(t.text_muted));

        let mut use_custom = self.use_custom_output_dir;
        if widgets::segmented(
            ui,
            "output_mode_seg",
            &mut use_custom,
            &[(false, tr("output.source_folder")), (true, tr("output.custom_folder"))],
            TOOLBAR_HEIGHT,
        ) {
            actions.push(ToolbarAction::ToggleCustomOutputDir(use_custom));
        }

        let right_cluster_budget = 310.0;
        if self.use_custom_output_dir {
            let avail_for_folder = (ui.available_width() - right_cluster_budget).clamp(96.0, 230.0);
            let (folder_label, folder_tip) = match self.custom_output_dir {
                Some(dir) => {
                    let name = dir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| dir.display().to_string());
                    (name, dir.display().to_string())
                }
                None => (tr("output.choose_folder").to_string(), String::new()),
            };

            if widgets::dropdown_button(ui, Some("folder"), &folder_label, avail_for_folder, TOOLBAR_HEIGHT, &folder_tip)
                .clicked()
            {
                actions.push(ToolbarAction::PickCustomFolder);
            }
        }
    }

    fn ui_toolbar_batch_and_workers(&self, ui: &mut egui::Ui, t: &Tokens, actions: &mut Vec<ToolbarAction>) {
        if self.is_batch_running {
            if widgets::danger_icon_pill(ui, "stop", tr("action.cancel_batch"), TOOLBAR_HEIGHT).clicked() {
                actions.push(ToolbarAction::CancelBatch);
            }
        } else {
            let label = if self.runnable_count > 0 {
                format!("{} ({})", tr("action.start_batch"), self.runnable_count)
            } else {
                tr("action.start_batch").to_string()
            };
            if ui
                .add_enabled_ui(self.runnable_count > 0, |ui| widgets::icon_pill(ui, "play", &label, true, TOOLBAR_HEIGHT))
                .inner
                .clicked()
            {
                actions.push(ToolbarAction::StartBatch);
            }
        }

        widgets::vline_divider(ui, TOOLBAR_HEIGHT);

        let mut parallel = self.parallel_files.clamp(1, 10);
        if widgets::stepper(ui, "parallel_workers", &mut parallel, 1..=10, TOOLBAR_HEIGHT, tr("workers.tip")) {
            actions.push(ToolbarAction::SetParallelWorkers(parallel));
        }

        ui.label(RichText::new(tr("workers.label")).font(theme::medium(12.5)).color(t.text_muted));
    }
}

impl PgsApp {
    pub fn ui_command_toolbar(&mut self, ui: &mut egui::Ui) {
        let runnable_count =
            self.items.iter().filter(|i| matches!(i.status, ItemStatus::Pending | ItemStatus::Failed(_))).count();
        let view = ToolbarView::new(
            self.is_batch_running,
            runnable_count,
            self.preferences.use_custom_output_dir,
            self.preferences.custom_output_dir.as_deref(),
            self.preferences.parallel_files,
        )
        .with_capabilities(self.capabilities)
        .with_downloadable_count(self.items.iter().filter(|i| i.srt.is_some()).count());
        let mut actions = Vec::new();
        view.show(ui, &mut actions);
        for action in actions {
            self.handle_toolbar_action(action);
        }
    }

    pub fn handle_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::AddFiles => self.pick_inputs(false),
            ToolbarAction::AddFolder => self.pick_inputs(true),
            ToolbarAction::ToggleCustomOutputDir(enabled) => {
                self.preferences.use_custom_output_dir = enabled;
                if enabled && self.preferences.custom_output_dir.is_none() {
                    if let Some(folder) = self.platform.pick_output_folder() {
                        self.preferences.custom_output_dir = Some(folder);
                    } else {
                        self.preferences.use_custom_output_dir = false;
                    }
                }
                self.update_all_output_paths();
                self.save_preferences();
            }
            ToolbarAction::PickCustomFolder => {
                if let Some(folder) = self.platform.pick_output_folder() {
                    self.preferences.custom_output_dir = Some(folder);
                    self.update_all_output_paths();
                    self.save_preferences();
                }
            }
            ToolbarAction::DownloadAll => self.download_srts(None),
            ToolbarAction::StartBatch => {
                self.tab = WorkspaceTab::Queue;
                self.start_batch();
            }
            ToolbarAction::CancelBatch => {
                self.cancel_batch();
            }
            ToolbarAction::SetParallelWorkers(parallel) => {
                self.preferences.parallel_files = parallel;
                self.save_preferences();
            }
        }
    }
}
