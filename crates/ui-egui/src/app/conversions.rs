use super::*;

struct WorkerCueProgress {
    id: usize,
    run: u64,
    cue: usize,
    total: usize,
    start_ts: u32,
    stop_ts: u32,
    text: String,
    rgba: Option<(usize, usize, Arc<[u8]>)>,
}

impl PgsApp {
    pub fn get_effective_custom_output_dir(&self) -> Option<&Path> {
        if self.preferences.use_custom_output_dir {
            self.preferences.custom_output_dir.as_deref()
        } else {
            None
        }
    }

    pub fn update_all_output_paths(&mut self) {
        let custom_dir = if self.preferences.use_custom_output_dir { self.preferences.custom_output_dir.clone() } else { None };
        for item in &mut self.items {
            item.recompute_output_path(custom_dir.as_deref());
        }
    }

    /// Queues the `.sup` inputs that are not queued yet; returns how many were added.
    pub fn add_inputs(&mut self, inputs: impl IntoIterator<Item = Input>) -> usize {
        inputs.into_iter().map(|input| self.add_input(input)).filter(|&added| added).count()
    }

    pub fn add_input(&mut self, input: Input) -> bool {
        if !is_sup(input.path()) || self.items.iter().any(|item| item.input == input) {
            return false;
        }

        let id = self.next_id;
        self.next_id += 1;
        let default_lang = self.default_ocr_language().to_string();
        let custom_dir = self.get_effective_custom_output_dir();
        let (item, needs_auto_probe) = if self.preferences.auto_detect_language {
            let item = SubtitleItem::new(id, input, &default_lang, custom_dir);
            let needs_probe = item.detected_language.is_none();
            (item, needs_probe)
        } else {
            let item = SubtitleItem::new_with_fixed_language(id, input, &default_lang, custom_dir);
            (item, false)
        };

        self.items.push(item);
        if self.selected_id.is_none() {
            self.selected_id = Some(id);
        }

        self.request_sample_preview(id);

        if needs_auto_probe {
            self.start_language_probe(id, false);
        }
        true
    }

    pub(crate) fn pick_inputs(&self, folder: bool) {
        self.platform.pick_inputs(folder, self.mailbox());
    }

    fn on_inputs_picked(&mut self, inputs: Vec<Input>, folder: bool) {
        let added = self.add_inputs(inputs);
        if added > 0 {
            self.tab = WorkspaceTab::Queue;
            self.notify(format!("{} ({added})", tr("toast.added_files")), false);
        } else if folder {
            self.notify(tr("toast.no_sup_in_folder"), true);
        }
    }

    pub(crate) fn pick_destination(&mut self, id: usize) {
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else {
            return;
        };
        if let Some(chosen) = self.platform.pick_output_file(&item.output_path) {
            item.output_path = chosen;
            item.custom_output_override = true;
        }
    }

    /// Signals all in-flight conversions and language probes to stop and waits for the platform's
    /// workers to release their OCR engines before process teardown.
    pub fn shutdown_workers(&mut self) {
        self.workers.cancel_all();
        self.platform.shutdown();
    }

    /// Validates language, model availability, output directory, and overwrite policy before spawning OCR.
    pub(crate) fn preflight_item(&self, item: &SubtitleItem) -> Result<(), String> {
        validate_language(&item.language).map_err(|e| e.to_string())?;
        if !self.installed_langs.iter().any(|l| l == &item.language) {
            return Err(format!(
                "{}: '{}' — {}",
                I18nService::get("error.ocr_failed"),
                item.language,
                I18nService::get("error.check_that_the_selected_language")
            ));
        }
        validate_output_name(&item.output_path).map_err(|e| e.to_string())?;
        self.platform.check_output(&item.output_path, self.preferences.allow_overwrite)
    }

