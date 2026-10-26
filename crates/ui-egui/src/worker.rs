use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

static PROBE_LOCK: Mutex<()> = Mutex::new(());

use crossbeam_channel::Sender;
use pgs_core::language::probe_language_from_file;
use pgs_core::{extract_sample_cue, Error, PgsToSrt, ProgressReporter, SampleCue};

#[derive(Clone, Debug)]
pub enum WorkerMessage {
    Started {
        id: usize,
        run: u64,
        total: usize,
    },
    CueProgress {
        id: usize,
        run: u64,
        cue: usize,
        total: usize,
        start_ts: u32,
        stop_ts: u32,
        text: String,
        rgba: Option<(usize, usize, Arc<[u8]>)>,
    },
    Completed {
        id: usize,
        run: u64,
        cues_count: usize,
    },
    Failed {
        id: usize,
        run: u64,
        error: String,
    },
    SampleLoaded {
        id: usize,
        sample: Option<SampleCue>,
    },
    LanguageProbed {
        id: usize,
        detected: Option<String>,
        manual: bool,
    },
}

struct GuiReporter {
    id: usize,
    run: u64,
    sender: Sender<WorkerMessage>,
    cancelled: Arc<AtomicBool>,
    last_cue: Arc<AtomicUsize>,
    ctx: Option<egui::Context>,
}

impl GuiReporter {
    fn wake(&self) {
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }
}

impl ProgressReporter for GuiReporter {
    fn set_total(&self, total: usize) {
        let _ = self.sender.send(WorkerMessage::Started { id: self.id, run: self.run, total });
        self.wake();
    }

    fn set_position(&self, position: usize) {
        self.last_cue.store(position, Ordering::Relaxed);
    }

    fn println(&self, _message: &str) {}

    fn wants_preview_frames(&self) -> bool {
        true
    }

    fn on_cue_frame(
        &self,
        cue: usize,
        total: usize,
        start_ts: u32,
        stop_ts: u32,
        text: &str,
        rgba: Option<(usize, usize, Vec<u8>)>,
    ) {
        self.last_cue.store(cue, Ordering::Relaxed);
        let rgba = rgba.map(|(w, h, bytes)| (w, h, Arc::<[u8]>::from(bytes)));
        let _ = self.sender.send(WorkerMessage::CueProgress {
            id: self.id,
            run: self.run,
            cue,
            total,
            start_ts,
            stop_ts,
            text: text.to_string(),
            rgba,
        });
        self.wake();
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_conversion(
    id: usize,
    run: u64,
    input_path: String,
    language: String,
    output_path: String,
    sender: Sender<WorkerMessage>,
    cancelled: Arc<AtomicBool>,
    ctx: Option<egui::Context>,
) -> Option<JoinHandle<()>> {
    let thread_sender = sender.clone();
    let thread_ctx = ctx.clone();
    match std::thread::Builder::new().name(format!("pgs2srt-convert-{id}")).spawn(move || {
        let last_cue = Arc::new(AtomicUsize::new(0));
        let reporter = GuiReporter {
            id,
            run,
            sender: thread_sender.clone(),
            cancelled,
            last_cue: last_cue.clone(),
            ctx: thread_ctx.clone(),
        };

        let converter = PgsToSrt::new(&reporter);
        match converter.run(&input_path, &language, &output_path) {
            Ok(()) => {
                let count = last_cue.load(Ordering::Relaxed);
                let _ = thread_sender.send(WorkerMessage::Completed { id, run, cues_count: count });
            }
            Err(e) => {
                if matches!(e, Error::Cancelled) {
                    log::info!("Conversion of {input_path} cancelled");
                } else {
                    log::error!("Conversion of {input_path} ({language}) failed: {e}");
                }
                let _ = thread_sender.send(WorkerMessage::Failed { id, run, error: e.to_string() });
            }
        }
        if let Some(ctx) = thread_ctx {
            ctx.request_repaint();
        }
    }) {
        Ok(handle) => Some(handle),
        Err(e) => {
            log::error!("Failed to spawn conversion thread: {e}");
            let _ = sender.send(WorkerMessage::Failed { id, run, error: format!("Failed to spawn worker thread: {e}") });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
            None
        }
    }
}

pub fn spawn_sample_loader(
    id: usize,
    input_path: PathBuf,
    sender: Sender<WorkerMessage>,
    ctx: Option<egui::Context>,
) -> Option<JoinHandle<()>> {
    let thread_sender = sender.clone();
    let thread_ctx = ctx.clone();
    match std::thread::Builder::new().name(format!("pgs2srt-sample-{id}")).spawn(move || {
        let sample = extract_sample_cue(&input_path).unwrap_or_else(|e| {
            log::warn!("Cannot load preview sample from {}: {e}", input_path.display());
            None
        });
        let _ = thread_sender.send(WorkerMessage::SampleLoaded { id, sample });
        if let Some(ctx) = thread_ctx {
            ctx.request_repaint();
        }
    }) {
        Ok(handle) => Some(handle),
        Err(e) => {
            log::error!("Failed to spawn sample loader thread: {e}");
            let _ = sender.send(WorkerMessage::SampleLoaded { id, sample: None });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
            None
        }
    }
}

pub fn spawn_language_probe(
    id: usize,
    input_path: PathBuf,
    candidates: Vec<String>,
    manual: bool,
    cancelled: Arc<AtomicBool>,
    sender: Sender<WorkerMessage>,
    ctx: Option<egui::Context>,
) -> Option<JoinHandle<()>> {
    let thread_sender = sender.clone();
    let thread_ctx = ctx.clone();
    match std::thread::Builder::new().name(format!("pgs2srt-probe-{id}")).spawn(move || {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        let _guard = PROBE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        let detected = probe_language_from_file(&input_path, &candidates, &cancelled);
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        let _ = thread_sender.send(WorkerMessage::LanguageProbed { id, detected, manual });
        if let Some(ctx) = thread_ctx {
            ctx.request_repaint();
        }
    }) {
        Ok(handle) => Some(handle),
        Err(e) => {
            log::error!("Failed to spawn language probe thread: {e}");
            let _ = sender.send(WorkerMessage::LanguageProbed { id, detected: None, manual });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
            None
        }
    }
}
