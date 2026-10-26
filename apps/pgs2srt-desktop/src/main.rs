#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod platform;

use eframe::{NativeOptions, Renderer};
use egui::Vec2;
use platform::DesktopPlatform;
use ui_egui::PgsApp;

fn preferred_renderer() -> Renderer {
    match std::env::var("PGS2SRT_RENDERER") {
        Ok(value) if value.eq_ignore_ascii_case("glow") => Renderer::Glow,
        Ok(value) if !value.eq_ignore_ascii_case("wgpu") => {
            log::warn!("Ignoring unknown PGS2SRT_RENDERER={value}; using wgpu");
            Renderer::Wgpu
        }
        _ => Renderer::Wgpu,
    }
}

// winit has no file drag and drop on Wayland, so prefer X11 (XWayland) when it is available.
#[cfg(target_os = "linux")]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    use winit::platform::x11::EventLoopBuilderExtX11;

    let is_set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    if !is_set("WAYLAND_DISPLAY") {
        return None;
    }
    let wants_wayland = std::env::var("PGS2SRT_BACKEND").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"));
    if wants_wayland || !is_set("DISPLAY") {
        log::warn!("Running as a native Wayland client; drag and drop of files is unavailable");
        return None;
    }
    // Only invoked when eframe creates the event loop, not when a renderer fallback reuses it.
    Some(Box::new(|builder| {
        log::info!("Using the X11 backend (XWayland) for drag and drop");
        builder.with_x11();
    }))
}

#[cfg(not(target_os = "linux"))]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    None
}

// Without an icon eframe shows the egui logo, on macOS even in place of the bundle icon.
fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../../packaging/icons/pgs2srt.png")).unwrap_or_default()
}

fn run(renderer: Renderer) -> eframe::Result<()> {
    let options = NativeOptions {
        renderer,
        event_loop_builder: event_loop_hook(),
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1180.0, 740.0))
            .with_min_inner_size(Vec2::new(920.0, 580.0))
            .with_title("pgs2srt — Blu-ray SUP to SRT Workbench")
            .with_icon(app_icon())
            .with_decorations(false)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "pgs2srt-desktop",
        options,
        Box::new(move |cc| {
            log::info!("Using the {renderer} renderer");
            Ok(Box::new(PgsApp::new(cc, Box::new(DesktopPlatform::new()))))
        }),
    )
}

fn main() -> eframe::Result<()> {
    pgs_core::logging::init("pgs2srt-desktop", cfg!(debug_assertions));

    let renderer = preferred_renderer();
    let result = match run(renderer) {
        // eframe keeps its event loop alive between runs (`run_and_return`), so glow can take over
        // when wgpu finds no usable adapter.
        Err(error) if renderer == Renderer::Wgpu => {
            log::error!("wgpu renderer failed, falling back to glow: {error}");
            run(Renderer::Glow)
        }
        result => result,
    };
    if let Err(error) = &result {
        log::error!("Cannot start the desktop app: {error}");
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_icon_decodes() {
        let icon = super::app_icon();
        assert_eq!((icon.width, icon.height), (512, 512));
    }
}
