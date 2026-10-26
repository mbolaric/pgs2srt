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

    pub fn add_file(&mut self, path: PathBuf, ctx: Option<&egui::Context>) -> bool {
        if !is_sup(&path) {
            return false;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        if self.items.iter().any(|item| item.input_path == canonical) {
            return false;
        }

        let id = self.next_id;
        self.next_id += 1;
        let default_lang = self.default_ocr_language().to_string();
        let custom_dir = self.get_effective_custom_output_dir();
        let (item, needs_auto_probe) = if self.preferences.auto_detect_language {
            let item = SubtitleItem::new(id, canonical.clone(), &default_lang, custom_dir);
            let needs_probe = item.detected_language.is_none() && self.installed_langs.len() > 1;
            (item, needs_probe)
        } else {
            let item = SubtitleItem::new_with_fixed_language(id, canonical.clone(), &default_lang, custom_dir);
            (item, false)
        };

        self.items.push(item);
        if self.selected_id.is_none() {
            self.selected_id = Some(id);
        }

        let sample_handle = spawn_sample_loader(id, canonical, self.tx.clone(), ctx.cloned());
        self.track_worker(sample_handle);
        if needs_auto_probe {
            self.start_language_probe(id, false, ctx);
        }
        true
    }

    pub(crate) fn track_worker(&mut self, handle: Option<std::thread::JoinHandle<()>>) {
        self.workers.track(handle);
    }

    /// Signals all in-flight conversion and auto-detection threads to stop and waits for them
    /// to release their `TessBaseAPI` instances before process teardown (`~ObjectCache()`).
    pub fn shutdown_workers(&mut self) {
        self.workers.shutdown();
    }

    pub fn add_folder(&mut self, folder: &Path, ctx: Option<&egui::Context>) -> usize {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return 0;
        };
        let mut paths: Vec<PathBuf> =
            entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && is_sup(p)).collect();
        paths.sort();

        let mut added = 0;
        for p in paths {
            if self.add_file(p, ctx) {
                added += 1;
            }
        }
        added
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

        let parent = item.output_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("{} {}: {e}", I18nService::get("app.cannot_create_output_folder"), parent.display()))?;
        }

        if !self.preferences.allow_overwrite && item.output_path.exists() {
            return Err(format!("{}: {}", I18nService::get("files.output_already_exists"), item.output_path.display()));
        }

        Ok(())
    }

    pub fn start_item(&mut self, id: usize, ctx: Option<&egui::Context>) {
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

        let input_path = item.input_path.to_string_lossy().to_string();
        let language = item.language.clone();
        let output_path = item.output_path.to_string_lossy().to_string();

        let handle = spawn_conversion(id, run, input_path, language, output_path, self.tx.clone(), cancel, ctx.cloned());
        if handle.is_none() {
            self.workers.active_cancellations.remove(&id);
            if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                item.status = ItemStatus::Failed("Failed to spawn worker thread".to_string());
            }
        }
        self.track_worker(handle);
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
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
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

    pub(super) fn handle_batch_concurrency(&mut self, ctx: &egui::Context) {
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
                self.start_item(id, Some(ctx));
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
                WorkerMessage::Completed { id, run, cues_count } => self.on_worker_completed(id, run, cues_count),
                WorkerMessage::Failed { id, run, error } => self.on_worker_failed(id, run, error),
                WorkerMessage::SampleLoaded { id, sample } => self.on_worker_sample_loaded(id, sample),
                WorkerMessage::LanguageProbed { id, detected, manual } => {
                    self.on_worker_language_probed(id, detected, manual);
                }
            }
        }
        self.workers.handles.retain(|h| !h.is_finished());
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

    fn on_worker_completed(&mut self, id: usize, run: u64, cues_count: usize) {
        if !self.workers.is_current_run(id, run) {
            return;
        }
        self.workers.active_cancellations.remove(&id);
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.total_cues_hint = Some(cues_count);
            item.status = ItemStatus::Completed { cues_count };
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

    fn on_worker_language_probed(&mut self, id: usize, detected: Option<String>, manual: bool) {
        let was_active_probe = self.workers.probe_cancellations.remove(&id).is_some();
        if !was_active_probe {
            return;
        }
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
                if manual {
                    let label = format_language_label(&code, ui_lang);
                    self.toast = Some((format!("{} {label}", tr("toast.detected_lang")), false, 0.0));
                }
            } else if manual {
                self.toast = Some((tr("toast.detect_none").to_string(), true, 0.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        app.tx.send(WorkerMessage::Completed { id, run: new_run, cues_count: 7 }).unwrap();
        app.handle_incoming_messages();
        assert_eq!(app.items[0].status, ItemStatus::Completed { cues_count: 7 });
        assert!(!app.workers.active_cancellations.contains_key(&id));
    }
}
