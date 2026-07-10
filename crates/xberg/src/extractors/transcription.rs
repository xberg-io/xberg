//! Built-in audio/video transcription extractor (speech-to-text).
//!
//! Only compiled when the `transcription` feature is enabled.
//! Registers for the audio and video MIME types declared in `core::mime`.
//!
//! The actual heavy lifting (model download + ORT inference) lives in
//! `crate::transcription`. This module is the thin "plugin" adapter that
//! the registry expects.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::core::config::ExtractionConfig;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::transcription::decode::{PcmAudio, decode_audio_to_pcm};
use crate::transcription::engine::WhisperEngine;
use crate::transcription::model::{WhisperModelPaths, ensure_whisper_model};
use crate::transcription::tags::AudioTags;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::metadata::{AudioMetadata, FormatMetadata};
use crate::{Result, XbergError};
use async_trait::async_trait;
use tokio::task;

/// Process-wide cache of loaded `WhisperEngine` instances, keyed by the
/// canonical model paths (encoder|tokenizer). Mirrors the pattern in
/// `crate::reranking::get_or_init_engine`.
static ENGINES: LazyLock<Mutex<HashMap<String, Arc<WhisperEngine>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Semaphore that limits the number of concurrent Whisper inference calls.
///
/// The budget matches `resolve_thread_budget` — the same value used by the
/// embedding and reranking semaphores so all ORT inference shares one
/// per-process concurrency bound.
static TRANSCRIPTION_SEMAPHORE: LazyLock<Arc<tokio::sync::Semaphore>> = LazyLock::new(|| {
    let budget = crate::core::config::concurrency::resolve_thread_budget(None);
    Arc::new(tokio::sync::Semaphore::new(budget))
});

/// Cache key for a loaded engine — stable across calls with identical model files.
fn engine_cache_key(paths: &WhisperModelPaths) -> String {
    format!("{}|{}", paths.encoder.display(), paths.tokenizer.display())
}

/// Return a cached `WhisperEngine` for `paths`, building and caching one on
/// the first call for each distinct model.
fn get_or_build_engine(paths: &WhisperModelPaths) -> Result<Arc<WhisperEngine>> {
    let key = engine_cache_key(paths);
    let mut map = ENGINES
        .lock()
        .map_err(|e| XbergError::transcription(format!("engine cache poisoned: {e}")))?;
    if let Some(engine) = map.get(&key) {
        return Ok(Arc::clone(engine));
    }
    let engine = WhisperEngine::load(paths)
        .map_err(|e| XbergError::transcription(format!("whisper engine load failed: {e}")))?;
    let arc = Arc::new(engine);
    map.insert(key, Arc::clone(&arc));
    Ok(arc)
}

/// The transcription extractor.
///
/// Priority is the normal default (50). If a user registers a custom
/// higher-priority transcription backend via the plugin system, it will win.
#[cfg_attr(alef, alef(skip))]
pub struct TranscriptionExtractor;

impl Plugin for TranscriptionExtractor {
    fn name(&self) -> &str {
        "transcription"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for TranscriptionExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let tcfg = config.transcription.as_ref().filter(|c| c.enabled).ok_or_else(|| {
            XbergError::transcription(
                "Transcription requested for audio/video input, but no `transcription` \
                     config block was provided (or `enabled` is false). \
                     Add `transcription = { enabled = true, model = \"tiny\" }` (or equivalent) \
                     to your ExtractionConfig.",
            )
        })?;

        if let Some(max_b) = tcfg.max_bytes
            && content.len() as u64 > max_b
        {
            return Err(XbergError::transcription(format!(
                "Input size {} bytes exceeds transcription.max_bytes limit of {}",
                content.len(),
                max_b
            )));
        }

        let bytes_owned = content.to_vec();
        let max_bytes_for_decode = tcfg.max_bytes;
        let (pcm, tags): (PcmAudio, crate::transcription::tags::AudioTags) = task::spawn_blocking(move || {
            let pcm = decode_audio_to_pcm(&bytes_owned, max_bytes_for_decode)?;
            let tags = crate::transcription::tags::read_audio_tags(&bytes_owned);
            Ok::<_, XbergError>((pcm, tags))
        })
        .await
        .map_err(|e| XbergError::transcription_with_source("Decoder task panicked", e))??;

        if let Some(max_dur) = tcfg.max_duration_ms
            && pcm.duration_ms > max_dur
        {
            return Err(XbergError::transcription(format!(
                "Decoded audio duration {} ms exceeds transcription.max_duration_ms limit of {}",
                pcm.duration_ms, max_dur
            )));
        }

        let paths = {
            let model = tcfg.model;
            let cache_dir = tcfg.model_cache_dir.clone();
            let allow_network = tcfg.allow_network;
            let verify_hash = tcfg.verify_hash;
            task::spawn_blocking(move || ensure_whisper_model(model, cache_dir.as_deref(), allow_network, verify_hash))
                .await
                .map_err(|e| XbergError::transcription(format!("model resolution task panicked: {e}")))?
                .map_err(|e| XbergError::transcription(format!("whisper model resolution failed: {e}")))?
        };

        let engine = get_or_build_engine(&paths)?;

        let _permit = TRANSCRIPTION_SEMAPHORE
            .acquire()
            .await
            .map_err(|e| XbergError::transcription(format!("semaphore closed: {e}")))?;

        let pcm_clone = pcm.clone();
        let lang_clone = tcfg.language.clone();
        let timestamps = tcfg.timestamps;
        let engine_for_task = Arc::clone(&engine);

        let transcript =
            task::spawn_blocking(move || engine_for_task.transcribe(&pcm_clone, lang_clone.as_deref(), timestamps))
                .await
                .map_err(|e| XbergError::transcription(format!("whisper task panicked: {e}")))?
                .map_err(|e| XbergError::transcription(format!("whisper inference failed: {e}")))?;

        let mut doc = build_audio_document(tags, &pcm, mime_type);
        if !transcript.is_empty() {
            doc.push_element(InternalElement::text(ElementKind::Paragraph, &transcript, 0));
        }
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
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
        50
    }
}

