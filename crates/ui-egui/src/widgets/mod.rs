//! Custom desktop controls built on `theme::Tokens` with exact shared heights and baselines,
//! modeled after PdfCraft (`storytold/pdfcraft`).

mod buttons;
mod display;
mod inputs;
mod navigation;

pub use buttons::*;
pub use display::*;
pub use inputs::*;
pub use navigation::*;

pub const TOOLBAR_HEIGHT: f32 = 30.0;
pub const ROW_CONTROL_HEIGHT: f32 = 28.0;
pub(crate) const CHECKBOX_RADIUS: u8 = 3;
