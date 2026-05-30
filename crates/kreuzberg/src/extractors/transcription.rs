//! Built-in audio/video transcription extractor (speech-to-text).
//!
//! Only compiled when the `transcription` feature is enabled.
//! Registers for the audio and video MIME types declared in `core::mime`.
//!
//! The actual heavy lifting (model download + ORT inference) lives in
//! `crate::transcription`. This module is the thin "plugin" adapter that
//! the registry expects.

use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::transcription::decode::{PcmAudio, decode_audio_to_pcm};
use crate::types::internal::InternalDocument;
use crate::{KreuzbergError, Result};
use async_trait::async_trait;

#[cfg(feature = "transcription")]
use tokio::task;

/// The transcription extractor.
///
/// Priority is the normal default (50). If a user registers a custom
/// higher-priority transcription backend via the plugin system, it will win.
pub struct TranscriptionExtractor;

impl Plugin for TranscriptionExtractor {
    fn name(&self) -> &str {
        "transcription"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        // Nothing heavy at registration time. Model loading is lazy.
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for TranscriptionExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        _mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        // The registry already validated the MIME, but we still need the
        // runtime config block.
        let tcfg = config.transcription.as_ref().filter(|c| c.enabled).ok_or_else(|| {
            KreuzbergError::transcription(
                "Transcription requested for audio/video input, but no `transcription` \
                     config block was provided (or `enabled` is false). \
                     Add `transcription = { enabled = true, model = \"tiny\" }` (or equivalent) \
                     to your ExtractionConfig.",
            )
        })?;

        // Hard size limit (defense in depth — the caller may also have set one).
        if let Some(max_b) = tcfg.max_bytes
            && content.len() as u64 > max_b
        {
            return Err(KreuzbergError::transcription(format!(
                "Input size {} bytes exceeds transcription.max_bytes limit of {}",
                content.len(),
                max_b
            )));
        }

        // Decode is CPU-bound and blocking → run on the blocking pool.
        // We must own the data for the spawned task ('static).
        let bytes_owned = content.to_vec();
        let max_bytes_for_decode = tcfg.max_bytes;
        let pcm: PcmAudio = task::spawn_blocking(move || decode_audio_to_pcm(&bytes_owned, max_bytes_for_decode))
            .await
            .map_err(|e| KreuzbergError::transcription_with_source("Decoder task panicked", e))??; // the inner Result

        // Duration limit (after we know the real duration).
        if let Some(max_dur) = tcfg.max_duration_ms
            && pcm.duration_ms > max_dur
        {
            return Err(KreuzbergError::transcription(format!(
                "Decoded audio duration {} ms exceeds transcription.max_duration_ms limit of {}",
                pcm.duration_ms, max_dur
            )));
        }

        // TODO in follow-up PR: actual Whisper ONNX inference via ORT + ModelCache.
        // For the initial Rust-only implementation we produce a high-quality stub
        // so that the plumbing, limits, error paths, metadata shape, and E2E
        // fixture expectations can be validated immediately.
        // For the absolute initial Rust-only PR we exercise decode + limits + config plumbing.
        // A rich InternalDocument with proper elements will be added when the real engine lands.
        let _transcript_note = format!(
            "Decoded {} samples ({:.1}s). Real Whisper STT coming in follow-up PR.",
            pcm.samples.len(),
            pcm.duration_ms as f64 / 1000.0
        );

        let doc = InternalDocument::new("audio-transcript");
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        // These must exactly match the entries we added to FORMATS in core/mime.rs
        // (plus the aliases the registry already normalizes).
        &[
            "audio/mpeg",
            "audio/mp4",
            "audio/wav",
            "audio/webm",
            "video/mp4",
            "video/webm",
        ]
    }

    fn priority(&self) -> i32 {
        50 // Normal default — users can override with a higher-priority custom plugin
    }
}

impl SyncExtractor for TranscriptionExtractor {
    fn extract_sync(&self, content: &[u8], _mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        // For the sync (WASM / no-tokio) path we still want the same logic.
        // Decode is sync here; the stub path does not require the runtime.
        let tcfg = config.transcription.as_ref().filter(|c| c.enabled).ok_or_else(|| {
            KreuzbergError::transcription("Transcription requested but config missing or disabled (sync path)")
        })?;

        if let Some(max_b) = tcfg.max_bytes
            && content.len() as u64 > max_b
        {
            return Err(KreuzbergError::transcription("Size limit exceeded (sync)"));
        }

        let pcm = decode_audio_to_pcm(content, tcfg.max_bytes)?;

        if let Some(max_d) = tcfg.max_duration_ms
            && pcm.duration_ms > max_d
        {
            return Err(KreuzbergError::transcription("Duration limit exceeded (sync)"));
        }

        // Same stub shape as the async path.
        let _note = format!("{} samples decoded in sync path", pcm.samples.len());
        let doc = InternalDocument::new("audio-transcript");
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::transcription::{TranscriptionConfig, WhisperModel};

    #[test]
    fn test_transcription_extractor_metadata() {
        let ext = TranscriptionExtractor;
        assert_eq!(ext.name(), "transcription");
        assert!(ext.supported_mime_types().contains(&"audio/mpeg"));
        assert!(ext.supported_mime_types().contains(&"video/mp4"));
    }

    #[cfg(feature = "transcription")]
    #[test]
    fn test_transcription_config_defaults_roundtrip() {
        let cfg = TranscriptionConfig {
            model: WhisperModel::Base,
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: TranscriptionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.model, WhisperModel::Base);
    }
}
