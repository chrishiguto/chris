//! The one Workers Cache write path: `cache.purge` from `cloudflare:workers`.
//! No workers-rs binding exists yet, so this imports the JS module directly.

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use worker::js_sys;

#[wasm_bindgen(module = "cloudflare:workers")]
extern "C" {
    #[wasm_bindgen(thread_local_v2, js_name = cache)]
    static CACHE: JsValue;
}

/// `cache.purge({purgeEverything: true})` — global via Instant Purge, scoped
/// to this worker's entrypoint. Resolves to `{success, errors}`.
pub(crate) async fn purge_everything() -> Result<(), String> {
    let cache = CACHE.with(JsValue::clone);
    let purge: js_sys::Function = js_sys::Reflect::get(&cache, &"purge".into())
        .map_err(fail)?
        .dyn_into()
        .map_err(|_| "cache.purge is not a function".to_string())?;
    let options = js_sys::Object::new();
    js_sys::Reflect::set(&options, &"purgeEverything".into(), &JsValue::TRUE).map_err(fail)?;
    let promise: js_sys::Promise = purge
        .call1(&cache, &options)
        .map_err(fail)?
        .dyn_into()
        .map_err(|_| "cache.purge did not return a promise".to_string())?;
    let outcome = worker::wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(fail)?;
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