    pub fn start_item(&mut self, id: usize) {
        let Some(idx) = self.items.iter().position(|i| i.id == id) else {
            return;
        };
        if matches!(self.items[idx].status, ItemStatus::Converting { .. }) {
            return;
        }

        if let Err(err) = self.preflight_item(&self.items[idx]) {
            self.items[idx].status = ItemStatus::Failed(err);
            return;
        }

        self.cancel_probe(id);

        let item = &mut self.items[idx];
        item.srt = None;
        item.downloaded = false;
        item.status = ItemStatus::Converting {
            cue: 0,
            total: item.total_cues_hint.unwrap_or(0),
            start_ts: 0,
            stop_ts: 0,
            last_text: String::new(),
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let run = self.workers.next_run();
        self.workers.active_cancellations.insert(id, (run, cancel.clone()));

        let job = conversion_job(id, run, item.input.clone(), item.language.clone(), cancel, self.mailbox());
        if let Err(e) = self.platform.spawn(job) {
            log::error!("Failed to start conversion: {e}");
            self.workers.active_cancellations.remove(&id);
            if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                item.status = ItemStatus::Failed(format!("Failed to spawn worker thread: {e}"));
            }
        }
    }

    pub fn cancel_item(&mut self, id: usize) {
        if let Some((_, cancel)) = self.workers.active_cancellations.remove(&id) {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if matches!(item.status, ItemStatus::Converting { .. }) {
                item.status = ItemStatus::Failed(tr("status.batch_cancelled").to_string());
            }
        }
    }

    pub fn remove_item(&mut self, id: usize) {
        self.cancel_probe(id);
        self.cancel_item(id);
        self.sample_previews.remove(&id);
        if self.live_preview.as_ref().is_some_and(|p| p.item_id == id) {
            self.live_preview = None;
        }
        let prev_pos = self.items.iter().position(|i| i.id == id);
        self.items.retain(|i| i.id != id);
        if self.selected_id == Some(id) {
            self.selected_id = prev_pos.and_then(|pos| self.items.get(pos.min(self.items.len().saturating_sub(1)))).map(|i| i.id);
        }
    }

    /// Starts batch conversion after checking for duplicate target `.srt` paths across queued items.
    pub fn start_batch(&mut self) {
        if self.preferences.use_custom_output_dir && self.preferences.custom_output_dir.is_none() {
            if let Some(folder) = self.platform.pick_output_folder() {
                self.preferences.custom_output_dir = Some(folder);
                self.update_all_output_paths();
                self.save_preferences();
            } else {
                return;
            }
        }

        // Reset Failed items back to Pending so Start Batch retries them alongside Pending items.
        for item in &mut self.items {
            if matches!(item.status, ItemStatus::Failed(_)) {
                item.status = ItemStatus::Pending;
            }
        }

        // Detect duplicate output paths across pending items (parity with pgs2srt-tui).
        self.detect_duplicate_output_paths();

        if self.items.iter().any(|i| matches!(i.status, ItemStatus::Pending)) {
            self.is_batch_running = true;
        }
    }

    fn detect_duplicate_output_paths(&mut self) {
        let mut output_counts: HashMap<PathBuf, usize> = HashMap::new();
        for item in self.items.iter().filter(|i| matches!(i.status, ItemStatus::Pending)) {
            *output_counts.entry(item.output_path.clone()).or_insert(0) += 1;
        }

        for item in self.items.iter_mut().filter(|i| matches!(i.status, ItemStatus::Pending)) {
            if output_counts.get(&item.output_path).copied().unwrap_or(0) > 1 {
                item.status = ItemStatus::Failed(format!(
                    "{}: {}. {}",
                    I18nService::get("app.multiple_inputs_have_the_same"),
                    item.output_path.display(),
                    I18nService::get("app.convert_them_separately_with_different")
                ));
            }
        }
    }

    pub fn cancel_batch(&mut self) {
        self.is_batch_running = false;
        let active_ids: Vec<usize> = self.workers.active_cancellations.keys().copied().collect();
        for id in active_ids {
            self.cancel_item(id);
        }
        self.notify(tr("status.batch_cancelled"), false);
    }

