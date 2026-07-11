use std::error::Error;
use std::fmt;

use async_trait::async_trait;
use js_sys::{Array, Float32Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use xberg_rag::error::{RagError, RagResult};
use xberg_rag::pipeline::Embedder;

#[derive(Debug)]
struct JsBridgeError(String);

impl fmt::Display for JsBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JS embedder error: {}", self.0)
    }
}

impl Error for JsBridgeError {}

pub struct JsEmbedder {
    inner: Object,
    timeout_ms: u32,
}

impl JsEmbedder {
    pub fn new(inner: Object) -> Self {
        Self {
            inner,
            timeout_ms: crate::bridge::BRIDGE_TIMEOUT_MS,
        }
    }

    pub fn with_timeout(inner: Object, timeout_ms: u32) -> Self {
        Self { inner, timeout_ms }
    }
}

#[async_trait(?Send)]
impl Embedder for JsEmbedder {
    async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
        let js_texts = Array::new();
        for t in &texts {
            js_texts.push(&JsValue::from_str(t));
        }
        let f = Reflect::get(&self.inner, &JsValue::from_str("embed"))
            .map_err(js_to_rag)?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| RagError::Backend(Box::new(JsBridgeError("injected embedder has no embed()".into()))))?;
        let promise = f.call1(&self.inner, &js_texts).map_err(js_to_rag)?;
        let result = crate::bridge::timed_js_future_with_timeout(js_sys::Promise::from(promise), self.timeout_ms)
            .await
            .map_err(js_to_rag)?;
        let arr: Array = result
            .dyn_into()
            .map_err(|_| RagError::Backend(Box::new(JsBridgeError("embed() did not resolve to an array".into()))))?;
        let mut out = Vec::with_capacity(arr.length() as usize);
        for v in arr.iter() {
            let f32arr: Float32Array = v
                .dyn_into()
                .map_err(|_| RagError::Backend(Box::new(JsBridgeError("embed() row is not Float32Array".into()))))?;
            out.push(f32arr.to_vec());
        }
        Ok(out)
    }
}

fn js_to_rag(v: JsValue) -> RagError {
    RagError::Backend(Box::new(JsBridgeError(format!("{v:?}"))))
}
