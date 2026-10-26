#[cfg(target_arch = "wasm32")]
mod download;
#[cfg(target_arch = "wasm32")]
mod platform;

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    let _ = eframe::WebLogger::init(log::LevelFilter::Info);
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().and_then(|window| window.document()).expect("pgs2srt runs in a page");
        let canvas = document
            .get_element_by_id("pgs2srt")
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("the page has the pgs2srt canvas");
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(ui_egui::PgsApp::new(cc, Box::new(platform::WebPlatform::new()))))),
            )
            .await;
        let status = document.get_element_by_id("status");
        match (started, status) {
            (Ok(()), Some(status)) => status.remove(),
            (Err(error), status) => {
                log::error!("Cannot start pgs2srt: {error:?}");
                if let Some(status) = status {
                    status.set_text_content(Some("pgs2srt cannot start in this browser."));
                }
            }
            (Ok(()), None) => {}
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("pgs2srt-web runs in the browser: build it with trunk in apps/pgs2srt-web");
}
