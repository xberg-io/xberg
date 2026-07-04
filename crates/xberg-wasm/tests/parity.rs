#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

/// Known fixture text containing multiple PII categories.
const FIXTURE_TEXT: &str = "Contact John Doe at john.doe@example.com or call +1-555-123-4567. SSN: 123-45-6789.";

/// Helper: create an engine with no injected bridges.
fn make_engine() -> xberg_wasm::engine::XbergEngine {
    let injection = js_sys::eval("({})").unwrap().dyn_into::<js_sys::Object>().unwrap();
    let config = js_sys::eval("({})").unwrap();
    xberg_wasm::engine::XbergEngine::new(config.into(), injection.into()).unwrap()
}

// ---------------------------------------------------------------------------
// PII detection parity
// ---------------------------------------------------------------------------

/// Detect PII on the fixture text and verify expected categories + counts.
///
/// The fixture contains:
///   - 1 email  (john.doe@example.com)
///   - 1 phone  (+1-555-123-4567)
///   - 1 SSN    (123-45-6789)
#[wasm_bindgen_test]
async fn pii_detection_parity() {
    let engine = make_engine();

    let result = engine.detect_pii(FIXTURE_TEXT, None).unwrap();
    let matches: js_sys::Array = result.dyn_into().unwrap();

    // Collect category labels from the result.
    let mut categories: Vec<String> = Vec::new();
    for i in 0..matches.length() {
        let m: js_sys::Object = matches.get(i).dyn_into().unwrap();
        let cat = js_sys::Reflect::get(&m, &JsValue::from_str("category"))
            .unwrap()
            .as_string()
            .unwrap();
        categories.push(cat);
    }

    // We expect at least email, phone, and ssn.
    assert!(
        categories.contains(&"email".to_string()),
        "expected email in {categories:?}"
    );
    assert!(
        categories.contains(&"phone".to_string()),
        "expected phone in {categories:?}"
    );
    assert!(
        categories.contains(&"ssn".to_string()),
        "expected ssn in {categories:?}"
    );

    // Count each expected category — should be exactly 1.
    let email_count = categories.iter().filter(|c| *c == "email").count();
    let phone_count = categories.iter().filter(|c| *c == "phone").count();
    let ssn_count = categories.iter().filter(|c| *c == "ssn").count();
    assert_eq!(email_count, 1, "expected 1 email, got {email_count}");
    assert_eq!(phone_count, 1, "expected 1 phone, got {phone_count}");
    assert_eq!(ssn_count, 1, "expected 1 ssn, got {ssn_count}");
}

/// Same input must always produce the same PII categories and counts
/// (determinism / idempotency of detection).
#[wasm_bindgen_test]
async fn pii_detection_idempotent() {
    let engine = make_engine();

    let collect_categories = |engine: &xberg_wasm::engine::XbergEngine| -> Vec<String> {
        let result = engine.detect_pii(FIXTURE_TEXT, None).unwrap();
        let matches: js_sys::Array = result.dyn_into().unwrap();
        let mut cats = Vec::new();
        for i in 0..matches.length() {
            let m: js_sys::Object = matches.get(i).dyn_into().unwrap();
            let cat = js_sys::Reflect::get(&m, &JsValue::from_str("category"))
                .unwrap()
                .as_string()
                .unwrap();
            cats.push(cat);
        }
        cats.sort();
        cats
    };

    let first = collect_categories(&engine);
    let second = collect_categories(&engine);
    assert_eq!(first, second, "detection output must be deterministic");
}

// ---------------------------------------------------------------------------
// Redaction parity
// ---------------------------------------------------------------------------

