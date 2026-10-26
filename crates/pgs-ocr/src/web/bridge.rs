use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/web/bridge.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = createWorker)]
    pub(super) async fn create_worker(language: &str, lang_path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub(super) async fn recognize(
        worker: &JsValue,
        width: u32,
        height: u32,
        rgb: &[u8],
        page_mode: u32,
    ) -> Result<JsValue, JsValue>;

    pub(super) fn terminate(worker: &JsValue);

    #[wasm_bindgen(js_name = storedLanguages)]
    pub(super) fn stored_languages() -> String;

    #[wasm_bindgen(js_name = storeLanguages)]
    pub(super) fn store_languages(codes: &str);
}

#[wasm_bindgen]
extern "C" {
    pub(super) type Recognized;

    #[wasm_bindgen(method, getter)]
    pub(super) fn text(this: &Recognized) -> String;

    #[wasm_bindgen(method, getter)]
    pub(super) fn confidence(this: &Recognized) -> f32;
}

/// The bridge throws plain strings.
pub(super) fn message(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
