//! Experimental Rust-only local NLI classification from explicitly pre-staged immutable artifacts.
//! The experimental API may change between releases.
//!
//! This adapter is zero-shot: the request supplies arbitrary labels and a hypothesis template, and
//! the staged NLI cross-encoder scores every premise/hypothesis pair. A label is not a trained
//! class. [`LocalClassificationMode::TrainedTaxonomy`] is a different backend over a
//! classification head with its own label set, and [`LocalTextClassifier::classify_trained_taxonomy`]
//! rejects it while no fixed-taxonomy candidate is staged, rather than implying a taxonomy from
//! caller labels.
//!
//! The NLI-MiniLM2 candidate is unqualified and is selected only through
//! [`LocalNliModel::staged`] or an explicit caller-pinned descriptor. It is not a default, and
//! the provider-backed [`crate::text`] classification path is unchanged.

#[cfg(feature = "classification-onnx")]
mod artifacts;
#[cfg(feature = "classification-onnx")]
mod control;
#[cfg(feature = "classification-onnx")]
mod decoding;
#[cfg(feature = "classification-onnx")]
mod execution;
#[cfg(feature = "classification-onnx")]
mod inspection;
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
pub use inspection::{
    LocalClassificationArtifact, LocalClassificationArtifactReport, LocalClassificationArtifactState,
};
#[cfg(feature = "classification-onnx")]
pub use runtime::LocalTextClassifier;

pub use types::{
    LocalClassificationBackend, LocalClassificationConfig, LocalClassificationLimits, LocalClassificationMode,
    LocalClassificationModelInfo, LocalClassificationOptions, LocalClassificationPrecision, LocalClassificationResult,
    LocalNliModel,
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

#[cfg(all(test, feature = "classification-onnx"))]
mod candidate_tests;
