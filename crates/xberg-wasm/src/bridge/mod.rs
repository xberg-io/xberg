//! JS bridge utilities for injected backends.
//!
//! Hand-written module (declared via `custom_rust_modules` in `alef.toml`),
//! not managed by alef.

pub mod ocr;

use std::sync::OnceLock;

use wasm_bindgen::prelude::*;

pub(crate) const BRIDGE_TIMEOUT_MS: u32 = 30_000;

fn get_timeout_racer() -> &'static js_sys::Function {
    static RACER: OnceLock<js_sys::Function> = OnceLock::new();
    RACER.get_or_init(|| {
        js_sys::Function::new_with_args(
            "p, ms",
            "return (() => {\n\
             let id;\n\
             const timer = new Promise((_, reject) => {\n\
                 id = setTimeout(() => reject(new Error('bridge call timed out')), ms);\n\
             });\n\
             p.finally(() => clearTimeout(id));\n\
             return Promise.race([p, timer]);\n\
             })()",
        )
    })
}

/// Wrap a JS `Promise` with a timeout.
///
/// If the promise does not resolve within `ms` milliseconds, the returned
/// promise rejects with an `Error("bridge call timed out")`.  Uses
/// `Promise.race` under the hood so the original promise is still cancellable.
/// The timer handle is cleared via `p.finally` so the timeout does not stay
/// alive after the promise settles.
pub fn with_timeout(promise: js_sys::Promise, ms: u32) -> js_sys::Promise {
    let racer = get_timeout_racer();
    let p_val = JsValue::from(&promise);
    match racer.call2(&JsValue::NULL, &p_val, &JsValue::from(ms)) {
        Ok(val) => val.into(),
        Err(_) => promise,
    }
}

/// Convenience wrapper: create a timed-out `JsFuture` from a `Promise`.
pub fn timed_js_future(promise: js_sys::Promise) -> wasm_bindgen_futures::JsFuture {
    wasm_bindgen_futures::JsFuture::from(with_timeout(promise, BRIDGE_TIMEOUT_MS))
}

/// Convenience wrapper: create a `JsFuture` with a custom timeout from a `Promise`.
pub fn timed_js_future_with_timeout(promise: js_sys::Promise, ms: u32) -> wasm_bindgen_futures::JsFuture {
    wasm_bindgen_futures::JsFuture::from(with_timeout(promise, ms))
}
