//! Audio/video transcription (speech-to-text) pipeline.
//!
//! This is the internal implementation behind the `transcription` feature.
//! The public surface is the `TranscriptionConfig` (under `transcription-types`)
//! and the automatic routing that happens when an audio/video MIME type is
//! presented to the extractor registry.

pub mod decode;
#[cfg(feature = "transcription")]
pub mod tags;
