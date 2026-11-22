//! Held-out scoring for CPU ONNX candidate qualification (#2334).
//!
//! The package does not choose defaults, presets or bundles. It turns a frozen corpus, a
//! preregistered bar and an SDK-produced inference report into measured numbers and a per-workload
//! verdict, and it refuses to score work it cannot account for.
//!
//! Portable builds audit manifests, build and check preregistrations, score a supplied report and
//! exercise refusal controls. The `runtime` feature adds the only code that loads a model: the
//! plan builders that derive descriptors from the canonical inventory and the two runners that
//! call `LocalTextClassifier` and `LocalImageCaptioner` through the SDK.

pub mod caption;
pub mod controls;
pub mod corpus;
pub mod decision;
pub mod metrics;
pub mod prereg;
#[cfg(feature = "runtime")]
pub mod runtime;
pub mod score;
pub use nli_qualification::files;

use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Canonical JSON: object keys sorted recursively, no whitespace. The acquisition scripts and
/// this crate must agree on one byte order for a corpus digest, and struct field order is not it.
pub fn canonical_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&canonical_value(&serde_json::to_value(value)?))?)
}

pub fn canonical_digest<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(digest(&canonical_json(value)?))
}

fn canonical_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut object = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                object.insert(key.clone(), canonical_value(&map[key]));
            }
            serde_json::Value::Object(object)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(items.iter().map(canonical_value).collect()),
        other => other.clone(),
    }
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned().into())
    }
}

pub(crate) fn hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}
