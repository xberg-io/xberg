//! Post-processing and chunking configuration.
//!
//! Defines configuration for post-processing pipelines, text chunking,
//! and embedding generation.

use ahash::AHashSet;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Type of text chunker to use.
///
/// # Variants
///
/// * `Text` - Generic text splitter, splits on whitespace and punctuation
/// * `Markdown` - Markdown-aware splitter, preserves formatting and structure
/// * `Yaml` - YAML-aware splitter, creates one chunk per top-level key
/// * `Semantic` - Topic-aware chunker. With an `EmbeddingConfig`, splits at
///   embedding-based topic shifts tuned by `topic_threshold` (default 0.75,
///   lower = more splits). Without an embedding, falls back to a
///   structural-boundary heuristic (ALL-CAPS headers, numbered sections,
///   blank-line paragraphs) and merges groups into chunks capped at
///   `max_characters` (default 1000). `topic_threshold` has no effect in the
///   fallback path. For best results, pair with an embedding model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChunkerType {
    #[default]
    Text,
    Markdown,
    Yaml,
    Semantic,
}

/// How chunk size is measured.
///
/// Defaults to `Characters` (Unicode character count). When using token-based sizing,
/// chunks are sized by token count according to the specified tokenizer.
///
/// Token-based sizing uses HuggingFace tokenizers loaded at runtime. Any tokenizer
/// available on HuggingFace Hub can be used, including OpenAI-compatible tokenizers
/// (e.g., `Xenova/gpt-4o`, `Xenova/cl100k_base`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChunkSizing {
    /// Size measured in Unicode characters (default).
    #[default]
    Characters,
    /// Size measured in tokens from a HuggingFace tokenizer.
    #[cfg(feature = "chunking-tokenizers")]
    Tokenizer {
        /// HuggingFace model ID or path, e.g. "Xenova/gpt-4o", "bert-base-uncased".
        model: String,
        /// Optional cache directory override for tokenizer files.
        /// Defaults to hf-hub's standard cache (`~/.cache/huggingface/`).
        /// Can also be set via `KREUZBERG_TOKENIZER_CACHE_DIR` environment variable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_dir: Option<std::path::PathBuf>,
    },
}

/// Post-processor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessorConfig {
    /// Enable post-processors
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whitelist of processor names to run (None = all enabled)
    #[serde(default)]
    pub enabled_processors: Option<Vec<String>>,

    /// Blacklist of processor names to skip (None = none disabled)
    #[serde(default)]
    pub disabled_processors: Option<Vec<String>>,

    /// Pre-computed AHashSet for O(1) enabled processor lookup
    #[serde(skip)]
    pub enabled_set: Option<AHashSet<String>>,

    /// Pre-computed AHashSet for O(1) disabled processor lookup
    #[serde(skip)]
    pub disabled_set: Option<AHashSet<String>>,
}

impl PostProcessorConfig {
    /// Pre-compute HashSets for O(1) processor name lookups.
    ///
    /// This method converts the enabled/disabled processor Vec to HashSet
    /// for constant-time lookups in the pipeline.
    #[cfg(test)]
    pub(crate) fn build_lookup_sets(&mut self) {
        if let Some(ref enabled) = self.enabled_processors {
            self.enabled_set = Some(enabled.iter().cloned().collect());
        }
        if let Some(ref disabled) = self.disabled_processors {
            self.disabled_set = Some(disabled.iter().cloned().collect());
        }
    }
}

impl Default for PostProcessorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enabled_processors: None,
            disabled_processors: None,
            enabled_set: None,
            disabled_set: None,
        }
    }
}

