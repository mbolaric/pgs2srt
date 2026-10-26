use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;
use pgs::PgsParser;
use pgs_core::language::probe_language;
use pgs_core::{sample_cue, Error, PgsToSrt, ProgressReporter, SampleCue};

use crate::input::Input;
use crate::platform::Job;

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
        srt: Vec<u8>,
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
        run: u64,
        detected: Option<String>,
        manual: bool,
    },
    LanguageInstalled {
        code: String,
        result: Result<(), String>,
    },
    InputsPicked {
        inputs: Vec<Input>,
        folder: bool,
    },
}

/// Sends worker results to the UI and wakes it to handle them.
#[derive(Clone)]
pub struct Mailbox {
    sender: Sender<WorkerMessage>,
    ctx: Option<egui::Context>,
}

impl Mailbox {
    pub fn new(sender: Sender<WorkerMessage>, ctx: Option<egui::Context>) -> Self {
        Self { sender, ctx }
    }

    pub fn send(&self, message: WorkerMessage) {
        let _ = self.sender.send(message);
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }
}

struct GuiReporter {
    id: usize,
    run: u64,
    mailbox: Mailbox,
    cancelled: Arc<AtomicBool>,
}

impl ProgressReporter for GuiReporter {
    fn set_total(&self, total: usize) {
        self.mailbox.send(WorkerMessage::Started { id: self.id, run: self.run, total });
    }

    fn set_position(&self, _position: usize) {}

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
        let rgba = rgba.map(|(w, h, bytes)| (w, h, Arc::<[u8]>::from(bytes)));
        self.mailbox.send(WorkerMessage::CueProgress {
            id: self.id,
            run: self.run,
            cue,
            total,
            start_ts,
            stop_ts,
            text: text.to_string(),
            rgba,
        });
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

fn parse(input: &Input) -> pgs_core::Result<PgsParser<'static>> {
    Ok(PgsParser::parse_bytes(input.bytes()?)?)
}

pub fn conversion_job(id: usize, run: u64, input: Input, language: String, cancelled: Arc<AtomicBool>, mailbox: Mailbox) -> Job {
    Box::new(move |backend| {
        Box::pin(async move {
            let reporter = GuiReporter { id, run, mailbox: mailbox.clone(), cancelled };
            let mut srt = Vec::new();
            let converted = match parse(&input) {
                Ok(parser) => PgsToSrt::new(&reporter, backend).convert(parser.get_display_sets(), &language, &mut srt).await,
                Err(e) => Err(e),
            };
            let message = match converted {
                Ok(cues_count) => WorkerMessage::Completed { id, run, cues_count, srt },
                Err(e) => {
                    if matches!(e, Error::Cancelled) {
                        log::info!("Conversion of {} cancelled", input.path().display());
                    } else {
                        log::error!("Conversion of {} ({language}) failed: {e}", input.path().display());
                    }
                    WorkerMessage::Failed { id, run, error: e.to_string() }
                }
            };
            mailbox.send(message);
        })
    })
}

pub fn sample_job(id: usize, input: Input, mailbox: Mailbox) -> Job {
    Box::new(move |_| {
        Box::pin(async move {
            let sample = parse(&input).and_then(|parser| sample_cue(parser.get_display_sets())).unwrap_or_else(|e| {
                log::warn!("Cannot load preview sample from {}: {e}", input.path().display());
                None
            });
            mailbox.send(WorkerMessage::SampleLoaded { id, sample });
        })
    })
}

pub fn probe_job(
    id: usize,
    run: u64,
    input: Input,
    candidates: Vec<String>,
    manual: bool,
    cancelled: Arc<AtomicBool>,
    mailbox: Mailbox,
) -> Job {
    Box::new(move |backend| {
        Box::pin(async move {
            let detected = match parse(&input) {
                Ok(parser) => probe_language(parser.get_display_sets(), &candidates, backend, &cancelled).await,
                Err(e) => {
                    log::warn!("Cannot detect the language of {}: {e}", input.path().display());
                    None
                }
            };
            mailbox.send(WorkerMessage::LanguageProbed { id, run, detected, manual });
        })
    })
}

pub fn install_job(code: String, mailbox: Mailbox) -> Job {
    Box::new(move |backend| {
        Box::pin(async move {
            let result = backend.install(&code).await.map_err(|e| Error::from(e).to_string());
            mailbox.send(WorkerMessage::LanguageInstalled { code, result });
        })
    })
}