#[cfg(test)]
impl TranscriptionExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let tcfg = config.transcription.as_ref().filter(|c| c.enabled).ok_or_else(|| {
            XbergError::transcription(
                "Transcription requested for audio/video input, but no `transcription` \
                 config block was provided (or `enabled` is false). \
                 Add `transcription = { enabled = true, model = \"tiny\" }` (or equivalent) \
                 to your ExtractionConfig.",
            )
        })?;

        if let Some(max_b) = tcfg.max_bytes
            && content.len() as u64 > max_b
        {
            return Err(XbergError::transcription(format!(
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
            return Err(XbergError::transcription(format!(
                "Decoded audio duration {} ms exceeds transcription.max_duration_ms limit of {}",
                pcm.duration_ms, max_d
            )));
        }

        let paths = ensure_whisper_model(
            tcfg.model,
            tcfg.model_cache_dir.as_deref(),
            tcfg.allow_network,
            tcfg.verify_hash,
        )
        .map_err(|e| XbergError::transcription(format!("whisper model resolution failed: {e}")))?;

        let engine = get_or_build_engine(&paths)?;

        let transcript = engine
            .transcribe(&pcm, tcfg.language.as_deref(), tcfg.timestamps)
            .map_err(|e| XbergError::transcription(format!("whisper inference failed: {e}")))?;

        let mut doc = build_audio_document(tags, &pcm, mime_type);
        if !transcript.is_empty() {
            doc.push_element(InternalElement::text(ElementKind::Paragraph, &transcript, 0));
        }
        Ok(doc)
    }
}

/// Construct an [`InternalDocument`] with metadata derived from audio tags and decoded PCM.
///
/// Populates the common [`Metadata`] fields (title, authors, created_at, language) from tag data
/// and attaches an [`AudioMetadata`] carrying codec/container/sample-rate/channel/bitrate info.
/// The caller pushes transcript text as a `Paragraph` element after Whisper inference.
fn build_audio_document(tags: AudioTags, pcm: &PcmAudio, mime_type: &str) -> InternalDocument {
    let audio_meta = AudioMetadata {
        duration_ms: tags.duration_ms.or(Some(pcm.duration_ms)),
        codec: tags.container.clone(),
        container: tags.container,
        sample_rate_hz: tags.sample_rate_hz.or(Some(pcm.sample_rate_hz)),
        channels: tags.channels.or(Some(pcm.channels)),
        bitrate: tags.bitrate,
    };

    let mut doc = InternalDocument::new("audio-transcript");
    doc.mime_type = mime_type.to_string();
    doc.metadata.title = tags.title;
    doc.metadata.authors = tags.artist.map(|a| vec![a]);
    doc.metadata.created_at = tags.year;
    doc.metadata.language = tags.language;
    doc.metadata.format = Some(FormatMetadata::Audio(audio_meta));
    doc
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
        let cfg = ExtractionConfig::default();
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
        let wav_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/audio/silence-1s.wav");
        let bytes = std::fs::read(&wav_path).unwrap_or_else(|e| panic!("missing audio fixture {wav_path:?}: {e}"));

        let ext = TranscriptionExtractor;
        let tcfg = TranscriptionConfig {
            max_duration_ms: Some(0),
            ..Default::default()
        };
        let cfg = config_with_transcription(tcfg);
        let result = ext.extract_sync(&bytes, "audio/wav", &cfg);
        assert!(result.is_err(), "expected error when decoded duration exceeds limit");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("duration") || msg.contains("limit"), "unexpected: {msg}");
    }

    #[tokio::test]
    async fn test_async_no_config_returns_error() {
        let ext = TranscriptionExtractor;
        let cfg = ExtractionConfig::default();
        let result = ext.extract_content(&[], "audio/mpeg", &cfg).await;
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
        let result = ext.extract_content(&oversized, "audio/mpeg", &cfg).await;
        assert!(result.is_err(), "expected error when input exceeds max_bytes (async)");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("exceed") || msg.contains("limit") || msg.contains("size"),
            "unexpected: {msg}"
        );
    }

    fn make_pcm(duration_ms: u64) -> PcmAudio {
        PcmAudio {
            samples: vec![],
            sample_rate_hz: 16_000,
            channels: 1,
            duration_ms,
        }
    }

    #[test]
    fn test_build_audio_document_populates_common_metadata() {
        let tags = AudioTags {
            title: Some("My Song".to_string()),
            artist: Some("Test Artist".to_string()),
            year: Some("2023".to_string()),
            language: Some("eng".to_string()),
            ..Default::default()
        };
        let pcm = make_pcm(90_000);
        let doc = build_audio_document(tags, &pcm, "audio/mpeg");

        assert_eq!(doc.metadata.title.as_deref(), Some("My Song"));
        assert_eq!(doc.metadata.authors.as_deref(), Some(&["Test Artist".to_string()][..]));
        assert_eq!(doc.metadata.created_at.as_deref(), Some("2023"));
        assert_eq!(doc.metadata.language.as_deref(), Some("eng"));
        assert_eq!(doc.mime_type, "audio/mpeg");
    }

    #[test]
    fn test_build_audio_document_populates_audio_format_metadata() {
        use crate::types::metadata::FormatMetadata;

        let tags = AudioTags {
            duration_ms: Some(30_000),
            sample_rate_hz: Some(44_100),
            channels: Some(2),
            bitrate: Some(320),
            container: Some("mp3".to_string()),
            ..Default::default()
        };
        let pcm = make_pcm(30_000);
        let doc = build_audio_document(tags, &pcm, "audio/mpeg");

        let Some(FormatMetadata::Audio(ref audio)) = doc.metadata.format else {
            panic!("expected FormatMetadata::Audio, got {:?}", doc.metadata.format);
        };
        assert_eq!(audio.duration_ms, Some(30_000));
        assert_eq!(audio.sample_rate_hz, Some(44_100));
        assert_eq!(audio.channels, Some(2));
        assert_eq!(audio.bitrate, Some(320));
        assert_eq!(audio.container.as_deref(), Some("mp3"));
    }

    #[test]
    fn test_build_audio_document_falls_back_to_pcm_properties() {
        use crate::types::metadata::FormatMetadata;

        let tags = AudioTags::default();
        let pcm = make_pcm(60_000);
        let doc = build_audio_document(tags, &pcm, "audio/wav");

        let Some(FormatMetadata::Audio(ref audio)) = doc.metadata.format else {
            panic!("expected FormatMetadata::Audio");
        };
        assert_eq!(
            audio.duration_ms,
            Some(60_000),
            "duration should fall back to PCM value"
        );
        assert_eq!(
            audio.sample_rate_hz,
            Some(16_000),
            "sample_rate should fall back to PCM value"
        );
        assert_eq!(audio.channels, Some(1), "channels should fall back to PCM value");
    }

    #[test]
    fn test_build_audio_document_empty_tags_no_common_metadata() {
        let tags = AudioTags::default();
        let pcm = make_pcm(0);
        let doc = build_audio_document(tags, &pcm, "audio/flac");

        assert!(doc.metadata.title.is_none(), "title should be absent for untagged file");
        assert!(
            doc.metadata.authors.is_none(),
            "authors should be absent for untagged file"
        );
        assert!(
            doc.metadata.created_at.is_none(),
            "created_at should be absent for untagged file"
        );
        assert!(
            doc.metadata.language.is_none(),
            "language should be absent for untagged file"
        );
    }
}