/// Chunking configuration.
///
/// Configures text chunking for document content, including chunk size,
/// overlap, trimming behavior, and optional embeddings.
///
/// Use `..Default::default()` when constructing to allow for future field additions:
/// ```rust
/// # use kreuzberg::ChunkingConfig;
/// let config = ChunkingConfig {
///     max_characters: 500,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Maximum size per chunk (in units determined by `sizing`).
    ///
    /// When `sizing` is `Characters` (default), this is the max character count.
    /// When using token-based sizing, this is the max token count.
    ///
    /// Default: 1000
    #[serde(default = "default_chunk_size", rename = "max_chars", alias = "max_characters")]
    pub max_characters: usize,

    /// Overlap between chunks (in units determined by `sizing`).
    ///
    /// Default: 200
    #[serde(default = "default_chunk_overlap", rename = "max_overlap", alias = "overlap")]
    pub overlap: usize,

    /// Whether to trim whitespace from chunk boundaries.
    ///
    /// Default: true
    #[serde(default = "default_trim")]
    pub trim: bool,

    /// Type of chunker to use (Text or Markdown).
    ///
    /// Default: Text
    #[serde(default = "default_chunker_type")]
    pub chunker_type: ChunkerType,

    /// Optional embedding configuration for chunk embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<EmbeddingConfig>,

    /// Use a preset configuration (overrides individual settings if provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,

    /// How to measure chunk size.
    ///
    /// Default: `Characters` (Unicode character count).
    /// Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing.
    #[serde(default)]
    pub sizing: ChunkSizing,

    /// When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy
    /// path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string.
    ///
    /// This is useful for RAG pipelines where each chunk needs self-contained
    /// context about its position in the document structure.
    ///
    /// Default: `false`
    #[serde(default)]
    pub prepend_heading_context: bool,

    /// Optional cosine similarity threshold for semantic topic boundary detection.
    ///
    /// Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is
    /// provided. You almost never need to set this. When omitted, defaults to
    /// `0.75` which works well for most documents. Lower values detect more
    /// topic boundaries (more, smaller chunks); higher values detect fewer.
    /// Range: `0.0..=1.0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_threshold: Option<f32>,
}

impl ChunkingConfig {
    /// Create a new `ChunkingConfig` with the given max characters, overlap, and trim settings.
    ///
    /// Other fields are set to their defaults. Use the setter methods to customize further.
    #[cfg(test)]
    pub(crate) fn new(max_characters: usize, overlap: usize, trim: bool) -> Self {
        Self {
            max_characters,
            overlap,
            trim,
            chunker_type: ChunkerType::Text,
            embedding: None,
            preset: None,
            sizing: ChunkSizing::default(),
            prepend_heading_context: false,
            topic_threshold: None,
        }
    }

    /// Set the chunker type.
    #[cfg(test)]
    pub(crate) fn with_chunker_type(mut self, chunker_type: ChunkerType) -> Self {
        self.chunker_type = chunker_type;
        self
    }

    /// Set the sizing strategy.
    #[cfg(test)]
    pub(crate) fn with_sizing(mut self, sizing: ChunkSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// Enable or disable prepending heading context to chunk content.
    #[cfg(test)]
    pub(crate) fn with_prepend_heading_context(mut self, prepend: bool) -> Self {
        self.prepend_heading_context = prepend;
        self
    }

    /// Set the cosine similarity threshold for semantic topic boundary detection.
    ///
    /// # Panics
    ///
    /// Panics if `threshold` is outside `[0.0, 1.0]`.
    #[cfg(test)]
    pub(crate) fn with_topic_threshold(mut self, threshold: f32) -> Self {
        assert!(
            (0.0..=1.0).contains(&threshold),
            "topic_threshold must be in [0.0, 1.0], got {threshold}"
        );
        self.topic_threshold = Some(threshold);
        self
    }

    /// Resolve a preset name into concrete chunking and embedding configuration.
    ///
    /// When `preset` is set (e.g., `"balanced"`), this overrides `max_characters` and
    /// `overlap` from the preset definition, and configures the embedding model if
    /// no embedding config was explicitly provided.
    ///
    /// If the preset name is not recognized, a warning is logged and the config
    /// is returned unchanged.
    ///
    /// Requires the `embeddings` feature. Without it, this is a no-op that returns
    /// the config unchanged.
    #[cfg(feature = "embeddings")]
    pub(crate) fn resolve_preset(&self) -> Self {
        let preset_name = match &self.preset {
            Some(name) => name,
            None => return self.clone(),
        };

        let preset = match crate::embeddings::get_preset(preset_name) {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "Unknown chunking preset '{}', using manual config. Available: {:?}",
                    preset_name,
                    crate::embeddings::list_presets()
                );
                return self.clone();
            }
        };

