//! Post-processing and chunking configuration.
//!
//! Defines configuration for post-processing pipelines, text chunking,
//! and embedding generation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Type of text chunker to use.
///
/// # Variants
///
/// * `Text` - Generic text splitter, splits on whitespace and punctuation
/// * `Markdown` - Markdown-aware splitter, preserves formatting and structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChunkerType {
    #[default]
    Text,
    Markdown,
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

    /// Pre-computed HashSet for O(1) enabled processor lookup
    #[serde(skip)]
    pub enabled_set: Option<HashSet<String>>,

    /// Pre-computed HashSet for O(1) disabled processor lookup
    #[serde(skip)]
    pub disabled_set: Option<HashSet<String>>,
}

impl PostProcessorConfig {
    /// Pre-compute HashSets for O(1) processor name lookups.
    ///
    /// This method converts the enabled/disabled processor Vec to HashSet
    /// for constant-time lookups in the pipeline.
    pub fn build_lookup_sets(&mut self) {
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
}

impl ChunkingConfig {
    /// Create a new `ChunkingConfig` with the given max characters, overlap, and trim settings.
    ///
    /// Other fields are set to their defaults. Use the setter methods to customize further.
    pub fn new(max_characters: usize, overlap: usize, trim: bool) -> Self {
        Self {
            max_characters,
            overlap,
            trim,
            chunker_type: ChunkerType::Text,
            embedding: None,
            preset: None,
            sizing: ChunkSizing::default(),
        }
    }

    /// Set the chunker type.
    pub fn with_chunker_type(mut self, chunker_type: ChunkerType) -> Self {
        self.chunker_type = chunker_type;
        self
    }

    /// Set the sizing strategy.
    pub fn with_sizing(mut self, sizing: ChunkSizing) -> Self {
        self.sizing = sizing;
        self
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
        }
    }
}

/// Embedding configuration for text chunks.
///
/// Configures embedding generation using ONNX models via fastembed-rs.
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
        }
    }
}

/// Embedding model types supported by Kreuzberg.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbeddingModelType {
    /// Use a preset model configuration (recommended)
    Preset { name: String },

    /// Use a specific fastembed model by name
    #[cfg(feature = "embeddings")]
    FastEmbed { model: String, dimensions: usize },

    /// Use a custom ONNX model from HuggingFace
    Custom { model_id: String, dimensions: usize },
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
