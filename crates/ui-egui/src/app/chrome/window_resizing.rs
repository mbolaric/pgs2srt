use egui::{Context, CursorIcon, PointerButton, Pos2, Rect, ResizeDirection, ViewportCommand};

/// Window edge and corner hit-testing and resizing controller following
/// the Clean Architecture / Craft-UI Pattern modeled after PdfCraft (`storytold/pdfcraft`).
///
/// For undecorated/borderless desktop windows, native window managers do not
/// provide OS border frames. This component monitors pointer positions along
/// the viewport perimeter, manages appropriate directional cursor icons, and
/// triggers native OS viewport interactive resizing via `ViewportCommand::BeginResize`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowResizer {
    pub border_thickness: f32,
    pub corner_size: f32,
}

impl WindowResizer {
    /// Default border thickness in logical points along all outer edges.
    pub const DEFAULT_BORDER_THICKNESS: f32 = 6.0;

    /// Default corner grab dimension in logical points along both axes.
    pub const DEFAULT_CORNER_SIZE: f32 = 14.0;

    /// Creates a new `WindowResizer` with custom edge and corner sensitivities.
    pub fn new(border_thickness: f32, corner_size: f32) -> Self {
        Self { border_thickness, corner_size }
    }

    /// Evaluates whether the given pointer position falls into any border or corner resize zone.
    /// Returns `Some(ResizeDirection)` if within a resize zone, or `None` if in the content area
    /// or outside the window.
    pub fn hit_test(viewport_rect: Rect, pointer_pos: Pos2, border_thickness: f32, corner_size: f32) -> Option<ResizeDirection> {
        let slack = 2.0;
        let bounds = viewport_rect.expand(slack);
        if !bounds.contains(pointer_pos) {
            return None;
        }

        let in_top = pointer_pos.y <= viewport_rect.min.y + border_thickness;
        let in_bottom = pointer_pos.y >= viewport_rect.max.y - border_thickness;
        let in_left = pointer_pos.x <= viewport_rect.min.x + border_thickness;
        let in_right = pointer_pos.x >= viewport_rect.max.x - border_thickness;

        let near_top_corner = pointer_pos.y <= viewport_rect.min.y + corner_size;
        let near_bottom_corner = pointer_pos.y >= viewport_rect.max.y - corner_size;
        let near_left_corner = pointer_pos.x <= viewport_rect.min.x + corner_size;
        let near_right_corner = pointer_pos.x >= viewport_rect.max.x - corner_size;

        // Corners have priority: when pointer is near an outer corner within corner_size
        // and on at least one border edge
        if (in_top && near_left_corner) || (in_left && near_top_corner) {
            Some(ResizeDirection::NorthWest)
        } else if (in_top && near_right_corner) || (in_right && near_top_corner) {
            Some(ResizeDirection::NorthEast)
        } else if (in_bottom && near_left_corner) || (in_left && near_bottom_corner) {
            Some(ResizeDirection::SouthWest)
        } else if (in_bottom && near_right_corner) || (in_right && near_bottom_corner) {
            Some(ResizeDirection::SouthEast)
        } else if in_top {
            Some(ResizeDirection::North)
        } else if in_bottom {
            Some(ResizeDirection::South)
        } else if in_left {
            Some(ResizeDirection::West)
        } else if in_right {
            Some(ResizeDirection::East)
        } else {
            None
        }
    }

    /// Maps a `ResizeDirection` to the corresponding egui `CursorIcon`.
    pub fn cursor_for_direction(direction: ResizeDirection) -> CursorIcon {
        match direction {
            ResizeDirection::North => CursorIcon::ResizeNorth,
            ResizeDirection::South => CursorIcon::ResizeSouth,
            ResizeDirection::East => CursorIcon::ResizeEast,
            ResizeDirection::West => CursorIcon::ResizeWest,
            ResizeDirection::NorthEast => CursorIcon::ResizeNorthEast,
            ResizeDirection::NorthWest => CursorIcon::ResizeNorthWest,
            ResizeDirection::SouthEast => CursorIcon::ResizeSouthEast,
            ResizeDirection::SouthWest => CursorIcon::ResizeSouthWest,
        }
    }

    /// Primary handler called every frame.
    ///
    /// If the window is unmaximized and the pointer is hovering over any outer edge or corner:
    /// - Updates the context cursor to the appropriate resize cursor icon.
    /// - Dispatches `ViewportCommand::BeginResize` if the primary pointer button was pressed down.
    ///
    /// Returns `Some(ResizeDirection)` if the pointer is within a resize region, or `None` otherwise.
    pub fn handle(&self, ctx: &Context) -> Option<ResizeDirection> {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if is_maximized || is_fullscreen {
            return None;
        }

        let pointer_pos = ctx.input(|i| i.pointer.latest_pos())?;
        let viewport_rect = ctx.viewport_rect();

        let direction = Self::hit_test(viewport_rect, pointer_pos, self.border_thickness, self.corner_size)?;

        ctx.set_cursor_icon(Self::cursor_for_direction(direction));

        if ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }

        Some(direction)
    }

    /// Checks if the pointer is currently within any resize edge or corner of the window.
    pub fn is_in_resize_zone(&self, ctx: &Context) -> bool {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if is_maximized || is_fullscreen {
            return false;
        }

        let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) else {
            return false;
        };
        let viewport_rect = ctx.viewport_rect();
        Self::hit_test(viewport_rect, pos, self.border_thickness, self.corner_size).is_some()
    }
}

impl Default for WindowResizer {
    fn default() -> Self {
        Self { border_thickness: Self::DEFAULT_BORDER_THICKNESS, corner_size: Self::DEFAULT_CORNER_SIZE }
    }
}
