pub trait ProgressReporter: Send + Sync {
    fn set_total(&self, total: usize);
    fn set_position(&self, position: usize);
    fn println(&self, message: &str);

    /// Optional live notification for each recognized subtitle cue.
    fn on_cue(&self, _cue: usize, _total: usize, _start_ts: u32, _stop_ts: u32, _text: &str) {}

    /// Whether the reporter wants decoded RGBA subtitle bitmaps passed to [`Self::on_cue_frame`].
    fn wants_preview_frames(&self) -> bool {
        false
    }

    /// Optional live notification with the decoded RGBA bitmap `(width, height, rgba_bytes)`.
    #[allow(clippy::too_many_arguments)]
    fn on_cue_frame(
        &self,
        cue: usize,
        total: usize,
        start_ts: u32,
        stop_ts: u32,
        text: &str,
        _rgba: Option<(usize, usize, Vec<u8>)>,
    ) {
        self.on_cue(cue, total, start_ts, stop_ts, text);
    }

    /// Checks if cancellation has been requested.
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpProgressReporter;

impl ProgressReporter for NoOpProgressReporter {
    fn set_total(&self, _total: usize) {}
    fn set_position(&self, _position: usize) {}
    fn println(&self, _message: &str) {}
}
