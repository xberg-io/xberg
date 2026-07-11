#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn js_embedder_returns_vectors() {
    let stub = js_sys::eval("({ embed: async (t) => t.map(() => new Float32Array([0.1, 0.2])) })")
        .unwrap()
        .dyn_into::<js_sys::Object>()
        .unwrap();
    let emb = xberg_wasm::bridge::embedder::JsEmbedder::new(stub);
    let out = emb.embed(vec!["a".into(), "b".into()]).await.unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0], vec![0.1_f32, 0.2]);
}
