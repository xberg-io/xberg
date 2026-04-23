//! MCP request parameter types.
//!
//! This module defines the parameter structures for all MCP tool calls.

use rmcp::schemars;

/// Request parameters for file extraction.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ExtractFileParams {
    /// Path to the file to extract
    pub path: String,
    /// Optional MIME type hint (auto-detected if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Extraction configuration (JSON object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Password for encrypted PDFs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_password: Option<String>,
    /// Wire format for the response: "json" (default) or "toon"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
}

/// Request parameters for bytes extraction.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ExtractBytesParams {
    /// Base64-encoded file content
    pub data: String,
    /// Optional MIME type hint (auto-detected if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Extraction configuration (JSON object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Password for encrypted PDFs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_password: Option<String>,
    /// Wire format for the response: "json" (default) or "toon"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
}

/// Request parameters for batch file extraction.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct BatchExtractFilesParams {
    /// Paths to files to extract
    pub paths: Vec<String>,
    /// Extraction configuration (JSON object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Password for encrypted PDFs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_password: Option<String>,
    /// Per-file extraction configuration overrides (parallel array to paths).
    /// Each entry is either null (use default) or a FileExtractionConfig JSON object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_configs: Option<Vec<Option<serde_json::Value>>>,
    /// Wire format for the response: "json" (default) or "toon"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
}

/// Request parameters for MIME type detection.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DetectMimeTypeParams {
    /// Path to the file
    pub path: String,
    /// Use content-based detection (default: true)
    #[serde(default = "default_use_content")]
    pub use_content: bool,
}

fn default_use_content() -> bool {
    true
}

/// Empty parameters for tools that take no arguments.
///
/// This generates `{"type": "object", "properties": {}}` which is required by
/// the MCP specification, unlike `()` which generates `{"const": null}`.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct EmptyParams {}

/// Request parameters for cache warm (model download).
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CacheWarmParams {
    /// Download all embedding model presets
    #[serde(default)]
    pub all_embeddings: bool,
    /// Specific embedding preset name to download (e.g. "balanced", "speed", "quality")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
}

/// Request parameters for embedding generation.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct EmbedTextParams {
    /// List of text strings to generate embeddings for
    pub texts: Vec<String>,
    /// Embedding preset name (default: "balanced"). Available: "speed", "balanced", "quality"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// LLM model for provider-hosted embeddings (e.g., "openai/text-embedding-3-small").
    /// When set, overrides preset and uses liter-llm for embedding generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// API key for the LLM provider (optional, falls back to env).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Name of a pre-registered in-process embedding plugin backend.
    /// When set, overrides both preset and model and dispatches to the registered callback.
    /// Requires a prior call to `kreuzberg::plugins::register_embedding_backend`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_plugin: Option<String>,
}

/// Default schema name for structured extraction.
fn default_schema_name() -> String {
    "extraction".to_string()
}

/// Request parameters for LLM-based structured extraction.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ExtractStructuredParams {
    /// File path to extract from
    pub path: String,
    /// JSON schema for structured output
    pub schema: serde_json::Value,
    /// LLM model (e.g., "openai/gpt-4o")
    pub model: String,
    /// Schema name (default: "extraction")
    #[serde(default = "default_schema_name")]
    pub schema_name: String,
    /// Schema description for the LLM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_description: Option<String>,
    /// Custom Jinja2 prompt template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// API key (optional, falls back to env)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Enable strict mode
    #[serde(default)]
    pub strict: bool,
}

/// Request parameters for text chunking.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ChunkTextParams {
    /// Text content to split into chunks
    pub text: String,
    /// Maximum characters per chunk (default: 2000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters: Option<usize>,
    /// Number of overlapping characters between chunks (default: 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlap: Option<usize>,
    /// Chunker type: "text", "markdown", "yaml", or "semantic" (default: "text")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunker_type: Option<String>,
    /// Topic threshold for semantic chunking (0.0-1.0, default: 0.75)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_threshold: Option<f32>,
}