    pub(super) fn handle_batch_concurrency(&mut self) {
        if !self.is_batch_running {
            return;
        }
        if self.downloading_lang.is_some() {
            return;
        }

        let active_count = self.items.iter().filter(|i| matches!(i.status, ItemStatus::Converting { .. })).count();
        let max_parallel = self.preferences.parallel_files.clamp(1, 10);

        if active_count < max_parallel {
            let slots = max_parallel - active_count;
            let pending_ids: Vec<usize> =
                self.items.iter().filter(|i| matches!(i.status, ItemStatus::Pending)).take(slots).map(|i| i.id).collect();

            for id in pending_ids {
                self.start_item(id);
            }
        }

        let any_converting = self.items.iter().any(|i| matches!(i.status, ItemStatus::Converting { .. }));
        let any_pending = self.items.iter().any(|i| matches!(i.status, ItemStatus::Pending));

        if !any_converting && !any_pending {
            self.is_batch_running = false;
            self.notify(tr("status.batch_done"), false);
        }
    }

    pub(super) fn handle_incoming_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMessage::Started { id, run, total } => self.on_worker_started(id, run, total),
                WorkerMessage::CueProgress { id, run, cue, total, start_ts, stop_ts, text, rgba } => {
                    self.on_worker_cue_progress(WorkerCueProgress { id, run, cue, total, start_ts, stop_ts, text, rgba });
                }
                WorkerMessage::Completed { id, run, cues_count, srt } => self.on_worker_completed(id, run, cues_count, srt),
                WorkerMessage::Failed { id, run, error } => self.on_worker_failed(id, run, error),
                WorkerMessage::SampleLoaded { id, sample } => self.on_worker_sample_loaded(id, sample),
                WorkerMessage::LanguageProbed { id, run, detected, manual } => {
                    self.on_worker_language_probed(id, run, detected, manual);
                }
                WorkerMessage::LanguageInstalled { code, result } => self.on_language_installed(code, result),
                WorkerMessage::InputsPicked { inputs, folder } => self.on_inputs_picked(inputs, folder),
            }
        }
    }

    fn on_worker_started(&mut self, id: usize, run: u64, total: usize) {
        if !self.workers.is_current_run(id, run) {
            return;
        }
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.total_cues_hint = Some(total);
            if let ItemStatus::Converting { total: ref mut t, .. } = item.status {
                *t = total;
            }
        }
    }

    fn on_worker_cue_progress(&mut self, progress: WorkerCueProgress) {
        let WorkerCueProgress { id, run, cue, total, start_ts, stop_ts, text, rgba } = progress;
        if !self.workers.is_current_run(id, run) {
            return;
        }
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.total_cues_hint = Some(total);
            item.status = ItemStatus::Converting { cue, total, start_ts, stop_ts, last_text: text.clone() };
        }
        if self.selected_id == Some(id) || self.selected_id.is_none() {
            self.preview_seq_counter += 1;
            let preview = PreviewData { item_id: id, cue, total, start_ts, stop_ts, text, rgba, seq: self.preview_seq_counter };
            self.sample_previews.insert(id, preview.clone());
            self.live_preview = Some(preview);
        }
    }

    fn on_worker_completed(&mut self, id: usize, run: u64, cues_count: usize, srt: Vec<u8>) {
        if !self.workers.is_current_run(id, run) {
            return;
        }
        self.workers.active_cancellations.remove(&id);
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else {
            return;
        };
        item.total_cues_hint = Some(cues_count);
        if !self.capabilities.writes_files {
            item.srt = Some(srt.into());
            item.status = ItemStatus::Completed { cues_count };
            return;
        }
        item.status = match self.platform.deliver_srts(&[(&item.output_path, &srt)]) {
            Ok(()) => ItemStatus::Completed { cues_count },
            Err(error) => {
                log::error!("Cannot save {}: {error}", item.output_path.display());
                ItemStatus::Failed(error)
            }
        };
    }

    /// Delivers the kept SRT of `id`, or of every finished item, in one download.
    pub fn download_srts(&mut self, id: Option<usize>) {
        let picked = |item: &SubtitleItem| item.srt.is_some() && id.is_none_or(|id| item.id == id);
        let srts: Vec<(&Path, &[u8])> =
            self.items.iter().filter(|i| picked(i)).filter_map(|i| Some((i.output_path.as_path(), i.srt.as_deref()?))).collect();
        if srts.is_empty() {
            return;
        }
        if let Err(error) = self.platform.deliver_srts(&srts) {
            log::error!("Cannot download subtitles: {error}");
            self.notify(error, true);
            return;
        }
        for item in self.items.iter_mut().filter(|i| picked(i)) {
            item.downloaded = true;
        }
    }

    fn on_worker_failed(&mut self, id: usize, run: u64, error: String) {
        if !self.workers.is_current_run(id, run) {
            return;
        }
        self.workers.active_cancellations.remove(&id);
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = ItemStatus::Failed(error);
        }
    }

    fn on_worker_sample_loaded(&mut self, id: usize, sample: Option<pgs_core::SampleCue>) {
        let Some(sample) = sample else {
            return;
        };
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.total_cues_hint = Some(sample.total_cues);
        }
        self.preview_seq_counter += 1;
        let preview = PreviewData {
            item_id: id,
            cue: sample.cue_index,
            total: sample.total_cues,
            start_ts: sample.start_ts,
            stop_ts: sample.stop_ts,
            text: String::new(),
            rgba: Some((sample.width, sample.height, Arc::<[u8]>::from(sample.rgba))),
            seq: self.preview_seq_counter,
        };
        self.sample_previews.entry(id).or_insert(preview);
    }

    fn on_worker_language_probed(&mut self, id: usize, run: u64, detected: Option<String>, manual: bool) {
        if self.workers.running_probe == Some(run) {
            self.workers.running_probe = None;
        }
        self.apply_probe_result(id, run, detected, manual);
        self.start_next_probe();
    }

    pub(super) fn apply_probe_result(&mut self, id: usize, run: u64, detected: Option<String>, manual: bool) {
        if !self.workers.is_current_probe(id, run) {
            return;
        }
        self.workers.probe_cancellations.remove(&id);
        let custom_dir = self.get_effective_custom_output_dir().map(Path::to_path_buf);
        let ui_lang = self.preferences.ui_language;
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.probing_language = false;
            if let Some(code) = detected {
                item.detected_language = Some(code.clone());
                // Do not overwrite an already-converting item's active OCR language.
                if !matches!(item.status, ItemStatus::Converting { .. }) {
                    item.update_language(code.clone(), custom_dir.as_deref());
                }
                let installed = self.installed_langs.contains(&code);
                if manual || !installed {
                    let label = format_language_label(&code, ui_lang);
                    let mut message = format!("{} {label}", tr("toast.detected_lang"));
                    if !installed {
                        message = format!("{message} ({})", tr("badge.missing_model"));
                    }
                    self.toast = Some((message, !installed, 0.0));
                }
            } else if manual {
                self.toast = Some((tr("toast.detect_none").to_string(), true, 0.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;

    use egui::DroppedFileHandle;
    use pgs_ocr::OcrBackend;

    use super::*;
    use crate::platform::Job;

    fn converting() -> ItemStatus {
        ItemStatus::Converting { cue: 0, total: 0, start_ts: 0, stop_ts: 0, last_text: String::new() }
    }

    #[test]
    fn late_messages_from_a_cancelled_run_cannot_clobber_the_retried_run() {
        let mut app = PgsApp::default();
        let id = 1;
        app.items.push(SubtitleItem::new_with_fixed_language(id, PathBuf::from("/films/a.sup"), "eng", None));

        // Run 1 starts and is cancelled; Retry immediately registers run 2 while run 1's thread is still unwinding.
        let old_run = app.workers.next_run();
        app.workers.active_cancellations.insert(id, (old_run, Arc::new(AtomicBool::new(false))));
        app.items[0].status = converting();
        app.cancel_item(id);
        let new_run = app.workers.next_run();
        app.workers.active_cancellations.insert(id, (new_run, Arc::new(AtomicBool::new(false))));
        app.items[0].status = converting();

        let late_cue = WorkerMessage::CueProgress {
            id,
            run: old_run,
            cue: 5,
            total: 9,
            start_ts: 0,
            stop_ts: 0,
            text: "old".into(),
            rgba: None,
        };
        app.tx.send(late_cue).unwrap();
        app.tx.send(WorkerMessage::Failed { id, run: old_run, error: "Conversion was cancelled".into() }).unwrap();
        app.handle_incoming_messages();

        assert_eq!(app.items[0].status, converting(), "stale run must not touch the retried item");
        assert!(app.workers.is_current_run(id, new_run), "retried run must stay cancellable");

        app.tx.send(WorkerMessage::Completed { id, run: new_run, cues_count: 7, srt: Vec::new() }).unwrap();
        app.handle_incoming_messages();
        assert_eq!(app.items[0].status, ItemStatus::Completed { cues_count: 7 });
        assert!(!app.workers.active_cancellations.contains_key(&id));
    }

    /// Keeps SRTs for download like the web platform and records each delivery.
    struct Downloads(Rc<RefCell<Vec<Vec<PathBuf>>>>);

    impl Platform for Downloads {
        fn capabilities(&self) -> Capabilities {
            Capabilities::NONE
        }

        fn backend(&self) -> &dyn OcrBackend {
            &Headless
        }

        fn spawn(&self, _job: Job) -> io::Result<()> {
            Err(io::ErrorKind::Unsupported.into())
        }

        fn pick_inputs(&self, _folder: bool, _mailbox: Mailbox) {}

        fn take_dropped_files(&self, _files: Vec<DroppedFileHandle>, _mailbox: Mailbox) {}

        fn deliver_srts(&self, srts: &[(&Path, &[u8])]) -> Result<(), String> {
            self.0.borrow_mut().push(srts.iter().map(|(output, _)| output.to_path_buf()).collect());
            Ok(())
        }

        fn load_preferences(&self) -> Preferences {
            Preferences::default()
        }

        fn save_preferences(&self, _preferences: &Preferences) {}
    }

    #[test]
    fn without_file_access_srts_are_kept_until_downloaded() {
        let delivered = Rc::default();
        let mut app = PgsApp::with_platform(Box::new(Downloads(Rc::clone(&delivered))));
        for id in [1, 2] {
            app.items.push(SubtitleItem::new_with_fixed_language(id, PathBuf::from(format!("/films/{id}.sup")), "eng", None));
            let run = app.workers.next_run();
            app.workers.active_cancellations.insert(id, (run, Arc::new(AtomicBool::new(false))));
            app.tx.send(WorkerMessage::Completed { id, run, cues_count: 1, srt: b"1\n".to_vec() }).unwrap();
        }
        app.handle_incoming_messages();
        assert!(delivered.borrow().is_empty());
        assert!(app.items.iter().all(SubtitleItem::awaits_download));

        app.download_srts(Some(2));
        assert_eq!(app.items.iter().map(SubtitleItem::awaits_download).collect::<Vec<_>>(), [true, false]);

        app.download_srts(None);
        let (first, second) = (PathBuf::from("/films/1.eng.srt"), PathBuf::from("/films/2.eng.srt"));
        assert_eq!(*delivered.borrow(), [vec![second.clone()], vec![first, second]]);
        assert!(!app.items.iter().any(SubtitleItem::awaits_download));
    }
}
