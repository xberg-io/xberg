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
        mime_type: &str,
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

        // Decode PCM and read audio tags on the blocking pool in one pass.
        let bytes_owned = content.to_vec();
        let max_bytes_for_decode = tcfg.max_bytes;
        let (pcm, tags): (PcmAudio, crate::transcription::tags::AudioTags) =
            task::spawn_blocking(move || {
                let pcm = decode_audio_to_pcm(&bytes_owned, max_bytes_for_decode)?;
                let tags = crate::transcription::tags::read_audio_tags(&bytes_owned);
                Ok::<_, KreuzbergError>((pcm, tags))
            })
            .await
            .map_err(|e| KreuzbergError::transcription_with_source("Decoder task panicked", e))??;

        // Duration limit (after we know the real duration).
        if let Some(max_dur) = tcfg.max_duration_ms
            && pcm.duration_ms > max_dur
        {
            return Err(KreuzbergError::transcription(format!(
                "Decoded audio duration {} ms exceeds transcription.max_duration_ms limit of {}",
                pcm.duration_ms, max_dur
            )));
        }

        // Whisper ONNX inference is wired in the follow-up PR.
        // Tags and PCM metadata are already populated above — the follow-up PR replaces
        // this Err with the inference call and moves the doc construction to Ok(doc).
        let _ = (pcm, tags);
        Err(KreuzbergError::transcription(
            "Whisper inference not yet implemented — pending follow-up PR",
        ))
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

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

impl SyncExtractor for TranscriptionExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        // Sync path used when no tokio runtime is available.
        let tcfg = config.transcription.as_ref().filter(|c| c.enabled).ok_or_else(|| {
            KreuzbergError::transcription(
                "Transcription requested for audio/video input, but no `transcription` \
                 config block was provided (or `enabled` is false). \
                 Add `transcription = { enabled = true, model = \"tiny\" }` (or equivalent) \
                 to your ExtractionConfig.",
            )
        })?;

        if let Some(max_b) = tcfg.max_bytes
            && content.len() as u64 > max_b
        {
            return Err(KreuzbergError::transcription(format!(
                "Input size {} bytes exceeds transcription.max_bytes limit of {}",
                content.len(),
                max_b
            )));
        }

        let pcm = decode_audio_to_pcm(content, tcfg.max_bytes)?;
        let tags = crate::transcription::tags::read_audio_tags(content);

        if let Some(max_d) = tcfg.max_duration_ms
            && pcm.duration_ms > max_d
        {
            return Err(KreuzbergError::transcription(format!(
                "Decoded audio duration {} ms exceeds transcription.max_duration_ms limit of {}",
                pcm.duration_ms, max_d
            )));
        }

        // Whisper inference is wired in the follow-up PR.
        let _ = (pcm, tags);
        Err(KreuzbergError::transcription(
            "Whisper inference not yet implemented — pending follow-up PR",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ExtractionConfig;
    use crate::core::config::transcription::{TranscriptionConfig, WhisperModel};

    #[test]
    fn test_transcription_extractor_metadata() {
        let ext = TranscriptionExtractor;
        assert_eq!(ext.name(), "transcription");
        assert!(ext.supported_mime_types().contains(&"audio/mpeg"));
        assert!(ext.supported_mime_types().contains(&"video/mp4"));
    }

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

    fn config_with_transcription(tcfg: TranscriptionConfig) -> ExtractionConfig {
        ExtractionConfig {
            transcription: Some(tcfg),
            ..Default::default()
        }
    }

    #[test]
    fn test_sync_no_config_returns_error() {
        let ext = TranscriptionExtractor;
        let cfg = ExtractionConfig::default(); // no transcription block
        let result = ext.extract_sync(&[], "audio/mpeg", &cfg);
        assert!(result.is_err(), "expected error when no transcription config");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("config") || msg.contains("disabled"), "unexpected: {msg}");
    }

    #[test]
    fn test_sync_disabled_config_returns_error() {
        let ext = TranscriptionExtractor;
        let tcfg = TranscriptionConfig {
            enabled: false,
            ..Default::default()
        };
        let cfg = config_with_transcription(tcfg);
        let result = ext.extract_sync(&[], "audio/mpeg", &cfg);
        assert!(result.is_err(), "expected error when transcription disabled");
    }

    #[test]
    fn test_sync_size_limit_enforced() {
        let ext = TranscriptionExtractor;
        let tcfg = TranscriptionConfig {
            max_bytes: Some(10),
            ..Default::default()
        };
        let cfg = config_with_transcription(tcfg);
        let oversized = vec![0u8; 11];
        let result = ext.extract_sync(&oversized, "audio/mpeg", &cfg);
        assert!(result.is_err(), "expected error when input exceeds max_bytes");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("exceed") || msg.contains("limit") || msg.contains("size"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn test_sync_duration_limit_enforced() {
        let ext = TranscriptionExtractor;
        // Decode stub returns duration_ms based on byte count; use a tiny limit to trigger it.
        let tcfg = TranscriptionConfig {
            max_duration_ms: Some(0),
            ..Default::default()
        };
        let cfg = config_with_transcription(tcfg);
        let result = ext.extract_sync(&[0u8; 16], "audio/mpeg", &cfg);
        assert!(result.is_err(), "expected error when decoded duration exceeds limit");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("duration") || msg.contains("limit"), "unexpected: {msg}");
    }

    #[tokio::test]
    async fn test_async_no_config_returns_error() {
        let ext = TranscriptionExtractor;
        let cfg = ExtractionConfig::default();
        let result = ext.extract_bytes(&[], "audio/mpeg", &cfg).await;
        assert!(result.is_err(), "expected error when no transcription config (async)");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("config") || msg.contains("disabled"), "unexpected: {msg}");
    }

    #[tokio::test]
    async fn test_async_size_limit_enforced() {
        let ext = TranscriptionExtractor;
        let tcfg = TranscriptionConfig {
            max_bytes: Some(10),
            ..Default::default()
        };
        let cfg = config_with_transcription(tcfg);
        let oversized = vec![0u8; 11];
        let result = ext.extract_bytes(&oversized, "audio/mpeg", &cfg).await;
        assert!(result.is_err(), "expected error when input exceeds max_bytes (async)");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("exceed") || msg.contains("limit") || msg.contains("size"),
            "unexpected: {msg}"
        );
    }

    #[tokio::test]
    async fn test_async_stub_returns_inference_not_implemented_error() {
        let ext = TranscriptionExtractor;
        let cfg = config_with_transcription(TranscriptionConfig::default());
        // Before Whisper inference is wired, the extractor must return an explicit error
        // rather than silently returning an empty document.
        let result = ext.extract_bytes(&[0u8; 64], "audio/mpeg", &cfg).await;
        assert!(result.is_err(), "expected inference-not-implemented error");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not yet implemented") || msg.contains("follow-up"),
            "unexpected error message: {msg}"
        );
    }
}