/// Redact with token_replace and verify PII is removed and rehydration map
/// is populated.
#[wasm_bindgen_test]
async fn redaction_parity() {
    let engine = make_engine();

    let result = engine
        .redact(FIXTURE_TEXT, Some("token_replace".to_string()), None)
        .unwrap();
    let obj: js_sys::Object = result.dyn_into().unwrap();

    let redacted = js_sys::Reflect::get(&obj, &JsValue::from_str("redacted"))
        .unwrap()
        .as_string()
        .unwrap();

    // Redacted text must not contain the original PII values.
    assert!(
        !redacted.contains("john.doe@example.com"),
        "email should be removed from: {redacted}"
    );
    assert!(
        !redacted.contains("+1-555-123-4567"),
        "phone should be removed from: {redacted}"
    );
    assert!(
        !redacted.contains("123-45-6789"),
        "SSN should be removed from: {redacted}"
    );

    // Rehydration map must be a non-empty object.
    let map_val = js_sys::Reflect::get(&obj, &JsValue::from_str("rehydrationMap")).unwrap();
    assert!(!map_val.is_undefined(), "rehydrationMap must be defined");
    assert!(!map_val.is_null(), "rehydrationMap must not be null");
    let map_obj: js_sys::Object = map_val.dyn_into().unwrap();
    let keys = js_sys::Reflect::own_keys(&map_obj).unwrap();
    assert!(
        keys.length() >= 3,
        "rehydration map should have >=3 entries (email, phone, ssn), got {}",
        keys.length()
    );
}

/// Redacting the same text twice must produce the same redacted output
/// (deterministic token assignment).
#[wasm_bindgen_test]
async fn redaction_idempotent() {
    let engine = make_engine();

    let redact_once = |eng: &xberg_wasm::engine::XbergEngine| -> String {
        let result = eng
            .redact(FIXTURE_TEXT, Some("token_replace".to_string()), None)
            .unwrap();
        let obj: js_sys::Object = result.dyn_into().unwrap();
        js_sys::Reflect::get(&obj, &JsValue::from_str("redacted"))
            .unwrap()
            .as_string()
            .unwrap()
    };

    let first = redact_once(&engine);
    let second = redact_once(&engine);
    assert_eq!(first, second, "redaction output must be deterministic");
}

/// Mask strategy must also remove PII and use [REDACTED] tokens.
#[wasm_bindgen_test]
async fn redaction_mask_parity() {
    let engine = make_engine();

    let result = engine.redact(FIXTURE_TEXT, Some("mask".to_string()), None).unwrap();
    let obj: js_sys::Object = result.dyn_into().unwrap();
    let redacted = js_sys::Reflect::get(&obj, &JsValue::from_str("redacted"))
        .unwrap()
        .as_string()
        .unwrap();

    assert!(!redacted.contains("john.doe@example.com"), "email should be masked");
    assert!(!redacted.contains("+1-555-123-4567"), "phone should be masked");
    assert!(!redacted.contains("123-45-6789"), "SSN should be masked");
    // Mask strategy uses literal "[REDACTED]" for each match.
    assert!(
        redacted.contains("[REDACTED]"),
        "mask strategy should insert [REDACTED]"
    );
}

// ---------------------------------------------------------------------------
// Rehydration round-trip
// ---------------------------------------------------------------------------

/// Redact with token_replace, then apply the rehydration map to restore the
/// original text.  This tests the full round-trip without encryption (the
/// encrypt/decrypt layer is tested separately via `redaction-rehydrate`).
#[wasm_bindgen_test]
async fn rehydration_round_trip() {
    let engine = make_engine();

    let result = engine
        .redact(FIXTURE_TEXT, Some("token_replace".to_string()), None)
        .unwrap();
    let obj: js_sys::Object = result.dyn_into().unwrap();

    let redacted = js_sys::Reflect::get(&obj, &JsValue::from_str("redacted"))
        .unwrap()
        .as_string()
        .unwrap();

    let map_val = js_sys::Reflect::get(&obj, &JsValue::from_str("rehydrationMap")).unwrap();
    let map_obj: js_sys::Object = map_val.dyn_into().unwrap();
    let keys = js_sys::Reflect::own_keys(&map_obj).unwrap();

    // Walk the map and substitute each token back into the redacted text.
    let mut restored = redacted.clone();
    for i in 0..keys.length() {
        let key = keys.get(i).as_string().unwrap();
        let original = js_sys::Reflect::get(&map_obj, &JsValue::from_str(&key))
            .unwrap()
            .as_string()
            .unwrap();
        restored = restored.replace(&key, &original);
    }

    assert_eq!(
        restored, FIXTURE_TEXT,
        "rehydrated text must match the original"
    );
}
