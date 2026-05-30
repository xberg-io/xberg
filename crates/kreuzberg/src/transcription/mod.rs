//! Audio/video transcription (speech-to-text) pipeline.
//!
//! This is the internal implementation behind the `transcription` feature.
//! The public surface is the `TranscriptionConfig` (under `transcription-types`)
//! and the automatic routing that happens when an audio/video MIME type is
//! presented to the extractor registry.

pub mod decode;

// Note: decode_audio_to_pcm is currently only used inside the cfg-gated extractor.
// Re-export will be restored when the real decode implementation is wired.
