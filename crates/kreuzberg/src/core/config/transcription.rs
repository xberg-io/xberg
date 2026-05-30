//! Transcription configuration for audio/video speech-to-text.
//!
//! This module is behind the `transcription-types` feature for the pure-Rust
//! config structs (safe on WASM/Android) and the `transcription` feature for
//! the full ORT + decode implementation.
//!
//! Design follows the exact established pattern of `EmbeddingConfig`,
//! `LayoutDetectionConfig`, and `PaddleOcrConfig`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for audio/video transcription (speech-to-text).
///
/// When present and `enabled`, Kreuzberg will route audio and video files
/// (mp3, mp4, m4a, wav, webm, etc.) through the transcription pipeline.
///
/// The heavy dependencies (ORT, hf-hub, symphonia) are only pulled when the
/// `transcription` feature is enabled. The config struct itself is available
/// under `transcription-types` so that `ExtractionConfig` round-trips on all
/// targets.
///
/// All fields have sensible defaults. The recommended starting point is:
///
/// ```toml
/// [extraction.transcription]
/// enabled = true
/// model = "tiny"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Master switch. When false the block is ignored and audio files fall back
    /// to the normal "unsupported format" path.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whisper model size to use.
    ///
    /// Smaller = faster + lower memory. `tiny` is the pragmatic default for
    /// first-time users and CI.
    #[serde(default)]
    pub model: WhisperModel,

    /// Optional language hint (ISO-639-1 code, e.g. "en", "de").
    ///
    /// When `None` (default) the engine may attempt auto-detection if supported.
    /// For deterministic production output, always set this explicitly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Whether to emit segment-level timestamps in the result metadata.
    ///
    /// When true, `metadata["transcription.segments"]` will contain an array
    /// of `{start_ms, end_ms, text}` objects (if the engine supports it).
    #[serde(default)]
    pub timestamps: bool,

    /// Hard safety limit on input duration (milliseconds).
    ///
    /// Files longer than this are rejected *before* any decode or model work.
    /// Default: 30 minutes. Set to `None` to disable (not recommended for
    /// untrusted input).
    #[serde(default = "default_max_duration_ms")]
    pub max_duration_ms: Option<u64>,

    /// Hard safety limit on input size (bytes).
    ///
    /// Default: 512 MiB. Protects against pathological or malicious uploads.
    #[serde(default = "default_max_bytes")]
    pub max_bytes: Option<u64>,

    /// Wall-clock timeout for the entire transcription operation (ms).
    ///
    /// Includes model download (first time), decode, and inference.
    /// Default: 10 minutes. Uses `tokio::select!` so the async runtime is
    /// never blocked.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: Option<u64>,

    /// Override the directory used for Whisper model cache.
    ///
    /// When `None`, uses the centralized resolver:
    /// `KREUZBERG_CACHE_DIR/transcription/whisper` or the platform default
    /// (`~/.cache/kreuzberg/transcription/whisper` on Linux, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_cache_dir: Option<PathBuf>,

    /// Allow network access to download models from Hugging Face Hub.
    ///
    /// When `false`, only previously cached models may be used. Useful for
    /// air-gapped or fully offline deployments.
    #[serde(default = "default_true")]
    pub allow_network: bool,

    /// Verify SHA256 checksums of downloaded model files (when known).
    ///
    /// Strongly recommended; disable only for debugging.
    #[serde(default = "default_true")]
    pub verify_hash: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: WhisperModel::default(),
            language: None,
            timestamps: false,
            max_duration_ms: default_max_duration_ms(),
            max_bytes: default_max_bytes(),
            timeout_ms: default_timeout_ms(),
            model_cache_dir: None,
            allow_network: true,
            verify_hash: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_duration_ms() -> Option<u64> {
    Some(30 * 60 * 1000) // 30 minutes
}

fn default_max_bytes() -> Option<u64> {
    Some(512 * 1024 * 1024) // 512 MiB
}

fn default_timeout_ms() -> Option<u64> {
    Some(10 * 60 * 1000) // 10 minutes
}

/// Supported Whisper model sizes.
///
/// These map to published ONNX exports on Hugging Face (onnx-community or
/// similar orgs). The actual filenames and repos are resolved inside the
/// transcription engine.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModel {
    /// ~39 MB, fastest, lowest quality. Good default for development and CI.
    #[default]
    Tiny,
    /// ~74 MB, reasonable quality/speed tradeoff.
    Base,
    /// ~244 MB, better accuracy.
    Small,
    /// ~769 MB, high quality (slower, more memory).
    Medium,
    /// ~1550 MB, best quality (large-v3). Use only when latency is acceptable.
    LargeV3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_sensible() {
        let cfg = TranscriptionConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.model, WhisperModel::Tiny);
        assert!(cfg.language.is_none());
        assert!(cfg.max_duration_ms.unwrap() > 1_000_000);
        assert!(cfg.allow_network);
    }

    #[test]
    fn test_serde_roundtrip_minimal() {
        let json = r#"{"enabled": true, "model": "base", "timestamps": true}"#;
        let cfg: TranscriptionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.model, WhisperModel::Base);
        assert!(cfg.timestamps);

        let back = serde_json::to_string(&cfg).unwrap();
        assert!(back.contains("\"model\":\"base\""));
        assert!(back.contains("\"timestamps\":true"));
    }

    #[test]
    fn test_serde_omits_none_fields() {
        let cfg = TranscriptionConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        // language and cache_dir should be omitted when None
        assert!(!json.contains("language"));
        assert!(!json.contains("model_cache_dir"));
    }
}