// These param structs are constructed by the rmcp framework via serde deserialization,
// not directly in Rust code, so clippy's dead_code lint is a false positive.
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DownloadGrammarsParams {
    /// Specific languages to download (e.g., ["python", "rust", "javascript"]).
    /// If not provided, must specify groups or all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,

    /// Language groups to download (e.g., ["web", "systems", "scripting"]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,

    /// Download all available languages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ListGrammarsParams {
    /// Only show downloaded/cached languages (default: false, shows all available).
    #[serde(default)]
    pub downloaded_only: bool,

    /// Filter languages by name substring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_params_defaults() {
        let json = r#"{"path": "/test.pdf"}"#;
        let params: ExtractFileParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.path, "/test.pdf");
        assert_eq!(params.mime_type, None);
        assert_eq!(params.config, None);
    }

    #[test]
    fn test_extract_bytes_params_defaults() {
        let json = r#"{"data": "SGVsbG8="}"#;
        let params: ExtractBytesParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.data, "SGVsbG8=");
        assert_eq!(params.mime_type, None);
        assert_eq!(params.config, None);
    }

    #[test]
    fn test_batch_extract_files_params_defaults() {
        let json = r#"{"paths": ["/a.pdf", "/b.pdf"]}"#;
        let params: BatchExtractFilesParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.paths.len(), 2);
        assert_eq!(params.config, None);
    }

    #[test]
    fn test_detect_mime_type_params_defaults() {
        let json = r#"{"path": "/test.pdf"}"#;
        let params: DetectMimeTypeParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.path, "/test.pdf");
        assert!(params.use_content);
    }

    #[test]
    fn test_detect_mime_type_params_use_content_false() {
        let json = r#"{"path": "/test.pdf", "use_content": false}"#;
        let params: DetectMimeTypeParams = serde_json::from_str(json).unwrap();

        assert!(!params.use_content);
    }

    #[test]
    fn test_extract_file_params_with_config() {
        let json = r#"{"path": "/test.pdf", "config": {"use_cache": false}}"#;
        let params: ExtractFileParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.path, "/test.pdf");
        assert!(params.config.is_some());
    }

    #[test]
    fn test_extract_file_params_serialization() {
        let params = ExtractFileParams {
            path: "/test.pdf".to_string(),
            mime_type: Some("application/pdf".to_string()),
            config: Some(serde_json::json!({"use_cache": false})),
            pdf_password: None,
            response_format: None,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: ExtractFileParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.path, deserialized.path);
        assert_eq!(params.mime_type, deserialized.mime_type);
        assert_eq!(params.config, deserialized.config);
    }

    #[test]
    fn test_extract_bytes_params_serialization() {
        let params = ExtractBytesParams {
            data: "SGVsbG8=".to_string(),
            mime_type: None,
            config: None,
            pdf_password: None,
            response_format: None,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: ExtractBytesParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.data, deserialized.data);
    }

    #[test]
    fn test_batch_extract_params_serialization() {
        let params = BatchExtractFilesParams {
            paths: vec!["/a.pdf".to_string(), "/b.pdf".to_string()],
            config: Some(serde_json::json!({"use_cache": true})),
            pdf_password: None,
            file_configs: None,
            response_format: None,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: BatchExtractFilesParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.paths, deserialized.paths);
        assert_eq!(params.config, deserialized.config);
    }

    #[test]
    fn test_detect_mime_type_params_serialization() {
        let params = DetectMimeTypeParams {
            path: "/test.pdf".to_string(),
            use_content: false,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: DetectMimeTypeParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.path, deserialized.path);
        assert_eq!(params.use_content, deserialized.use_content);
    }

    #[test]
    fn test_empty_params_schema_has_type_object() {
        let schema = schemars::schema_for!(EmptyParams);
        let json = serde_json::to_value(&schema).unwrap();
        assert_eq!(json["type"], "object");
    }

    #[test]
    fn test_empty_params_deserializes_from_empty_object() {
        let params: EmptyParams = serde_json::from_str("{}").unwrap();
        let _ = params;
    }

    #[test]
    fn test_cache_warm_params_defaults() {
        let json = r#"{}"#;
        let params: CacheWarmParams = serde_json::from_str(json).unwrap();
        assert!(!params.all_embeddings);
        assert!(params.embedding_model.is_none());
    }

    #[test]
    fn test_cache_warm_params_with_values() {
        let json = r#"{"all_embeddings": true, "embedding_model": "balanced"}"#;
        let params: CacheWarmParams = serde_json::from_str(json).unwrap();
        assert!(params.all_embeddings);
        assert_eq!(params.embedding_model.as_deref(), Some("balanced"));
    }

    #[test]
    fn test_embed_text_params_defaults() {
        let json = r#"{"texts": ["hello", "world"]}"#;
        let params: EmbedTextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.texts.len(), 2);
        assert!(params.preset.is_none());
    }

    #[test]
    fn test_embed_text_params_with_preset() {
        let json = r#"{"texts": ["hello"], "preset": "quality"}"#;
        let params: EmbedTextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.preset.as_deref(), Some("quality"));
    }

    #[test]
    fn test_chunk_text_params_defaults() {
        let json = r#"{"text": "some long text"}"#;
        let params: ChunkTextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.text, "some long text");
        assert!(params.max_characters.is_none());
        assert!(params.overlap.is_none());
        assert!(params.chunker_type.is_none());
    }

    #[test]
    fn test_chunk_text_params_with_all_fields() {
        let json = r#"{"text": "hello", "max_characters": 500, "overlap": 50, "chunker_type": "markdown"}"#;
        let params: ChunkTextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.max_characters, Some(500));
        assert_eq!(params.overlap, Some(50));
        assert_eq!(params.chunker_type.as_deref(), Some("markdown"));
    }

    #[test]
    fn test_chunk_text_params_with_topic_threshold() {
        let json = r#"{"text": "hello", "chunker_type": "semantic", "topic_threshold": 0.6}"#;
        let params: ChunkTextParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.chunker_type.as_deref(), Some("semantic"));
        assert_eq!(params.topic_threshold, Some(0.6));
    }

    #[test]
    fn test_chunk_text_params_topic_threshold_defaults_to_none() {
        let json = r#"{"text": "hello"}"#;
        let params: ChunkTextParams = serde_json::from_str(json).unwrap();
        assert!(params.topic_threshold.is_none());
    }
}
