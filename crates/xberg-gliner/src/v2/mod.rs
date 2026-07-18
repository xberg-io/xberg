//! GLiNER2 (schema-prompt) support. "v2" throughout this module refers to
//! the second-generation GLiNER2 model architecture (the `fastino/gliner2`
//! reference: schema-prompt encoding, hardcoded batch size of 1), not to a
//! revision of this crate. The unversioned sibling modules at the crate root
//! implement the original span-mode GLiNER.
//!
//! `preprocess`, `splitter`, and `tokenizer` are backend-agnostic (also
//! consumed by the Candle implementation in the `candle` module); the
//! rest drive the ONNX Runtime engine and are gated on `ort-backend`.

#[cfg(feature = "ort-backend")]
pub(crate) mod decode;
#[cfg(feature = "ort-backend")]
pub(crate) mod engine;
pub(crate) mod preprocess;
#[cfg(feature = "ort-backend")]
pub(crate) mod session;
pub(crate) mod splitter;
#[cfg(feature = "ort-backend")]
pub(crate) mod tensor;
pub(crate) mod tokenizer;
