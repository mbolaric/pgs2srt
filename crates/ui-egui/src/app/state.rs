use super::*;

#[derive(Default)]
pub struct WorkerPool {
    pub active_cancellations: HashMap<usize, (u64, Arc<AtomicBool>)>,
    pub probe_cancellations: HashMap<usize, Arc<AtomicBool>>,
    pub handles: Vec<std::thread::JoinHandle<()>>,
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

    pub fn track(&mut self, handle: Option<std::thread::JoinHandle<()>>) {
        self.handles.retain(|h| !h.is_finished());
        if let Some(h) = handle {
            self.handles.push(h);
        }
    }

    pub fn shutdown(&mut self) {
        for (_, (_, flag)) in self.active_cancellations.drain() {
            flag.store(true, Ordering::Relaxed);
        }
        for (_, flag) in self.probe_cancellations.drain() {
            flag.store(true, Ordering::Relaxed);
        }
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shutdown();
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
    pub(crate) download_rx: Option<std::sync::mpsc::Receiver<pgs_core::error::Result<String>>>,

    // Transient feedback toast: (message, is_error, start_time)
    pub(crate) toast: Option<(String, bool, f64)>,

    // Saved position of the inspector panel splitter to debounce disk writes
    pub(crate) last_saved_splitter: f32,
}

impl Default for PgsApp {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let preferences = Preferences::load().unwrap_or_else(|e| {
            log::error!("Cannot load settings, using defaults: {e}");
            Preferences::default()
        });
        I18nService::set(preferences.ui_language);

        let theme_pref = ThemePreference::from_code(&preferences.theme);
        let theme_kind = theme_pref.resolve(None, ThemeKind::Light);

        let mut installed = installed_languages().unwrap_or_else(|e| {
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
            workers: WorkerPool::default(),
            is_batch_running: false,
            sample_previews: HashMap::new(),
            live_preview: None,
            preview_seq_counter: 1,
            preview_texture: None,
            installed_langs: installed,
            catalog: catalog_models(),
            tessdata_dir: tessdata_directory().ok(),
            language_search: String::new(),
            model_filter: ModelFilter::All,
            popup_lang_search: String::new(),
            popup_highlight_idx: 0,
            scroll_to_selected_row: false,
            downloading_lang: None,
            download_rx: None,
            toast: None,
            last_saved_splitter,
        }
    }
}

impl PgsApp {
    pub fn save_preferences(&self) {
        if let Err(e) = self.preferences.save() {
            log::error!("Cannot save settings: {e}");
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let mut app = Self::default();
        let sys_dark = cc.egui_ctx.input(|i| i.raw.system_theme).map(|t| t == egui::Theme::Dark);
        app.theme_kind = app.theme_pref.resolve(sys_dark, ThemeKind::Light);
        theme::apply(&cc.egui_ctx, app.theme_kind);
        app
    }
}
