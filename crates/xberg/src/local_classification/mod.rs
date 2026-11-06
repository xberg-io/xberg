//! Experimental Rust-only local NLI classification from explicitly pre-staged immutable artifacts.
//! The experimental API may change between releases.

#[cfg(feature = "classification-onnx")]
mod artifacts;
#[cfg(feature = "classification-onnx")]
mod control;
#[cfg(feature = "classification-onnx")]
mod decoding;
#[cfg(feature = "classification-onnx")]
mod execution;
#[cfg(feature = "classification-onnx")]
mod metadata;
#[cfg(feature = "classification-onnx")]
mod policy;
#[cfg(feature = "classification-onnx")]
mod preparation;
#[cfg(feature = "classification-onnx")]
mod runtime;
#[cfg(feature = "classification-onnx")]
mod schema;
#[cfg(feature = "classification-onnx")]
mod tokenization;
mod types;

#[cfg(feature = "classification-onnx")]
pub use runtime::LocalTextClassifier;

pub use types::{
    LocalClassificationBackend, LocalClassificationConfig, LocalClassificationLimits, LocalClassificationModelInfo,
    LocalClassificationOptions, LocalClassificationResult, LocalNliModel,
};

#[cfg(all(test, feature = "classification-onnx"))]
mod tests;

#[cfg(all(test, feature = "classification-onnx"))]
mod control_tests;
#[cfg(all(test, feature = "classification-onnx"))]
mod metadata_tests;
#[cfg(all(test, feature = "classification-onnx"))]
mod runtime_tests;
#[cfg(all(test, feature = "classification-onnx"))]
mod tokenization_tests;
#[cfg(all(test, feature = "classification-onnx"))]
mod toy_model;

#[cfg(all(test, feature = "classification-onnx"))]
mod external_memory_tests;
#[cfg(all(test, feature = "classification-onnx"))]
mod preparation_tests;
#[cfg(all(test, feature = "classification-onnx"))]
mod schema_tests;

#[cfg(all(test, feature = "classification-onnx"))]
mod artifacts_tests;
