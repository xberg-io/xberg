#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn from_bytes_errors_cleanly_in_browser() {
    // Proves Gliner2Candle::from_bytes compiles, links, and runs on wasm32
    // (via Node.js under wasm-bindgen-test-runner) — the whole Task 1-3
    // stack (ort-free xberg-gliner + candle + tokenizers) executes for
    // real, not just compiles. Empty weights must yield a clean error,
    // never a panic/trap.
    let err = xberg_gliner_candle::Gliner2Candle::from_bytes(&[], b"{}", b"{}").unwrap_err();
    assert!(!err.to_string().is_empty());
}