        let embedding = match &self.embedding {
            Some(existing) => Some(existing.clone()),
            None => Some(EmbeddingConfig {
                model: EmbeddingModelType::Preset {
                    name: preset_name.clone(),
                },
                ..EmbeddingConfig::default()
            }),
        };

        Self {
            max_characters: preset.chunk_size,
            overlap: preset.overlap,
            embedding,
            // Preserve caller's other settings
            trim: self.trim,
            chunker_type: self.chunker_type,
            preset: self.preset.clone(),
            sizing: self.sizing.clone(),
            prepend_heading_context: self.prepend_heading_context,
            topic_threshold: self.topic_threshold,
        }
    }

    /// Resolve a preset name (no-op without the `embeddings` feature).
    #[cfg(not(feature = "embeddings"))]
    pub(crate) fn resolve_preset(&self) -> Self {
        if self.preset.is_some() {
            tracing::warn!("Chunking presets require the 'embeddings' feature");
        }
        self.clone()
    }
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_characters: 1000,
            overlap: 200,
            trim: true,
            chunker_type: ChunkerType::Text,
            embedding: None,
            preset: None,
            sizing: ChunkSizing::default(),
            prepend_heading_context: false,
            topic_threshold: None,
        }
    }
}

/// Embedding configuration for text chunks.
///
/// Configures embedding generation using ONNX models via the vendored embedding engine.
/// Requires the `embeddings` feature to be enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// The embedding model to use (defaults to "balanced" preset if not specified)
    #[serde(default = "default_model")]
    pub model: EmbeddingModelType,

    /// Whether to normalize embedding vectors (recommended for cosine similarity)
    #[serde(default = "default_normalize")]
    pub normalize: bool,

    /// Batch size for embedding generation
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Show model download progress
    #[serde(default)]
    pub show_download_progress: bool,

    /// Custom cache directory for model files
    ///
    /// Defaults to `~/.cache/kreuzberg/embeddings/` if not specified.
    /// Allows full customization of model download location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,

    /// Hardware acceleration for the embedding ONNX model.
    ///
    /// When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT)
    /// is used for inference. Defaults to `None` (auto-select per platform).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceleration: Option<super::acceleration::AccelerationConfig>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: EmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
            normalize: true,
            batch_size: 32,
            show_download_progress: false,
            cache_dir: None,
            acceleration: None,
        }
    }
}

/// Embedding model types supported by Kreuzberg.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbeddingModelType {
    /// Use a preset model configuration (recommended)
    Preset { name: String },

    /// Use a custom ONNX model from HuggingFace
    Custom { model_id: String, dimensions: usize },

    /// Provider-hosted embedding model via liter-llm.
    ///
    /// Uses the model specified in the nested `LlmConfig` (e.g.,
    /// `"openai/text-embedding-3-small"`).
    Llm { llm: super::llm::LlmConfig },
}

impl Default for EmbeddingModelType {
    fn default() -> Self {
        Self::Preset { name: String::new() }
    }
}

fn default_true() -> bool {
    true
}

fn default_chunk_size() -> usize {
    1000
}

fn default_chunk_overlap() -> usize {
    200
}

fn default_trim() -> bool {
    true
}

fn default_chunker_type() -> ChunkerType {
    ChunkerType::Text
}

fn default_normalize() -> bool {
    true
}

fn default_batch_size() -> usize {
    32
}

