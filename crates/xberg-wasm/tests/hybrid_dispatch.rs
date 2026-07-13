#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn resolve_ner_with_injected_stub() {
    let stub = js_sys::eval(
        "({
            ner: async (text, categories) => [
                { category: 'person', text: 'Alice', start: 0, end: 5, confidence: 0.95 },
                { category: 'organization', text: 'Acme Corp', start: 6, end: 15, confidence: 0.88 }
            ]
        })",
    )
    .unwrap()
    .dyn_into::<js_sys::Object>()
    .unwrap();

    let entities = xberg_wasm::bridge::ner::resolve_ner(Some(stub), "Alice works at Acme Corp", &[])
        .await
        .unwrap();

    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0].text, "Alice");
    assert_eq!(entities[1].text, "Acme Corp");
}

#[wasm_bindgen_test]
async fn resolve_ner_without_injected_returns_error() {
    let result = xberg_wasm::bridge::ner::resolve_ner(None, "hello", &[]).await;
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("unavailable"), "expected 'unavailable' in error: {msg}");
}

#[wasm_bindgen_test]
async fn resolve_ocr_with_injected_stub() {
    let stub = js_sys::eval(
        "({
            ocr: async (bytes, opts) => ({
                text: 'hello from ocr',
                lines: [
                    { text: 'hello from ocr', confidence: 0.98, bbox: { x: 1, y: 2, w: 3, h: 4 } }
                ]
            })
        })",
    )
    .unwrap()
    .dyn_into::<js_sys::Object>()
    .unwrap();

    let result = xberg_wasm::bridge::ocr::resolve_ocr(Some(stub), &[0xFF, 0xD8, 0xFF, 0xE0], "eng")
        .await
        .unwrap();

    assert_eq!(result.text, "hello from ocr");
    assert_eq!(result.lines.len(), 1);
    assert_eq!(result.lines[0].text, "hello from ocr");
    assert!((result.lines[0].confidence - 0.98).abs() < f64::EPSILON);
    let bbox = result.lines[0].bbox.as_ref().expect("bbox should be present");
    assert_eq!(bbox.x, 1.0);
    assert_eq!(bbox.y, 2.0);
    assert_eq!(bbox.w, 3.0);
    assert_eq!(bbox.h, 4.0);
}

#[wasm_bindgen_test]
async fn resolve_ocr_with_injected_stub_missing_lines_defaults_to_empty() {
    let stub = js_sys::eval(
        "({
            ocr: async (bytes, opts) => ({ text: 'no geometry available' })
        })",
    )
    .unwrap()
    .dyn_into::<js_sys::Object>()
    .unwrap();

    let result = xberg_wasm::bridge::ocr::resolve_ocr(Some(stub), &[0xFF, 0xD8, 0xFF, 0xE0], "eng")
        .await
        .unwrap();

    assert_eq!(result.text, "no geometry available");
    assert!(result.lines.is_empty());
}

#[wasm_bindgen_test]
async fn resolve_ocr_without_injected_returns_error() {
    let result = xberg_wasm::bridge::ocr::resolve_ocr(None, &[0xFF, 0xD8, 0xFF, 0xE0], "eng").await;
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("unavailable"), "expected 'unavailable' in error: {msg}");
}
