use super::*;

#[derive(Default)]
pub struct WorkerPool {
    pub active_cancellations: HashMap<usize, (u64, Arc<AtomicBool>)>,
    pub probe_cancellations: HashMap<usize, (u64, Arc<AtomicBool>)>,
    pub(crate) queued_probes: VecDeque<(usize, u64, bool)>,
    pub(crate) running_probe: Option<u64>,
    next_run: u64,
}

impl WorkerPool {
    pub fn next_run(&mut self) -> u64 {
        self.next_run += 1;
        self.next_run
    }

    pub fn is_current_run(&self, id: usize, run: u64) -> bool {
        self.active_cancellations.get(&id).is_some_and(|(active, _)| *active == run)
    }

    pub fn is_current_probe(&self, id: usize, run: u64) -> bool {
        self.probe_cancellations.get(&id).is_some_and(|(active, _)| *active == run)
    }

    pub fn cancel_all(&mut self) {
        for (_, (_, flag)) in self.active_cancellations.drain().chain(self.probe_cancellations.drain()) {
            flag.store(true, Ordering::Relaxed);
        }
        self.queued_probes.clear();
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

pub struct PgsApp {
    pub theme_pref: ThemePreference,
    pub theme_kind: ThemeKind,
    pub preferences: Preferences,
    pub tab: WorkspaceTab,
    pub show_inspector: bool,

    pub items: Vec<SubtitleItem>,
    pub(crate) next_id: usize,
    pub selected_id: Option<usize>,

    pub(crate) tx: Sender<WorkerMessage>,
    pub(crate) rx: Receiver<WorkerMessage>,
    pub(crate) repaint: Option<egui::Context>,
    pub workers: WorkerPool,
    pub is_batch_running: bool,

    // Subtitle preview state (per-item sample cache + active live preview texture)
    pub(crate) sample_previews: HashMap<usize, PreviewData>,
    pub live_preview: Option<PreviewData>,
    pub(crate) preview_seq_counter: u64,
    pub(crate) preview_texture: Option<(u64, TextureHandle)>,

    // OCR languages & catalog
    pub installed_langs: Vec<String>,
    pub catalog: Vec<ModelInfo>,
    pub tessdata_dir: Option<PathBuf>,
    pub language_search: String,
    pub model_filter: ModelFilter,
    pub popup_lang_search: String,
    pub popup_highlight_idx: usize,
    pub(crate) scroll_to_selected_row: bool,
    pub(crate) downloading_lang: Option<String>,

    // Transient feedback toast: (message, is_error, start_time)
    pub(crate) toast: Option<(String, bool, f64)>,

    // Saved position of the inspector panel splitter to debounce disk writes
    pub(crate) last_saved_splitter: f32,

    pub capabilities: Capabilities,
    // Last field: dropped after `workers` has cancelled its jobs.
    pub(crate) platform: Box<dyn Platform>,
}

impl Default for PgsApp {
    fn default() -> Self {
        Self::with_platform(Box::new(Headless))
    }
}

impl PgsApp {
    pub fn with_platform(platform: Box<dyn Platform>) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let preferences = platform.load_preferences();
        I18nService::set(preferences.ui_language);

        let theme_pref = ThemePreference::from_code(&preferences.theme);
        let theme_kind = theme_pref.resolve(None, ThemeKind::Light);

        let mut installed = platform.backend().installed_languages().unwrap_or_else(|e| {
            log::warn!("Cannot list installed OCR languages: {e}");
            Vec::new()
        });
        if installed.is_empty() {
            installed.push("eng".to_string());
        }

        let show_inspector = preferences.show_inspector;
        let last_saved_splitter = preferences.splitter_position;

        Self {
            theme_pref,
            theme_kind,
            preferences,
            tab: WorkspaceTab::Queue,
            show_inspector,
            items: Vec::new(),
            next_id: 1,
            selected_id: None,
            tx,
            rx,
            repaint: None,
            workers: WorkerPool::default(),
            is_batch_running: false,
            sample_previews: HashMap::new(),
            live_preview: None,
            preview_seq_counter: 1,
            preview_texture: None,
            installed_langs: installed,
            catalog: pgs_ocr::catalog::models(),
            tessdata_dir: platform.backend().models_location(),
            language_search: String::new(),
            model_filter: ModelFilter::All,
            popup_lang_search: String::new(),
            popup_highlight_idx: 0,
            scroll_to_selected_row: false,
            downloading_lang: None,
            toast: None,
            last_saved_splitter,
            capabilities: platform.capabilities(),
            platform,
        }
    }

    pub fn save_preferences(&self) {
        self.platform.save_preferences(&self.preferences);
    }

    pub(crate) fn mailbox(&self) -> Mailbox {
        Mailbox::new(self.tx.clone(), self.repaint.clone())
    }

    pub fn new(cc: &eframe::CreationContext<'_>, platform: Box<dyn Platform>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let mut app = Self::with_platform(platform);
        app.repaint = Some(cc.egui_ctx.clone());
        let sys_dark = cc.egui_ctx.input(|i| i.raw.system_theme).map(|t| t == egui::Theme::Dark);
        app.theme_kind = app.theme_pref.resolve(sys_dark, ThemeKind::Light);
        theme::apply(&cc.egui_ctx, app.theme_kind);
        app
    }
}
