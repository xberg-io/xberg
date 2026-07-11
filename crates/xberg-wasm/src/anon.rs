//! Anonymisation helpers — encrypt / decrypt rehydration maps.
//!
//! These are thin wrappers over
//! [`xberg::text::redaction::rehydration`] that return raw `Vec<u8>`
//! (encrypted blob) or `JsValue` (the decrypted `HashMap`).
//!
//! Gated on the `redaction-rehydrate` feature.

use std::collections::HashMap;

use wasm_bindgen::prelude::*;

/// Encrypt a token→original map with `passphrase`.
///
/// Returns the raw ciphertext bytes (`XPII\x01` wire format).
#[allow(clippy::missing_errors_doc)]
#[wasm_bindgen(js_name = "encryptRehydrationMap")]
pub fn encrypt_rehydration_map(map: JsValue, passphrase: &str) -> Result<Vec<u8>, JsValue> {
    let inner: HashMap<String, String> =
        serde_wasm_bindgen::from_value(map).map_err(|e| JsValue::from_str(&e.to_string()))?;
    xberg::text::redaction::rehydration::encrypt_map(&inner, passphrase).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Decrypt an encrypted blob back into a token→original map.
#[allow(clippy::missing_errors_doc)]
#[wasm_bindgen(js_name = "decryptRehydrationMap")]
pub fn decrypt_rehydration_map(blob: Vec<u8>, passphrase: &str) -> Result<JsValue, JsValue> {
    let inner = xberg::text::redaction::rehydration::decrypt_map(&blob, passphrase)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&inner).map_err(|e| JsValue::from_str(&e.to_string()))
}