fn default_model() -> EmbeddingModelType {
    EmbeddingModelType::Preset {
        name: "balanced".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postprocessor_config_default() {
        let config = PostProcessorConfig::default();
        assert!(config.enabled);
        assert!(config.enabled_processors.is_none());
        assert!(config.disabled_processors.is_none());
    }

    #[test]
    fn test_postprocessor_config_build_lookup_sets() {
        let mut config = PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec!["a".to_string(), "b".to_string()]),
            disabled_processors: Some(vec!["c".to_string()]),
            enabled_set: None,
            disabled_set: None,
        };

        config.build_lookup_sets();

        assert!(config.enabled_set.is_some());
        assert!(config.disabled_set.is_some());
        assert!(config.enabled_set.unwrap().contains("a"));
        assert!(config.disabled_set.unwrap().contains("c"));
    }

    #[test]
    fn test_chunking_config_defaults() {
        let config = ChunkingConfig::default();
        assert_eq!(config.max_characters, 1000);
        assert_eq!(config.overlap, 200);
        assert!(config.trim);
        assert_eq!(config.chunker_type, ChunkerType::Text);
        assert!(matches!(config.sizing, ChunkSizing::Characters));
    }

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert!(config.normalize);
        assert_eq!(config.batch_size, 32);
        assert!(config.cache_dir.is_none());
    }

    /// Tests that EmbeddingModelType::Preset serializes with "type" field (internally-tagged).
    /// This validates the API schema matches the documented format:
    /// `{"type": "preset", "name": "fast"}` NOT `{"preset": {"name": "fast"}}`
    #[test]
    fn test_embedding_model_type_preset_serialization() {
        let model = EmbeddingModelType::Preset {
            name: "fast".to_string(),
        };
        let json = serde_json::to_string(&model).unwrap();

        // Should use internally-tagged format with "type" discriminator
        assert!(json.contains(r#""type":"preset""#), "Should contain type:preset field");
        assert!(json.contains(r#""name":"fast""#), "Should contain name:fast field");

        // Should NOT use adjacently-tagged format
        assert!(
            !json.contains(r#"{"preset":"#),
            "Should NOT use adjacently-tagged format"
        );
    }

    /// Tests that EmbeddingModelType::Preset deserializes from the documented API format.
    /// API documentation shows: `{"type": "preset", "name": "fast"}`
    #[test]
    fn test_embedding_model_type_preset_deserialization() {
        // This is the documented API format that users should send
        let json = r#"{"type": "preset", "name": "fast"}"#;
        let model: EmbeddingModelType = serde_json::from_str(json).unwrap();

        match model {
            EmbeddingModelType::Preset { name } => {
                assert_eq!(name, "fast");
            }
            _ => panic!("Expected Preset variant"),
        }
    }

    /// Tests that the wrong format (adjacently-tagged) is rejected.
    /// This ensures the API doesn't accept the old/wrong documentation format.
    #[test]
    fn test_embedding_model_type_rejects_wrong_format() {
        // This is the WRONG format that was in the old documentation
        let wrong_json = r#"{"preset": {"name": "fast"}}"#;
        let result: Result<EmbeddingModelType, _> = serde_json::from_str(wrong_json);

        // Should fail to parse - the wrong format should be rejected
        assert!(result.is_err(), "Should reject adjacently-tagged format");
    }

    /// Tests round-trip serialization/deserialization of EmbeddingConfig.
    #[test]
    fn test_embedding_config_roundtrip() {
        let config = EmbeddingConfig {
            model: EmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
            normalize: true,
            batch_size: 64,
            show_download_progress: false,
            cache_dir: None,
            acceleration: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EmbeddingConfig = serde_json::from_str(&json).unwrap();

        match deserialized.model {
            EmbeddingModelType::Preset { name } => {
                assert_eq!(name, "balanced");
            }
            _ => panic!("Expected Preset variant"),
        }
        assert!(deserialized.normalize);
        assert_eq!(deserialized.batch_size, 64);
    }

    /// Tests Custom model type serialization format.
    #[test]
    fn test_embedding_model_type_custom_serialization() {
        let model = EmbeddingModelType::Custom {
            model_id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            dimensions: 384,
        };
        let json = serde_json::to_string(&model).unwrap();

        assert!(json.contains(r#""type":"custom""#), "Should contain type:custom field");
        assert!(json.contains(r#""model_id":"#), "Should contain model_id field");
        assert!(json.contains(r#""dimensions":384"#), "Should contain dimensions field");
    }

    #[test]
    #[cfg(feature = "embeddings")]
    fn test_resolve_preset_balanced() {
        let config = ChunkingConfig {
            preset: Some("balanced".to_string()),
            ..Default::default()
        };
        let resolved = config.resolve_preset();
        assert_eq!(resolved.max_characters, 1024);
        assert_eq!(resolved.overlap, 100);
        assert!(resolved.embedding.is_some());
        match &resolved.embedding.unwrap().model {
            EmbeddingModelType::Preset { name } => assert_eq!(name, "balanced"),
            _ => panic!("Expected Preset model type"),
        }
    }

    #[test]
    #[cfg(feature = "embeddings")]
    fn test_resolve_preset_preserves_explicit_embedding() {
        let explicit_embedding = EmbeddingConfig {
            model: EmbeddingModelType::Custom {
                model_id: "custom/model".to_string(),
                dimensions: 512,
            },
            batch_size: 64,
            ..Default::default()
        };
        let config = ChunkingConfig {
            preset: Some("fast".to_string()),
            embedding: Some(explicit_embedding),
            ..Default::default()
        };
        let resolved = config.resolve_preset();
        assert_eq!(resolved.max_characters, 512);
        assert_eq!(resolved.overlap, 50);
        // Explicit embedding config preserved
        match &resolved.embedding.unwrap().model {
            EmbeddingModelType::Custom { model_id, .. } => assert_eq!(model_id, "custom/model"),
            _ => panic!("Expected Custom model type to be preserved"),
        }
    }

    #[test]
    fn test_resolve_preset_no_preset_returns_unchanged() {
        let config = ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            ..Default::default()
        };
        let resolved = config.resolve_preset();
        assert_eq!(resolved.max_characters, 500);
        assert_eq!(resolved.overlap, 50);
        assert!(resolved.embedding.is_none());
    }

    #[test]
    fn test_resolve_preset_unknown_name_returns_unchanged() {
        let config = ChunkingConfig {
            max_characters: 500,
            preset: Some("nonexistent".to_string()),
            ..Default::default()
        };
        let resolved = config.resolve_preset();
        assert_eq!(resolved.max_characters, 500);
    }

    #[test]
    fn test_embedding_model_type_llm_roundtrip() {
        let model_type = EmbeddingModelType::Llm {
            llm: crate::core::config::llm::LlmConfig {
                model: "openai/text-embedding-3-small".to_string(),
                api_key: None,
                base_url: None,
                timeout_secs: None,
                max_retries: None,
                temperature: None,
                max_tokens: None,
            },
        };
        let json = serde_json::to_string(&model_type).unwrap();
        assert!(json.contains("\"type\":\"llm\""));
        assert!(json.contains("openai/text-embedding-3-small"));

        let deserialized: EmbeddingModelType = serde_json::from_str(&json).unwrap();
        match deserialized {
            EmbeddingModelType::Llm { llm } => {
                assert_eq!(llm.model, "openai/text-embedding-3-small");
            }
            _ => panic!("Expected Llm variant"),
        }
    }

    #[test]
    #[should_panic(expected = "topic_threshold must be in [0.0, 1.0]")]
    fn test_with_topic_threshold_panics_above_one() {
        ChunkingConfig::default().with_topic_threshold(1.1);
    }

    #[test]
    #[should_panic(expected = "topic_threshold must be in [0.0, 1.0]")]
    fn test_with_topic_threshold_panics_below_zero() {
        ChunkingConfig::default().with_topic_threshold(-0.1);
    }

    #[test]
    fn test_with_topic_threshold_accepts_boundary_values() {
        let config = ChunkingConfig::default().with_topic_threshold(0.0);
        assert_eq!(config.topic_threshold, Some(0.0));

        let config = ChunkingConfig::default().with_topic_threshold(1.0);
        assert_eq!(config.topic_threshold, Some(1.0));
    }

    /// Tests Custom model type deserialization.
    #[test]
    fn test_embedding_model_type_custom_deserialization() {
        let json = r#"{"type": "custom", "model_id": "test/model", "dimensions": 512}"#;
        let model: EmbeddingModelType = serde_json::from_str(json).unwrap();

        match model {
            EmbeddingModelType::Custom { model_id, dimensions } => {
                assert_eq!(model_id, "test/model");
                assert_eq!(dimensions, 512);
            }
            _ => panic!("Expected Custom variant"),
        }
    }
}
