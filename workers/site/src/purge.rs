//! The one Workers Cache write path: `cache.purge` from `cloudflare:workers`.
//! No workers-rs binding exists yet, so this imports the JS module directly.

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use worker::js_sys;
use worker::wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "cloudflare:workers")]
extern "C" {
    type CacheApi;
    #[wasm_bindgen(thread_local_v2, js_name = cache)]
    static CACHE: CacheApi;
    #[wasm_bindgen(method, catch)]
    fn purge(this: &CacheApi, options: &JsValue) -> Result<js_sys::Promise, JsValue>;
}

/// `cache.purge({purgeEverything: true})` — global via Instant Purge, scoped
/// to this worker's entrypoint. Resolves to `{success, errors}`.
pub(crate) async fn purge_everything() -> Result<(), String> {
    let options = js_sys::Object::new();
    js_sys::Reflect::set(&options, &"purgeEverything".into(), &JsValue::TRUE).map_err(fail)?;
    let promise = CACHE
        .with(|cache| cache.purge(options.as_ref()))
        .map_err(fail)?;
    let outcome = JsFuture::from(promise).await.map_err(fail)?;
    let success = js_sys::Reflect::get(&outcome, &"success".into())
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    success
        .then_some(())
        .ok_or_else(|| format!("purge reported failure: {outcome:?}"))
}

fn fail(err: JsValue) -> String {
    format!("{err:?}")
}
