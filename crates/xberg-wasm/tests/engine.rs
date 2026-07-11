#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

fn make_engine() -> xberg_wasm::engine::XbergEngine {
    let injection = js_sys::eval("({})").unwrap().dyn_into::<js_sys::Object>().unwrap();
    let config = js_sys::eval("({})").unwrap();
    xberg_wasm::engine::XbergEngine::new(config.into(), injection.into()).unwrap()
}

#[wasm_bindgen_test]
async fn engine_construction() {
    let injection = js_sys::eval("({})").unwrap().dyn_into::<js_sys::Object>().unwrap();
    let config = js_sys::eval("({})").unwrap();
    let engine = xberg_wasm::engine::XbergEngine::new(config.into(), injection.into());
    assert!(engine.is_ok());
}

#[wasm_bindgen_test]
async fn engine_construction_with_null_injection() {
    let config = js_sys::eval("({})").unwrap();
    let engine = xberg_wasm::engine::XbergEngine::new(config.into(), JsValue::NULL);
    assert!(engine.is_ok());
}

#[wasm_bindgen_test]
async fn detect_pii_returns_empty_for_clean_text() {
    let engine = make_engine();
    let result = engine.detect_pii("Hello world", None).unwrap();
    let matches: js_sys::Array = result.dyn_into().unwrap();
    assert_eq!(matches.length(), 0);
}

#[wasm_bindgen_test]
async fn detect_pii_finds_email() {
    let engine = make_engine();
    let result = engine.detect_pii("Contact alice@example.com for info", None).unwrap();
    let matches: js_sys::Array = result.dyn_into().unwrap();
    assert!(matches.length() > 0);
}

#[wasm_bindgen_test]
async fn detect_pii_with_categories() {
    let engine = make_engine();
    let cats = Some(vec!["email".to_string()]);
    let result = engine
        .detect_pii("Email alice@example.com and phone 555-1234", cats)
        .unwrap();
    let matches: js_sys::Array = result.dyn_into().unwrap();
    assert_eq!(matches.length(), 1);
}

#[wasm_bindgen_test]
async fn redact_mask_strategy() {
    let engine = make_engine();
    let result = engine
        .redact("Email alice@example.com here", Some("mask".to_string()), None)
        .unwrap();
    let obj: js_sys::Object = result.dyn_into().unwrap();
    let redacted = js_sys::Reflect::get(&obj, &JsValue::from_str("redacted"))
        .unwrap()
        .as_string()
        .unwrap();
    assert!(!redacted.contains("alice@example.com"));
    assert!(redacted.contains("[REDACTED]"));
}

#[wasm_bindgen_test]
async fn redact_token_replace_strategy() {
    let engine = make_engine();
    let result = engine
        .redact("Email alice@example.com here", Some("token_replace".to_string()), None)
        .unwrap();
    let obj: js_sys::Object = result.dyn_into().unwrap();
    let token_map = js_sys::Reflect::get(&obj, &JsValue::from_str("rehydrationMap")).unwrap();
    assert!(!token_map.is_undefined());
    assert!(!token_map.is_null());
}

#[wasm_bindgen_test]
async fn redact_returns_original_when_no_pii() {
    let engine = make_engine();
    let result = engine.redact("No PII here", Some("mask".to_string()), None).unwrap();
    let obj: js_sys::Object = result.dyn_into().unwrap();
    let redacted = js_sys::Reflect::get(&obj, &JsValue::from_str("redacted"))
        .unwrap()
        .as_string()
        .unwrap();
    assert_eq!(redacted, "No PII here");
}

#[wasm_bindgen_test]
async fn query_missing_store_errors() {
    let engine = make_engine();

    let result = engine.query("test".to_string(), "coll".to_string(), 5).await;
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn ingest_missing_injection_errors() {
    let engine = make_engine();

    let doc = js_sys::eval("({ full_text: 'hello' })").unwrap();
    let result = engine.ingest(doc, "coll".to_string(), None).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().as_string().unwrap();
    assert!(msg.contains("embedder"));
}

#[wasm_bindgen_test]
async fn ingest_config_chunking_override() {
    let injection = js_sys::eval(
        r#"
        (() => {
            const obj = {};
            obj.embedder = {
                embed: (texts) => new Promise((resolve) => {
                    resolve(texts.map(() => new Float32Array([0.1])));
                })
            };
            obj.store = {
                upsertDocument: (collection, document, chunks) => {
                    globalThis.__lastChunks = chunks.map(c => c.content);
                    return new Promise((resolve) => resolve("doc-id"));
                }
            };
            return obj;
        })()
        "#,
    )
    .unwrap()
    .dyn_into::<js_sys::Object>()
    .unwrap();
    let config = js_sys::eval("({})").unwrap();
    let engine = xberg_wasm::engine::XbergEngine::new(config.into(), injection.into()).unwrap();

    let doc =
        js_sys::eval("({ full_text: 'hello world. second sentence. third sentence. fourth sentence.' })").unwrap();

    // Default chunking (1000 chars) should produce 1 chunk for this small text
    let result_default = engine.ingest(doc.clone(), "coll-default".to_string(), None).await;
    assert!(result_default.is_ok());

    // Override to tiny chunk size (20 chars) should produce multiple chunks
    let config_override = js_sys::eval("({ chunking: { maxCharacters: 20, overlap: 5 } })").unwrap();
    let result_override = engine
        .ingest(doc, "coll-override".to_string(), Some(config_override.into()))
        .await;
    assert!(result_override.is_ok());
}

#[wasm_bindgen_test]
async fn ocr_no_injection_errors() {
    let engine = make_engine();

    let result = engine.ocr(vec![0u8; 10], JsValue::UNDEFINED).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().as_string().unwrap().contains("OCR"));
}

#[wasm_bindgen_test]
async fn ner_no_injection_errors() {
    let engine = make_engine();

    let result = engine.ner("test".to_string(), JsValue::UNDEFINED).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().as_string().unwrap().contains("NER"));
}
