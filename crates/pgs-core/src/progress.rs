pub trait ProgressReporter: Send + Sync {
    fn set_total(&self, total: usize);
    fn set_position(&self, position: usize);
    fn println(&self, message: &str);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpProgressReporter;

impl ProgressReporter for NoOpProgressReporter {
    fn set_total(&self, _total: usize) {}
    fn set_position(&self, _position: usize) {}
    fn println(&self, _message: &str) {}
}
