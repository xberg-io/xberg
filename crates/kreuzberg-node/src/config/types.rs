use napi::bindgen_prelude::*;
use napi_derive::napi;

use html_to_markdown_rs::options::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle,
    PreprocessingOptions as HtmlPreprocessingOptions, PreprocessingPreset, WhitespaceMode,
};
use kreuzberg::extractors::security::SecurityLimits as RustSecurityLimits;
use kreuzberg::keywords::{
    KeywordAlgorithm as RustKeywordAlgorithm, KeywordConfig as RustKeywordConfig, RakeParams as RustRakeParams,
    YakeParams as RustYakeParams,
};
use kreuzberg::pdf::HierarchyConfig as RustHierarchyConfig;
use kreuzberg::{
    AccelerationConfig as RustAccelerationConfig, ChunkerType, ChunkingConfig as RustChunkingConfig,
    ContentFilterConfig as RustContentFilterConfig, EmailConfig as RustEmailConfig,
    EmbeddingConfig as RustEmbeddingConfig, EmbeddingModelType as RustEmbeddingModelType,
    ExecutionProviderType as RustExecutionProviderType, ExtractionConfig, FileExtractionConfig,
    ImageExtractionConfig as RustImageExtractionConfig, LanguageDetectionConfig as RustLanguageDetectionConfig,
    LlmConfig as RustLlmConfig, OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig,
    PostProcessorConfig as RustPostProcessorConfig, StructuredExtractionConfig as RustStructuredExtractionConfig,
    TesseractConfig as RustTesseractConfig, TokenReductionConfig as RustTokenReductionConfig,
    TreeSitterConfig as RustTreeSitterConfig, TreeSitterProcessConfig as RustTreeSitterProcessConfig,
};
use std::ffi::c_char;

#[allow(improper_ctypes)]
unsafe extern "C" {
    /// Parse HeadingStyle from string to discriminant
    pub fn kreuzberg_parse_heading_style(value: *const c_char) -> i32;

    /// Parse CodeBlockStyle from string to discriminant
    pub fn kreuzberg_parse_code_block_style(value: *const c_char) -> i32;

    /// Parse HighlightStyle from string to discriminant
    pub fn kreuzberg_parse_highlight_style(value: *const c_char) -> i32;

    /// Parse ListIndentType from string to discriminant
    pub fn kreuzberg_parse_list_indent_type(value: *const c_char) -> i32;

}

#[napi(object)]
pub struct JsOcrConfig {
    pub enabled: Option<bool>,
    pub backend: String,
    pub language: Option<String>,
    pub tesseract_config: Option<JsTesseractConfig>,
    pub paddle_ocr_config: Option<JsPaddleOcrConfig>,
    pub element_config: Option<JsOcrElementConfig>,
    /// VLM configuration for vision-language model OCR backend.
    pub vlm_config: Option<JsLlmConfig>,
    /// Custom prompt template for VLM OCR.
    pub vlm_prompt: Option<String>,
}

#[napi(object)]
pub struct JsPaddleOcrConfig {
    pub cache_dir: Option<String>,
    pub use_angle_cls: Option<bool>,
    pub enable_table_detection: Option<bool>,
    pub det_db_thresh: Option<f64>,
    pub det_db_box_thresh: Option<f64>,
    pub det_db_unclip_ratio: Option<f64>,
    pub det_limit_side_len: Option<u32>,
    pub rec_batch_num: Option<u32>,
    pub min_confidence: Option<f64>,
    pub output_format: Option<String>,
}

#[napi(object)]
pub struct JsOcrElementConfig {
    pub include_elements: Option<bool>,
    pub min_level: Option<String>,
    pub min_confidence: Option<f64>,
    pub build_hierarchy: Option<bool>,
}

impl From<JsOcrConfig> for RustOcrConfig {
    fn from(val: JsOcrConfig) -> Self {
        RustOcrConfig {
            enabled: val.enabled.unwrap_or(true),
            backend: val.backend,
            language: val.language.unwrap_or_else(|| "eng".to_string()),
            tesseract_config: val.tesseract_config.map(Into::into),
            output_format: None,
            paddle_ocr_config: val.paddle_ocr_config.map(|p| {
                let mut map = serde_json::Map::new();
                if let Some(v) = p.cache_dir {
                    map.insert("cache_dir".into(), serde_json::Value::String(v));
                }
                if let Some(v) = p.use_angle_cls {
                    map.insert("use_angle_cls".into(), serde_json::Value::Bool(v));
                }
                if let Some(v) = p.enable_table_detection {
                    map.insert("enable_table_detection".into(), serde_json::Value::Bool(v));
                }
                if let Some(v) = p.det_db_thresh {
                    map.insert(
                        "det_db_thresh".into(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(v).unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                }
                if let Some(v) = p.det_db_box_thresh {
                    map.insert(
                        "det_db_box_thresh".into(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(v).unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                }
                if let Some(v) = p.det_db_unclip_ratio {
                    map.insert(
                        "det_db_unclip_ratio".into(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(v).unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                }
                if let Some(v) = p.det_limit_side_len {
                    map.insert("det_limit_side_len".into(), serde_json::Value::Number(v.into()));
                }
                if let Some(v) = p.rec_batch_num {
                    map.insert("rec_batch_num".into(), serde_json::Value::Number(v.into()));
                }
                if let Some(v) = p.min_confidence {
                    map.insert(
                        "min_confidence".into(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(v).unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                }
                if let Some(v) = p.output_format {
                    map.insert("output_format".into(), serde_json::Value::String(v));
                }
                serde_json::Value::Object(map)
            }),
            element_config: val.element_config.map(|ec| kreuzberg::OcrElementConfig {
                include_elements: ec.include_elements.unwrap_or(false),
                min_level: ec
                    .min_level
                    .as_deref()
                    .map(|s| match s {
                        "word" | "Word" => kreuzberg::OcrElementLevel::Word,
                        "line" | "Line" => kreuzberg::OcrElementLevel::Line,
                        "block" | "Block" => kreuzberg::OcrElementLevel::Block,
                        "page" | "Page" => kreuzberg::OcrElementLevel::Page,
                        _ => kreuzberg::OcrElementLevel::default(),
                    })
                    .unwrap_or_default(),
                min_confidence: ec.min_confidence.unwrap_or(0.0),
                build_hierarchy: ec.build_hierarchy.unwrap_or(false),
            }),
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_config: val.vlm_config.map(Into::into),
            vlm_prompt: val.vlm_prompt,
        }
    }
}

#[napi(object)]
pub struct JsTesseractConfig {
    pub psm: Option<i32>,
    pub enable_table_detection: Option<bool>,
    pub tessedit_char_whitelist: Option<String>,
}

impl From<JsTesseractConfig> for RustTesseractConfig {
    fn from(val: JsTesseractConfig) -> Self {
        let mut config = RustTesseractConfig::default();
        if let Some(psm) = val.psm {
            config.psm = psm;
        }
        if let Some(enabled) = val.enable_table_detection {
            config.enable_table_detection = enabled;
        }
        if let Some(whitelist) = val.tessedit_char_whitelist {
            config.tessedit_char_whitelist = whitelist;
        }
        config
    }
}

/// Tree-sitter processing options for Node.js bindings.
#[napi(object)]
pub struct JsTreeSitterProcessConfig {
    /// Extract structural items (functions, classes, structs, etc.). Default: true.
    pub structure: Option<bool>,
    /// Extract import statements. Default: true.
    pub imports: Option<bool>,
    /// Extract export statements. Default: true.
    pub exports: Option<bool>,
    /// Extract comments. Default: false.
    pub comments: Option<bool>,
    /// Extract docstrings. Default: false.
    pub docstrings: Option<bool>,
    /// Extract symbol definitions. Default: false.
    pub symbols: Option<bool>,
    /// Include parse diagnostics. Default: false.
    pub diagnostics: Option<bool>,
    /// Maximum chunk size in bytes. None disables chunking.
    pub chunk_max_size: Option<u32>,
    /// Content rendering mode: "chunks" (default), "raw", or "structure".
    pub content_mode: Option<String>,
}

impl From<JsTreeSitterProcessConfig> for RustTreeSitterProcessConfig {
    fn from(val: JsTreeSitterProcessConfig) -> Self {
        let mut config = RustTreeSitterProcessConfig::default();
        if let Some(v) = val.structure {
            config.structure = v;
        }
        if let Some(v) = val.imports {
            config.imports = v;
        }
        if let Some(v) = val.exports {
            config.exports = v;
        }
        if let Some(v) = val.comments {
            config.comments = v;
        }
        if let Some(v) = val.docstrings {
            config.docstrings = v;
        }
        if let Some(v) = val.symbols {
            config.symbols = v;
        }
        if let Some(v) = val.diagnostics {
            config.diagnostics = v;
        }
        if let Some(v) = val.chunk_max_size {
            config.chunk_max_size = Some(v as usize);
        }
        if let Some(ref v) = val.content_mode {
            config.content_mode = match v.as_str() {
                "raw" => kreuzberg::core::config::CodeContentMode::Raw,
                "structure" => kreuzberg::core::config::CodeContentMode::Structure,
                _ => kreuzberg::core::config::CodeContentMode::Chunks,
            };
        }
        config
    }
}

impl From<RustTreeSitterProcessConfig> for JsTreeSitterProcessConfig {
    fn from(val: RustTreeSitterProcessConfig) -> Self {
        let content_mode = match val.content_mode {
            kreuzberg::core::config::CodeContentMode::Chunks => "chunks",
            kreuzberg::core::config::CodeContentMode::Raw => "raw",
            kreuzberg::core::config::CodeContentMode::Structure => "structure",
        };
        Self {
            structure: Some(val.structure),
            imports: Some(val.imports),
            exports: Some(val.exports),
            comments: Some(val.comments),
            docstrings: Some(val.docstrings),
            symbols: Some(val.symbols),
            diagnostics: Some(val.diagnostics),
            chunk_max_size: val.chunk_max_size.map(|v| v as u32),
            content_mode: Some(content_mode.to_string()),
        }
    }
}

/// Tree-sitter language pack configuration for Node.js bindings.
#[napi(object)]
pub struct JsTreeSitterConfig {
    /// Enable code intelligence processing. Default: true.
    pub enabled: Option<bool>,
    /// Custom cache directory for downloaded grammars.
    pub cache_dir: Option<String>,
    /// Languages to pre-download on init (e.g., ["python", "rust"]).
    pub languages: Option<Vec<String>>,
    /// Language groups to pre-download (e.g., ["web", "systems", "scripting"]).
    pub groups: Option<Vec<String>>,
    /// Processing options for code analysis.
    pub process: Option<JsTreeSitterProcessConfig>,
}

impl From<JsTreeSitterConfig> for RustTreeSitterConfig {
    fn from(val: JsTreeSitterConfig) -> Self {
        RustTreeSitterConfig {
            enabled: val.enabled.unwrap_or(true),
            cache_dir: val.cache_dir.map(std::path::PathBuf::from),
            languages: val.languages,
            groups: val.groups,
            process: val.process.map(Into::into).unwrap_or_default(),
        }
    }
}

impl From<RustTreeSitterConfig> for JsTreeSitterConfig {
    fn from(val: RustTreeSitterConfig) -> Self {
        Self {
            enabled: Some(val.enabled),
            cache_dir: val.cache_dir.and_then(|p| p.to_str().map(String::from)),
            languages: val.languages,
            groups: val.groups,
            process: Some(JsTreeSitterProcessConfig::from(val.process)),
        }
    }
}

/// LLM provider/model configuration for Node.js bindings.
///
/// Used for VLM OCR, VLM embeddings, and structured extraction features.
#[napi(object)]
pub struct JsLlmConfig {
    /// Provider/model string using liter-llm routing format (e.g., "openai/gpt-4o").
    pub model: String,
    /// API key for the provider. When not set, falls back to provider env var.
    pub api_key: Option<String>,
    /// Custom base URL override for the provider endpoint.
    pub base_url: Option<String>,
    /// Request timeout in seconds.
    pub timeout_secs: Option<u32>,
    /// Maximum retry attempts.
    pub max_retries: Option<u32>,
    /// Sampling temperature for generation tasks.
    pub temperature: Option<f64>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
}

impl From<JsLlmConfig> for RustLlmConfig {
    fn from(val: JsLlmConfig) -> Self {
        RustLlmConfig {
            model: val.model,
            api_key: val.api_key,
            base_url: val.base_url,
            timeout_secs: val.timeout_secs.map(|v| v as u64),
            max_retries: val.max_retries,
            temperature: val.temperature,
            max_tokens: val.max_tokens.map(|v| v as u64),
        }
    }
}

impl From<RustLlmConfig> for JsLlmConfig {
    fn from(val: RustLlmConfig) -> Self {
        Self {
            model: val.model,
            api_key: val.api_key,
            base_url: val.base_url,
            timeout_secs: val.timeout_secs.map(|v| v as u32),
            max_retries: val.max_retries,
            temperature: val.temperature,
            max_tokens: val.max_tokens.map(|v| v as u32),
        }
    }
}

/// Structured extraction configuration for Node.js bindings.
///
/// Sends extracted document content to an LLM with a JSON schema,
/// returning structured data conforming to the schema.
#[napi(object)]
pub struct JsStructuredExtractionConfig {
    /// JSON Schema defining the desired output structure.
    #[napi(ts_type = "Record<string, unknown>")]
    pub schema: serde_json::Value,
    /// LLM configuration for the extraction.
    pub llm: JsLlmConfig,
    /// Schema name passed to the LLM's structured output mode.
    pub schema_name: Option<String>,
    /// Optional schema description for the LLM.
    pub schema_description: Option<String>,
    /// Enable strict mode — output must exactly match the schema.
    pub strict: Option<bool>,
    /// Custom Jinja2 extraction prompt template.
    pub prompt: Option<String>,
}

impl From<JsStructuredExtractionConfig> for RustStructuredExtractionConfig {
    fn from(val: JsStructuredExtractionConfig) -> Self {
        RustStructuredExtractionConfig {
            schema: val.schema,
            llm: val.llm.into(),
            schema_name: val.schema_name.unwrap_or_else(|| "extraction".to_string()),
            schema_description: val.schema_description,
            strict: val.strict.unwrap_or(false),
            prompt: val.prompt,
        }
    }
}

impl From<RustStructuredExtractionConfig> for JsStructuredExtractionConfig {
    fn from(val: RustStructuredExtractionConfig) -> Self {
        Self {
            schema: val.schema,
            llm: val.llm.into(),
            schema_name: Some(val.schema_name),
            schema_description: val.schema_description,
            strict: Some(val.strict),
            prompt: val.prompt,
        }
    }
}

/// Embedding model type configuration for Node.js bindings.
///
/// This struct represents different embedding model sources:
/// - `preset`: Use a named preset (e.g., "balanced", "fast", "quality", "multilingual")
/// - `custom`: Use a custom ONNX model from HuggingFace
#[napi(object)]
pub struct JsEmbeddingModelType {
    /// Type of model: "preset" or "custom"
    pub model_type: String,
    /// For preset: preset name; for custom: HuggingFace model ID
    pub value: String,
    /// Number of dimensions (only for custom)
    pub dimensions: Option<u32>,
}

impl From<JsEmbeddingModelType> for RustEmbeddingModelType {
    fn from(val: JsEmbeddingModelType) -> Self {
        match val.model_type.as_str() {
            "preset" => RustEmbeddingModelType::Preset { name: val.value },
            "custom" => RustEmbeddingModelType::Custom {
                model_id: val.value,
                dimensions: val.dimensions.unwrap_or(512) as usize,
            },
            "llm" => RustEmbeddingModelType::Llm {
                llm: RustLlmConfig {
                    model: val.value,
                    api_key: None,
                    base_url: None,
                    timeout_secs: None,
                    max_retries: None,
                    temperature: None,
                    max_tokens: None,
                },
            },
            _ => RustEmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
        }
    }
}

/// Embedding generation configuration for Node.js bindings.
#[napi(object)]
pub struct JsEmbeddingConfig {
    /// Embedding model configuration
    pub model: Option<JsEmbeddingModelType>,
    /// Whether to normalize embeddings (L2 normalization)
    pub normalize: Option<bool>,
    /// Batch size for embedding generation
    pub batch_size: Option<u32>,
    /// Whether to show download progress for models
    pub show_download_progress: Option<bool>,
    /// Custom cache directory for model storage
    pub cache_dir: Option<String>,
    /// Hardware acceleration configuration for ONNX Runtime inference
    pub acceleration: Option<JsAccelerationConfig>,
}

impl From<JsEmbeddingConfig> for RustEmbeddingConfig {
    fn from(val: JsEmbeddingConfig) -> Self {
        RustEmbeddingConfig {
            model: val.model.map(Into::into).unwrap_or(RustEmbeddingModelType::Preset {
                name: "balanced".to_string(),
            }),
            normalize: val.normalize.unwrap_or(true),
            batch_size: val.batch_size.unwrap_or(32) as usize,
            show_download_progress: val.show_download_progress.unwrap_or(false),
            cache_dir: val.cache_dir.map(std::path::PathBuf::from),
            acceleration: val.acceleration.map(Into::into),
        }
    }
}

#[napi(object)]
pub struct JsChunkingConfig {
    pub max_chars: Option<u32>,
    pub max_overlap: Option<u32>,
    /// Optional embedding configuration for generating embeddings
    pub embedding: Option<JsEmbeddingConfig>,
    /// Optional preset name for chunking parameters
    pub preset: Option<String>,
    /// Chunker type: "text" (default), "markdown", "yaml", or "semantic".
    /// Set to "semantic" for topic-aware chunking that works out of the box
    /// with sensible defaults. No other parameters needed.
    pub chunker_type: Option<String>,
    /// Sizing type: "characters" (default) or "tokenizer"
    pub sizing_type: Option<String>,
    /// HuggingFace model ID for tokenizer sizing (e.g., "Xenova/gpt-4o")
    pub sizing_model: Option<String>,
    /// Optional cache directory for tokenizer files
    pub sizing_cache_dir: Option<String>,
    /// Prepend heading context to each chunk when using markdown chunker
    #[napi(js_name = "prependHeadingContext")]
    pub prepend_heading_context: Option<bool>,
    /// Cosine similarity threshold for semantic topic detection (0.0-1.0).
    /// Optional, defaults to 0.75. Rarely needs tuning.
    #[napi(js_name = "topicThreshold")]
    pub topic_threshold: Option<f64>,
}

impl From<JsChunkingConfig> for RustChunkingConfig {
    fn from(val: JsChunkingConfig) -> Self {
        let ct = match val.chunker_type.as_deref() {
            Some("markdown") => ChunkerType::Markdown,
            Some("yaml") => ChunkerType::Yaml,
            Some("semantic") => ChunkerType::Semantic,
            _ => ChunkerType::Text,
        };
        let sizing = resolve_chunk_sizing(val.sizing_type, val.sizing_model, val.sizing_cache_dir);
        RustChunkingConfig {
            max_characters: val.max_chars.unwrap_or(1000) as usize,
            overlap: val.max_overlap.unwrap_or(200) as usize,
            trim: true,
            chunker_type: ct,
            embedding: val.embedding.map(Into::into),
            preset: val.preset,
            sizing,
            prepend_heading_context: val.prepend_heading_context.unwrap_or(false),
            topic_threshold: val.topic_threshold.map(|t| t as f32),
        }
    }
}

fn resolve_chunk_sizing(
    sizing_type: Option<String>,
    sizing_model: Option<String>,
    sizing_cache_dir: Option<String>,
) -> kreuzberg::ChunkSizing {
    match sizing_type.as_deref() {
        Some("tokenizer") => kreuzberg::ChunkSizing::Tokenizer {
            model: sizing_model.unwrap_or_else(|| "Xenova/gpt-4o".to_string()),
            cache_dir: sizing_cache_dir.map(std::path::PathBuf::from),
        },
        _ => kreuzberg::ChunkSizing::Characters,
    }
}

#[napi(object)]
pub struct JsLanguageDetectionConfig {
    pub enabled: Option<bool>,
    pub min_confidence: Option<f64>,
    pub detect_multiple: Option<bool>,
}

impl From<JsLanguageDetectionConfig> for RustLanguageDetectionConfig {
    fn from(val: JsLanguageDetectionConfig) -> Self {
        RustLanguageDetectionConfig {
            enabled: val.enabled.unwrap_or(true),
            min_confidence: val.min_confidence.unwrap_or(0.8),
            detect_multiple: val.detect_multiple.unwrap_or(false),
        }
    }
}

#[napi(object)]
pub struct JsTokenReductionConfig {
    pub mode: Option<String>,
    pub preserve_important_words: Option<bool>,
}

impl From<JsTokenReductionConfig> for RustTokenReductionConfig {
    fn from(val: JsTokenReductionConfig) -> Self {
        RustTokenReductionConfig {
            mode: val.mode.unwrap_or_else(|| "off".to_string()),
            preserve_important_words: val.preserve_important_words.unwrap_or(true),
        }
    }
}

#[napi(object)]
pub struct JsHierarchyConfig {
    pub enabled: Option<bool>,
    pub k_clusters: Option<i32>,
    pub include_bbox: Option<bool>,
    pub ocr_coverage_threshold: Option<f64>,
}

impl From<JsHierarchyConfig> for RustHierarchyConfig {
    fn from(val: JsHierarchyConfig) -> Self {
        RustHierarchyConfig {
            enabled: val.enabled.unwrap_or(true),
            k_clusters: val.k_clusters.map(|v| v as usize).unwrap_or(6),
            include_bbox: val.include_bbox.unwrap_or(true),
            ocr_coverage_threshold: val.ocr_coverage_threshold.map(|v| v as f32),
        }
    }
}

#[napi(object)]
pub struct JsPdfConfig {
    pub extract_images: Option<bool>,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: Option<bool>,
    pub hierarchy: Option<JsHierarchyConfig>,
    pub extract_annotations: Option<bool>,
    pub top_margin_fraction: Option<f64>,
    pub bottom_margin_fraction: Option<f64>,
    pub allow_single_column_tables: Option<bool>,
}

impl From<JsPdfConfig> for RustPdfConfig {
    fn from(val: JsPdfConfig) -> Self {
        RustPdfConfig {
            extract_images: val.extract_images.unwrap_or(false),
            passwords: val.passwords,
            extract_metadata: val.extract_metadata.unwrap_or(true),
            hierarchy: val.hierarchy.map(|h| h.into()),
            extract_annotations: val.extract_annotations.unwrap_or(false),
            top_margin_fraction: val.top_margin_fraction.map(|v| v as f32),
            bottom_margin_fraction: val.bottom_margin_fraction.map(|v| v as f32),
            allow_single_column_tables: val.allow_single_column_tables.unwrap_or(false),
            backend: kreuzberg::PdfBackend::default(),
        }
    }
}

#[napi(object)]
pub struct JsImageExtractionConfig {
    pub extract_images: Option<bool>,
    pub target_dpi: Option<i32>,
    pub max_image_dimension: Option<i32>,
    pub inject_placeholders: Option<bool>,
    pub auto_adjust_dpi: Option<bool>,
    pub min_dpi: Option<i32>,
    pub max_dpi: Option<i32>,
    pub max_images_per_page: Option<u32>,
}

impl From<JsImageExtractionConfig> for RustImageExtractionConfig {
    fn from(val: JsImageExtractionConfig) -> Self {
        RustImageExtractionConfig {
            extract_images: val.extract_images.unwrap_or(true),
            target_dpi: val.target_dpi.unwrap_or(300),
            max_image_dimension: val.max_image_dimension.unwrap_or(4096),
            inject_placeholders: val.inject_placeholders.unwrap_or(true),
            auto_adjust_dpi: val.auto_adjust_dpi.unwrap_or(true),
            min_dpi: val.min_dpi.unwrap_or(72),
            max_dpi: val.max_dpi.unwrap_or(600),
            max_images_per_page: val.max_images_per_page,
        }
    }
}

#[napi(object)]
pub struct JsPostProcessorConfig {
    pub enabled: Option<bool>,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}

impl From<JsPostProcessorConfig> for RustPostProcessorConfig {
    fn from(val: JsPostProcessorConfig) -> Self {
        let enabled_set = val
            .enabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());
        let disabled_set = val
            .disabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());

        RustPostProcessorConfig {
            enabled: val.enabled.unwrap_or(true),
            enabled_processors: val.enabled_processors,
            disabled_processors: val.disabled_processors,
            enabled_set,
            disabled_set,
        }
    }
}

#[napi(object)]
#[derive(Clone)]
pub struct JsHtmlPreprocessingOptions {
    pub enabled: Option<bool>,
    pub preset: Option<String>,
    pub remove_navigation: Option<bool>,
    pub remove_forms: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsHtmlOptions {
    pub heading_style: Option<String>,
    pub list_indent_type: Option<String>,
    pub list_indent_width: Option<u32>,
    pub bullets: Option<String>,
    pub strong_em_symbol: Option<String>,
    pub escape_asterisks: Option<bool>,
    pub escape_underscores: Option<bool>,
    pub escape_misc: Option<bool>,
    pub escape_ascii: Option<bool>,
    pub code_language: Option<String>,
    pub autolinks: Option<bool>,
    pub default_title: Option<bool>,
    pub br_in_tables: Option<bool>,
    pub highlight_style: Option<String>,
    pub extract_metadata: Option<bool>,
    pub whitespace_mode: Option<String>,
    pub strip_newlines: Option<bool>,
    pub wrap: Option<bool>,
    pub wrap_width: Option<u32>,
    pub convert_as_inline: Option<bool>,
    pub sub_symbol: Option<String>,
    pub sup_symbol: Option<String>,
    pub newline_style: Option<String>,
    pub code_block_style: Option<String>,
    pub keep_inline_images_in: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub debug: Option<bool>,
    pub strip_tags: Option<Vec<String>>,
    pub preserve_tags: Option<Vec<String>>,
    pub preprocessing: Option<JsHtmlPreprocessingOptions>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsYakeParams {
    pub window_size: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsRakeParams {
    pub min_word_length: Option<u32>,
    pub max_words_per_phrase: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsKeywordConfig {
    pub algorithm: Option<String>,
    pub max_keywords: Option<u32>,
    pub min_score: Option<f64>,
    #[napi(ts_type = "[number, number] | undefined")]
    pub ngram_range: Option<Vec<u32>>,
    pub language: Option<String>,
    pub yake_params: Option<JsYakeParams>,
    pub rake_params: Option<JsRakeParams>,
}

impl TryFrom<JsHtmlOptions> for ConversionOptions {
    type Error = Error;

    fn try_from(options: JsHtmlOptions) -> Result<Self> {
        let mut opts = ConversionOptions::default();

        if let Some(style) = options.heading_style {
            opts.heading_style = parse_heading_style(&style)?;
        }
        if let Some(indent) = options.list_indent_type {
            opts.list_indent_type = parse_list_indent_type(&indent)?;
        }
        if let Some(width) = options.list_indent_width {
            opts.list_indent_width = width as usize;
        }
        if let Some(bullets) = options.bullets {
            opts.bullets = bullets;
        }
        if let Some(symbol) = options.strong_em_symbol {
            let mut chars = symbol.chars();
            let first = chars.next().ok_or_else(|| {
                Error::new(
                    Status::InvalidArg,
                    "htmlOptions.strongEmSymbol must contain at least one character",
                )
            })?;
            opts.strong_em_symbol = first;
        }
        if let Some(value) = options.escape_asterisks {
            opts.escape_asterisks = value;
        }
        if let Some(value) = options.escape_underscores {
            opts.escape_underscores = value;
        }
        if let Some(value) = options.escape_misc {
            opts.escape_misc = value;
        }
        if let Some(value) = options.escape_ascii {
            opts.escape_ascii = value;
        }
        if let Some(language) = options.code_language {
            opts.code_language = language;
        }
        if let Some(value) = options.autolinks {
            opts.autolinks = value;
        }
        if let Some(value) = options.default_title {
            opts.default_title = value;
        }
        if let Some(value) = options.br_in_tables {
            opts.br_in_tables = value;
        }
        if let Some(style) = options.highlight_style {
            opts.highlight_style = parse_highlight_style(&style)?;
        }
        if let Some(value) = options.extract_metadata {
            opts.extract_metadata = value;
        }
        if let Some(mode) = options.whitespace_mode {
            opts.whitespace_mode = parse_whitespace_mode(&mode)?;
        }
        if let Some(value) = options.strip_newlines {
            opts.strip_newlines = value;
        }
        if let Some(value) = options.wrap {
            opts.wrap = value;
        }
        if let Some(width) = options.wrap_width {
            opts.wrap_width = width as usize;
        }
        if let Some(value) = options.convert_as_inline {
            opts.convert_as_inline = value;
        }
        if let Some(symbol) = options.sub_symbol {
            opts.sub_symbol = symbol;
        }
        if let Some(symbol) = options.sup_symbol {
            opts.sup_symbol = symbol;
        }
        if let Some(style) = options.newline_style {
            opts.newline_style = parse_newline_style(&style)?;
        }
        if let Some(style) = options.code_block_style {
            opts.code_block_style = parse_code_block_style(&style)?;
        }
        if let Some(tags) = options.keep_inline_images_in {
            opts.keep_inline_images_in = tags;
        }
        if let Some(encoding) = options.encoding {
            opts.encoding = encoding;
        }
        if let Some(debug) = options.debug {
            opts.debug = debug;
        }
        if let Some(tags) = options.strip_tags {
            opts.strip_tags = tags;
        }
        if let Some(tags) = options.preserve_tags {
            opts.preserve_tags = tags;
        }
        if let Some(pre) = options.preprocessing {
            let mut preprocessing = opts.preprocessing.clone();
            if let Some(enabled) = pre.enabled {
                preprocessing.enabled = enabled;
            }
            if let Some(preset) = pre.preset {
                preprocessing.preset = parse_preprocessing_preset(&preset)?;
            }
            if let Some(remove_navigation) = pre.remove_navigation {
                preprocessing.remove_navigation = remove_navigation;
            }
            if let Some(remove_forms) = pre.remove_forms {
                preprocessing.remove_forms = remove_forms;
            }
            opts.preprocessing = preprocessing;
        }

        Ok(opts)
    }
}

impl From<&HtmlPreprocessingOptions> for JsHtmlPreprocessingOptions {
    fn from(opts: &HtmlPreprocessingOptions) -> Self {
        Self {
            enabled: Some(opts.enabled),
            preset: Some(preprocessing_preset_to_string(opts.preset).to_string()),
            remove_navigation: Some(opts.remove_navigation),
            remove_forms: Some(opts.remove_forms),
        }
    }
}

impl From<&ConversionOptions> for JsHtmlOptions {
    fn from(opts: &ConversionOptions) -> Self {
        Self {
            heading_style: Some(heading_style_to_string(opts.heading_style).to_string()),
            list_indent_type: Some(list_indent_type_to_string(opts.list_indent_type).to_string()),
            list_indent_width: Some(opts.list_indent_width as u32),
            bullets: Some(opts.bullets.clone()),
            strong_em_symbol: Some(opts.strong_em_symbol.to_string()),
            escape_asterisks: Some(opts.escape_asterisks),
            escape_underscores: Some(opts.escape_underscores),
            escape_misc: Some(opts.escape_misc),
            escape_ascii: Some(opts.escape_ascii),
            code_language: Some(opts.code_language.clone()),
            autolinks: Some(opts.autolinks),
            default_title: Some(opts.default_title),
            br_in_tables: Some(opts.br_in_tables),
            highlight_style: Some(highlight_style_to_string(opts.highlight_style).to_string()),
            extract_metadata: Some(opts.extract_metadata),
            whitespace_mode: Some(whitespace_mode_to_string(opts.whitespace_mode).to_string()),
            strip_newlines: Some(opts.strip_newlines),
            wrap: Some(opts.wrap),
            wrap_width: Some(opts.wrap_width as u32),
            convert_as_inline: Some(opts.convert_as_inline),
            sub_symbol: Some(opts.sub_symbol.clone()),
            sup_symbol: Some(opts.sup_symbol.clone()),
            newline_style: Some(newline_style_to_string(opts.newline_style).to_string()),
            code_block_style: Some(code_block_style_to_string(opts.code_block_style).to_string()),
            keep_inline_images_in: Some(opts.keep_inline_images_in.clone()),
            encoding: Some(opts.encoding.clone()),
            debug: Some(opts.debug),
            strip_tags: Some(opts.strip_tags.clone()),
            preserve_tags: Some(opts.preserve_tags.clone()),
            preprocessing: Some(JsHtmlPreprocessingOptions::from(&opts.preprocessing)),
        }
    }
}

impl TryFrom<JsKeywordConfig> for RustKeywordConfig {
    type Error = Error;

    fn try_from(config: JsKeywordConfig) -> Result<Self> {
        let mut keywords = RustKeywordConfig::default();

        if let Some(max) = config.max_keywords {
            keywords.max_keywords = max as usize;
        }
        if let Some(score) = config.min_score {
            keywords.min_score = score as f32;
        }
        if let Some(range) = config.ngram_range {
            if range.len() != 2 {
                return Err(Error::new(
                    Status::InvalidArg,
                    "keywords.ngramRange must contain exactly two elements",
                ));
            }
            keywords.ngram_range = (range[0] as usize, range[1] as usize);
        }
        if let Some(language) = config.language {
            keywords.language = Some(language);
        }
        if let Some(algorithm) = config.algorithm {
            keywords.algorithm = parse_keyword_algorithm(&algorithm)?;
        }
        if let Some(yake) = config.yake_params {
            let mut params = RustYakeParams::default();
            if let Some(window) = yake.window_size {
                params.window_size = window as usize;
            }
            keywords.yake_params = Some(params);
        }
        if let Some(rake) = config.rake_params {
            let mut params = RustRakeParams::default();
            if let Some(min_len) = rake.min_word_length {
                params.min_word_length = min_len as usize;
            }
            if let Some(max_words) = rake.max_words_per_phrase {
                params.max_words_per_phrase = max_words as usize;
            }
            keywords.rake_params = Some(params);
        }

        Ok(keywords)
    }
}

impl From<RustKeywordConfig> for JsKeywordConfig {
    fn from(config: RustKeywordConfig) -> Self {
        Self {
            algorithm: Some(keyword_algorithm_to_string(config.algorithm).to_string()),
            max_keywords: Some(config.max_keywords as u32),
            min_score: Some(config.min_score as f64),
            ngram_range: Some(vec![config.ngram_range.0 as u32, config.ngram_range.1 as u32]),
            language: config.language,
            yake_params: config.yake_params.map(|params| JsYakeParams {
                window_size: Some(params.window_size as u32),
            }),
            rake_params: config.rake_params.map(|params| JsRakeParams {
                min_word_length: Some(params.min_word_length as u32),
                max_words_per_phrase: Some(params.max_words_per_phrase as u32),
            }),
        }
    }
}

fn parse_heading_style(value: &str) -> Result<HeadingStyle> {
    let c_str = std::ffi::CString::new(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.headingStyle '{}'", value),
        )
    })?;

    let _discriminant = unsafe { kreuzberg_parse_heading_style(c_str.as_ptr()) };

    match _discriminant {
        0 => Ok(HeadingStyle::Atx),
        1 => Ok(HeadingStyle::Underlined),
        2 => Ok(HeadingStyle::AtxClosed),
        _ => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.headingStyle '{}'", value),
        )),
    }
}

fn heading_style_to_string(style: HeadingStyle) -> &'static str {
    match style {
        HeadingStyle::Atx => "atx",
        HeadingStyle::Underlined => "underlined",
        HeadingStyle::AtxClosed => "atx_closed",
    }
}

fn parse_list_indent_type(value: &str) -> Result<ListIndentType> {
    let c_str = std::ffi::CString::new(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.listIndentType '{}'", value),
        )
    })?;

    let discriminant = unsafe { kreuzberg_parse_list_indent_type(c_str.as_ptr()) };

    match discriminant {
        0 => Ok(ListIndentType::Spaces),
        1 => Ok(ListIndentType::Tabs),
        _ => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.listIndentType '{}'", value),
        )),
    }
}

fn list_indent_type_to_string(value: ListIndentType) -> &'static str {
    match value {
        ListIndentType::Spaces => "spaces",
        ListIndentType::Tabs => "tabs",
    }
}

fn parse_highlight_style(value: &str) -> Result<HighlightStyle> {
    let c_str = std::ffi::CString::new(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.highlightStyle '{}'", value),
        )
    })?;

    let discriminant = unsafe { kreuzberg_parse_highlight_style(c_str.as_ptr()) };

    match discriminant {
        0 => Ok(HighlightStyle::DoubleEqual),
        1 => Ok(HighlightStyle::Html),
        2 => Ok(HighlightStyle::Bold),
        3 => Ok(HighlightStyle::None),
        _ => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.highlightStyle '{}'", value),
        )),
    }
}

fn highlight_style_to_string(style: HighlightStyle) -> &'static str {
    match style {
        HighlightStyle::DoubleEqual => "double_equal",
        HighlightStyle::Html => "html",
        HighlightStyle::Bold => "bold",
        HighlightStyle::None => "none",
    }
}

fn parse_whitespace_mode(value: &str) -> Result<WhitespaceMode> {
    match value.to_lowercase().as_str() {
        "normalized" => Ok(WhitespaceMode::Normalized),
        "strict" => Ok(WhitespaceMode::Strict),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.whitespaceMode '{}'", other),
        )),
    }
}

fn whitespace_mode_to_string(mode: WhitespaceMode) -> &'static str {
    match mode {
        WhitespaceMode::Normalized => "normalized",
        WhitespaceMode::Strict => "strict",
    }
}

fn parse_newline_style(value: &str) -> Result<NewlineStyle> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(NewlineStyle::Spaces),
        "backslash" => Ok(NewlineStyle::Backslash),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.newlineStyle '{}'", other),
        )),
    }
}

fn newline_style_to_string(value: NewlineStyle) -> &'static str {
    match value {
        NewlineStyle::Spaces => "spaces",
        NewlineStyle::Backslash => "backslash",
    }
}

fn parse_code_block_style(value: &str) -> Result<CodeBlockStyle> {
    let c_str = std::ffi::CString::new(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.codeBlockStyle '{}'", value),
        )
    })?;

    let discriminant = unsafe { kreuzberg_parse_code_block_style(c_str.as_ptr()) };

    match discriminant {
        0 => Ok(CodeBlockStyle::Indented),
        1 => Ok(CodeBlockStyle::Backticks),
        2 => Ok(CodeBlockStyle::Tildes),
        _ => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.codeBlockStyle '{}'", value),
        )),
    }
}

fn code_block_style_to_string(value: CodeBlockStyle) -> &'static str {
    match value {
        CodeBlockStyle::Indented => "indented",
        CodeBlockStyle::Backticks => "backticks",
        CodeBlockStyle::Tildes => "tildes",
    }
}

fn parse_preprocessing_preset(value: &str) -> Result<PreprocessingPreset> {
    match value.to_lowercase().as_str() {
        "minimal" => Ok(PreprocessingPreset::Minimal),
        "standard" => Ok(PreprocessingPreset::Standard),
        "aggressive" => Ok(PreprocessingPreset::Aggressive),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.preprocessing.preset '{}'", other),
        )),
    }
}

fn preprocessing_preset_to_string(preset: PreprocessingPreset) -> &'static str {
    match preset {
        PreprocessingPreset::Minimal => "minimal",
        PreprocessingPreset::Standard => "standard",
        PreprocessingPreset::Aggressive => "aggressive",
    }
}

fn parse_keyword_algorithm(value: &str) -> Result<RustKeywordAlgorithm> {
    match value.to_lowercase().as_str() {
        "yake" => Ok(RustKeywordAlgorithm::Yake),
        "rake" => Ok(RustKeywordAlgorithm::Rake),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid keywords.algorithm '{}'. Expected 'yake' or 'rake'", other),
        )),
    }
}

fn keyword_algorithm_to_string(algo: RustKeywordAlgorithm) -> &'static str {
    match algo {
        RustKeywordAlgorithm::Yake => "yake",
        RustKeywordAlgorithm::Rake => "rake",
    }
}

/// Concurrency configuration for Node.js bindings.
#[napi(object)]
pub struct JsConcurrencyConfig {
    /// Maximum number of threads for all internal thread pools.
    pub max_threads: Option<u32>,
}

impl From<JsConcurrencyConfig> for kreuzberg::core::config::ConcurrencyConfig {
    fn from(val: JsConcurrencyConfig) -> Self {
        kreuzberg::core::config::ConcurrencyConfig {
            max_threads: val.max_threads.map(|v| v as usize),
        }
    }
}

/// Email extraction configuration for Node.js bindings.
#[napi(object)]
pub struct JsEmailConfig {
    /// Windows codepage number for MSG files with no codepage property.
    /// Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western, default),
    /// 1253 (Greek), 1254 (Turkish), 932 (Japanese), 936 (Simplified Chinese).
    pub msg_fallback_codepage: Option<u32>,
}

impl From<JsEmailConfig> for RustEmailConfig {
    fn from(val: JsEmailConfig) -> Self {
        RustEmailConfig {
            msg_fallback_codepage: val.msg_fallback_codepage,
        }
    }
}

/// Content filtering configuration for Node.js bindings.
///
/// Controls whether "furniture" content (headers, footers, watermarks,
/// repeating text) is included in or stripped from extraction results.
#[napi(object)]
pub struct JsContentFilterConfig {
    /// Include running headers in extraction output. Default: false.
    pub include_headers: Option<bool>,
    /// Include running footers in extraction output. Default: false.
    pub include_footers: Option<bool>,
    /// Enable cross-page repeating text detection and removal. Default: true.
    pub strip_repeating_text: Option<bool>,
    /// Include watermark text in extraction output. Default: false.
    pub include_watermarks: Option<bool>,
}

impl From<JsContentFilterConfig> for RustContentFilterConfig {
    fn from(val: JsContentFilterConfig) -> Self {
        RustContentFilterConfig {
            include_headers: val.include_headers.unwrap_or(false),
            include_footers: val.include_footers.unwrap_or(false),
            strip_repeating_text: val.strip_repeating_text.unwrap_or(true),
            include_watermarks: val.include_watermarks.unwrap_or(false),
        }
    }
}

impl From<RustContentFilterConfig> for JsContentFilterConfig {
    fn from(val: RustContentFilterConfig) -> Self {
        Self {
            include_headers: Some(val.include_headers),
            include_footers: Some(val.include_footers),
            strip_repeating_text: Some(val.strip_repeating_text),
            include_watermarks: Some(val.include_watermarks),
        }
    }
}

/// HTML output configuration for styled HTML rendering.
///
/// Controls how `outputFormat: "html"` renders documents when `htmlOutput`
/// is set on the extraction config.
#[napi(object)]
pub struct JsHtmlOutputConfig {
    /// Inline CSS string injected after the theme stylesheet.
    pub css: Option<String>,
    /// Path to a CSS file loaded at renderer construction time.
    pub css_file: Option<String>,
    /// Built-in theme: "default", "github", "dark", "light", "unstyled". Default: "unstyled".
    pub theme: Option<String>,
    /// CSS class prefix for emitted class names. Default: "kb-".
    pub class_prefix: Option<String>,
    /// Embed resolved CSS in a `<style>` block. Default: true.
    pub embed_css: Option<bool>,
}

impl From<JsHtmlOutputConfig> for kreuzberg::HtmlOutputConfig {
    fn from(val: JsHtmlOutputConfig) -> Self {
        let theme = match val.theme.as_deref().unwrap_or("unstyled") {
            "default" => kreuzberg::HtmlTheme::Default,
            "github" => kreuzberg::HtmlTheme::GitHub,
            "dark" => kreuzberg::HtmlTheme::Dark,
            "light" => kreuzberg::HtmlTheme::Light,
            _ => kreuzberg::HtmlTheme::Unstyled,
        };

        kreuzberg::HtmlOutputConfig {
            css: val.css,
            css_file: val.css_file.map(std::path::PathBuf::from),
            theme,
            class_prefix: val.class_prefix.unwrap_or_else(|| "kb-".to_string()),
            embed_css: val.embed_css.unwrap_or(true),
        }
    }
}

impl From<kreuzberg::HtmlOutputConfig> for JsHtmlOutputConfig {
    fn from(val: kreuzberg::HtmlOutputConfig) -> Self {
        Self {
            css: val.css,
            css_file: val.css_file.map(|p| p.to_string_lossy().into_owned()),
            theme: Some(match val.theme {
                kreuzberg::HtmlTheme::Default => "default".to_string(),
                kreuzberg::HtmlTheme::GitHub => "github".to_string(),
                kreuzberg::HtmlTheme::Dark => "dark".to_string(),
                kreuzberg::HtmlTheme::Light => "light".to_string(),
                kreuzberg::HtmlTheme::Unstyled => "unstyled".to_string(),
            }),
            class_prefix: Some(val.class_prefix),
            embed_css: Some(val.embed_css),
        }
    }
}

/// Hardware acceleration configuration for ONNX Runtime inference.
///
/// Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
/// for layout detection and embedding generation.
#[napi(object)]
pub struct JsAccelerationConfig {
    /// Execution provider: "auto" (default), "cpu", "coreml", "cuda", "tensorrt".
    pub provider: Option<String>,
    /// GPU device ID for CUDA/TensorRT. Ignored for CPU/CoreML/Auto.
    pub device_id: Option<u32>,
}

impl From<JsAccelerationConfig> for RustAccelerationConfig {
    fn from(val: JsAccelerationConfig) -> Self {
        let provider = match val.provider.as_deref() {
            Some("cpu") => RustExecutionProviderType::Cpu,
            Some("coreml") => RustExecutionProviderType::CoreMl,
            Some("cuda") => RustExecutionProviderType::Cuda,
            Some("tensorrt") | Some("tensor_rt") => RustExecutionProviderType::TensorRt,
            // "auto" or anything unrecognized
            _ => RustExecutionProviderType::Auto,
        };
        RustAccelerationConfig {
            provider,
            device_id: val.device_id.unwrap_or(0),
        }
    }
}

/// Security limits to protect against DoS attacks (ZIP bombs, XML entity expansion, etc.).
#[napi(object)]
pub struct JsSecurityLimits {
    /// Maximum uncompressed size for archives in bytes.
    pub max_archive_size: Option<u32>,
    /// Maximum compression ratio before flagging as potential bomb.
    pub max_compression_ratio: Option<u32>,
    /// Maximum number of files in an archive.
    pub max_files_in_archive: Option<u32>,
    /// Maximum nesting depth for structures.
    pub max_nesting_depth: Option<u32>,
    /// Maximum entity/string length.
    pub max_entity_length: Option<u32>,
    /// Maximum content size in bytes.
    pub max_content_size: Option<u32>,
    /// Maximum iterations per operation.
    pub max_iterations: Option<u32>,
    /// Maximum XML depth in levels.
    pub max_xml_depth: Option<u32>,
    /// Maximum cells per table.
    pub max_table_cells: Option<u32>,
}

impl From<JsSecurityLimits> for RustSecurityLimits {
    fn from(val: JsSecurityLimits) -> Self {
        let defaults = RustSecurityLimits::default();
        RustSecurityLimits {
            max_archive_size: val
                .max_archive_size
                .map(|v| v as usize)
                .unwrap_or(defaults.max_archive_size),
            max_compression_ratio: val
                .max_compression_ratio
                .map(|v| v as usize)
                .unwrap_or(defaults.max_compression_ratio),
            max_files_in_archive: val
                .max_files_in_archive
                .map(|v| v as usize)
                .unwrap_or(defaults.max_files_in_archive),
            max_nesting_depth: val
                .max_nesting_depth
                .map(|v| v as usize)
                .unwrap_or(defaults.max_nesting_depth),
            max_entity_length: val
                .max_entity_length
                .map(|v| v as usize)
                .unwrap_or(defaults.max_entity_length),
            max_content_size: val
                .max_content_size
                .map(|v| v as usize)
                .unwrap_or(defaults.max_content_size),
            max_iterations: val
                .max_iterations
                .map(|v| v as usize)
                .unwrap_or(defaults.max_iterations),
            max_xml_depth: val.max_xml_depth.map(|v| v as usize).unwrap_or(defaults.max_xml_depth),
            max_table_cells: val
                .max_table_cells
                .map(|v| v as usize)
                .unwrap_or(defaults.max_table_cells),
        }
    }
}

#[napi(object)]
pub struct JsPageConfig {
    pub extract_pages: Option<bool>,
    pub insert_page_markers: Option<bool>,
    pub marker_format: Option<String>,
}

fn parse_table_model(s: &str) -> kreuzberg::core::config::layout::TableModel {
    match s {
        "tatr" => kreuzberg::core::config::layout::TableModel::Tatr,
        "slanet_wired" => kreuzberg::core::config::layout::TableModel::SlanetWired,
        "slanet_wireless" => kreuzberg::core::config::layout::TableModel::SlanetWireless,
        "slanet_plus" => kreuzberg::core::config::layout::TableModel::SlanetPlus,
        "slanet_auto" => kreuzberg::core::config::layout::TableModel::SlanetAuto,
        "disabled" => kreuzberg::core::config::layout::TableModel::Disabled,
        _ => kreuzberg::core::config::layout::TableModel::default(),
    }
}

#[napi(object)]
pub struct JsLayoutDetectionConfig {
    pub confidence_threshold: Option<f64>,
    pub apply_heuristics: Option<bool>,
    pub table_model: Option<String>,
    /// Hardware acceleration configuration for ONNX Runtime inference
    pub acceleration: Option<JsAccelerationConfig>,
}

impl From<JsLayoutDetectionConfig> for kreuzberg::core::config::layout::LayoutDetectionConfig {
    fn from(val: JsLayoutDetectionConfig) -> Self {
        kreuzberg::core::config::layout::LayoutDetectionConfig {
            confidence_threshold: val.confidence_threshold.map(|v| v as f32),
            apply_heuristics: val.apply_heuristics.unwrap_or(true),
            table_model: val.table_model.as_deref().map(parse_table_model).unwrap_or_default(),
            acceleration: val.acceleration.map(Into::into),
        }
    }
}

impl From<kreuzberg::core::config::layout::LayoutDetectionConfig> for JsLayoutDetectionConfig {
    fn from(config: kreuzberg::core::config::layout::LayoutDetectionConfig) -> Self {
        Self {
            confidence_threshold: config.confidence_threshold.map(|v| v as f64),
            apply_heuristics: Some(config.apply_heuristics),
            table_model: Some(config.table_model.to_string()),
            acceleration: config.acceleration.map(|a| JsAccelerationConfig {
                provider: Some(match a.provider {
                    RustExecutionProviderType::Auto => "auto".to_string(),
                    RustExecutionProviderType::Cpu => "cpu".to_string(),
                    RustExecutionProviderType::CoreMl => "coreml".to_string(),
                    RustExecutionProviderType::Cuda => "cuda".to_string(),
                    RustExecutionProviderType::TensorRt => "tensorrt".to_string(),
                }),
                device_id: Some(a.device_id),
            }),
        }
    }
}

#[napi(object)]
pub struct JsExtractionConfig {
    pub use_cache: Option<bool>,
    pub enable_quality_processing: Option<bool>,
    pub ocr: Option<JsOcrConfig>,
    pub force_ocr: Option<bool>,
    /// Disable OCR entirely — image files return empty content instead of errors
    pub disable_ocr: Option<bool>,
    /// List of 1-indexed page numbers to force OCR on (None = use force_ocr setting)
    pub force_ocr_pages: Option<Vec<u32>>,
    pub chunking: Option<JsChunkingConfig>,
    pub images: Option<JsImageExtractionConfig>,
    pub pdf_options: Option<JsPdfConfig>,
    pub token_reduction: Option<JsTokenReductionConfig>,
    pub language_detection: Option<JsLanguageDetectionConfig>,
    pub postprocessor: Option<JsPostProcessorConfig>,
    pub keywords: Option<JsKeywordConfig>,
    pub html_options: Option<JsHtmlOptions>,
    pub max_concurrent_extractions: Option<u32>,
    pub pages: Option<JsPageConfig>,
    /// Output text format: "plain" | "markdown" | "djot" | "html"
    pub output_format: Option<String>,
    /// Result structure format: "unified" | "element_based"
    pub result_format: Option<String>,
    /// Include document structure in extraction result
    pub include_document_structure: Option<bool>,
    /// Layout detection configuration (None = layout detection disabled)
    pub layout: Option<JsLayoutDetectionConfig>,
    /// Email extraction configuration
    pub email: Option<JsEmailConfig>,
    /// Hardware acceleration configuration for ONNX Runtime inference
    pub acceleration: Option<JsAccelerationConfig>,
    /// Security limits to guard against DoS attacks
    pub security_limits: Option<JsSecurityLimits>,
    /// Concurrency configuration for thread pool control
    pub concurrency: Option<JsConcurrencyConfig>,
    /// Cache namespace for tenant isolation
    pub cache_namespace: Option<String>,
    /// Per-request cache TTL in seconds (0 = skip cache)
    pub cache_ttl_secs: Option<u32>,
    /// Maximum recursion depth for archive extraction (default: 3)
    pub max_archive_depth: Option<u32>,
    /// Default per-file extraction timeout in seconds
    pub extraction_timeout_secs: Option<u32>,
    /// Tree-sitter language pack configuration for code analysis
    pub tree_sitter: Option<JsTreeSitterConfig>,
    /// Structured extraction configuration for LLM-based data extraction
    pub structured_extraction: Option<JsStructuredExtractionConfig>,
    /// Content filtering configuration for headers/footers/watermarks
    pub content_filter: Option<JsContentFilterConfig>,
    /// HTML output configuration for styled HTML rendering
    pub html_output: Option<JsHtmlOutputConfig>,
}

impl TryFrom<JsPageConfig> for kreuzberg::core::config::PageConfig {
    type Error = Error;

    fn try_from(val: JsPageConfig) -> Result<Self> {
        Ok(kreuzberg::core::config::PageConfig {
            extract_pages: val.extract_pages.unwrap_or(false),
            insert_page_markers: val.insert_page_markers.unwrap_or(false),
            marker_format: val
                .marker_format
                .unwrap_or_else(|| "\n\n<!-- PAGE {page_num} -->\n\n".to_string()),
        })
    }
}

impl From<kreuzberg::core::config::PageConfig> for JsPageConfig {
    fn from(config: kreuzberg::core::config::PageConfig) -> Self {
        Self {
            extract_pages: Some(config.extract_pages),
            insert_page_markers: Some(config.insert_page_markers),
            marker_format: Some(config.marker_format),
        }
    }
}

impl TryFrom<JsExtractionConfig> for ExtractionConfig {
    type Error = Error;

    fn try_from(val: JsExtractionConfig) -> Result<Self> {
        let html_options = match val.html_options {
            Some(options) => Some(ConversionOptions::try_from(options)?),
            None => None,
        };

        let keywords = match val.keywords {
            Some(config) => Some(RustKeywordConfig::try_from(config)?),
            None => None,
        };

        Ok(ExtractionConfig {
            use_cache: val.use_cache.unwrap_or(true),
            enable_quality_processing: val.enable_quality_processing.unwrap_or(true),
            ocr: val.ocr.map(Into::into),
            force_ocr: val.force_ocr.unwrap_or(false),
            disable_ocr: val.disable_ocr.unwrap_or(false),
            force_ocr_pages: val.force_ocr_pages.map(|v| v.into_iter().map(|p| p as usize).collect()),
            chunking: val.chunking.map(Into::into),
            content_filter: val.content_filter.map(Into::into),
            images: val.images.map(Into::into),
            pdf_options: val.pdf_options.map(Into::into),
            token_reduction: val.token_reduction.map(Into::into),
            language_detection: val.language_detection.map(Into::into),
            keywords,
            postprocessor: val.postprocessor.map(Into::into),
            html_options,
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as usize),
            pages: val.pages.map(|p| p.try_into()).transpose()?,
            output_format: val
                .output_format
                .map(|s| s.parse())
                .transpose()
                .map_err(|e: String| Error::new(Status::InvalidArg, e))?
                .unwrap_or_default(),
            result_format: val
                .result_format
                .map(|s| match s.as_str() {
                    "unified" => Ok(kreuzberg::types::OutputFormat::Unified),
                    "element_based" => Ok(kreuzberg::types::OutputFormat::ElementBased),
                    other => Err(Error::new(
                        Status::InvalidArg,
                        format!(
                            "Invalid result_format: {}. Expected 'unified' or 'element_based'",
                            other
                        ),
                    )),
                })
                .transpose()?
                .unwrap_or_default(),
            include_document_structure: val.include_document_structure.unwrap_or(false),
            security_limits: val.security_limits.map(Into::into),
            layout: val.layout.map(Into::into),
            acceleration: val.acceleration.map(Into::into),
            email: val.email.map(Into::into),
            concurrency: val.concurrency.map(Into::into),
            extraction_timeout_secs: val.extraction_timeout_secs.map(|v| v as u64),
            cache_namespace: val.cache_namespace,
            cache_ttl_secs: val.cache_ttl_secs.map(|v| v as u64),
            max_archive_depth: val.max_archive_depth.map(|v| v as usize).unwrap_or(3),
            tree_sitter: val.tree_sitter.map(Into::into),
            structured_extraction: val.structured_extraction.map(Into::into),
            html_output: val.html_output.map(Into::into),
            cancel_token: None,
        })
    }
}

impl TryFrom<ExtractionConfig> for JsExtractionConfig {
    type Error = napi::Error;

    fn try_from(val: ExtractionConfig) -> Result<Self> {
        Ok(JsExtractionConfig {
            use_cache: Some(val.use_cache),
            enable_quality_processing: Some(val.enable_quality_processing),
            ocr: val.ocr.map(|ocr| JsOcrConfig {
                enabled: Some(ocr.enabled),
                backend: ocr.backend,
                language: Some(ocr.language),
                tesseract_config: ocr.tesseract_config.map(|tc| JsTesseractConfig {
                    psm: Some(tc.psm),
                    enable_table_detection: Some(tc.enable_table_detection),
                    tessedit_char_whitelist: if tc.tessedit_char_whitelist.is_empty() {
                        None
                    } else {
                        Some(tc.tessedit_char_whitelist)
                    },
                }),
                paddle_ocr_config: ocr
                    .paddle_ocr_config
                    .and_then(|v| serde_json::from_value::<kreuzberg::PaddleOcrConfig>(v).ok())
                    .map(|p| JsPaddleOcrConfig {
                        cache_dir: p.cache_dir.map(|p| p.to_string_lossy().into_owned()),
                        use_angle_cls: Some(p.use_angle_cls),
                        enable_table_detection: Some(p.enable_table_detection),
                        det_db_thresh: Some(p.det_db_thresh as f64),
                        det_db_box_thresh: Some(p.det_db_box_thresh as f64),
                        det_db_unclip_ratio: Some(p.det_db_unclip_ratio as f64),
                        det_limit_side_len: Some(p.det_limit_side_len),
                        rec_batch_num: Some(p.rec_batch_num),
                        min_confidence: None,
                        output_format: None,
                    }),
                element_config: ocr.element_config.map(|ec| JsOcrElementConfig {
                    include_elements: Some(ec.include_elements),
                    min_level: Some(match ec.min_level {
                        kreuzberg::OcrElementLevel::Word => "word".to_string(),
                        kreuzberg::OcrElementLevel::Line => "line".to_string(),
                        kreuzberg::OcrElementLevel::Block => "block".to_string(),
                        kreuzberg::OcrElementLevel::Page => "page".to_string(),
                    }),
                    min_confidence: Some(ec.min_confidence),
                    build_hierarchy: Some(ec.build_hierarchy),
                }),
                vlm_config: ocr.vlm_config.map(JsLlmConfig::from),
                vlm_prompt: ocr.vlm_prompt,
            }),
            force_ocr: Some(val.force_ocr),
            disable_ocr: Some(val.disable_ocr),
            force_ocr_pages: val.force_ocr_pages.map(|v| v.into_iter().map(|p| p as u32).collect()),
            chunking: val.chunking.map(|chunk| JsChunkingConfig {
                max_chars: Some(chunk.max_characters as u32),
                max_overlap: Some(chunk.overlap as u32),
                embedding: chunk.embedding.map(|emb| JsEmbeddingConfig {
                    model: Some(JsEmbeddingModelType {
                        model_type: match emb.model {
                            RustEmbeddingModelType::Preset { .. } => "preset".to_string(),
                            RustEmbeddingModelType::Custom { .. } => "custom".to_string(),
                            RustEmbeddingModelType::Llm { .. } => "llm".to_string(),
                        },
                        value: match &emb.model {
                            RustEmbeddingModelType::Preset { name } => name.clone(),
                            RustEmbeddingModelType::Custom { model_id, .. } => model_id.clone(),
                            RustEmbeddingModelType::Llm { llm } => llm.model.clone(),
                        },
                        dimensions: match emb.model {
                            RustEmbeddingModelType::Custom { dimensions, .. } => Some(dimensions as u32),
                            _ => None,
                        },
                    }),
                    normalize: Some(emb.normalize),
                    batch_size: Some(emb.batch_size as u32),
                    show_download_progress: Some(emb.show_download_progress),
                    cache_dir: emb.cache_dir.and_then(|p| p.to_str().map(String::from)),
                    acceleration: emb.acceleration.map(|a| JsAccelerationConfig {
                        provider: Some(match a.provider {
                            RustExecutionProviderType::Auto => "auto".to_string(),
                            RustExecutionProviderType::Cpu => "cpu".to_string(),
                            RustExecutionProviderType::CoreMl => "coreml".to_string(),
                            RustExecutionProviderType::Cuda => "cuda".to_string(),
                            RustExecutionProviderType::TensorRt => "tensorrt".to_string(),
                        }),
                        device_id: Some(a.device_id),
                    }),
                }),
                preset: chunk.preset,
                chunker_type: match chunk.chunker_type {
                    ChunkerType::Text => None,
                    ChunkerType::Markdown => Some("markdown".to_string()),
                    ChunkerType::Yaml => Some("yaml".to_string()),
                    ChunkerType::Semantic => Some("semantic".to_string()),
                },
                sizing_type: match &chunk.sizing {
                    kreuzberg::ChunkSizing::Characters => None,
                    kreuzberg::ChunkSizing::Tokenizer { .. } => Some("tokenizer".to_string()),
                },
                sizing_model: match &chunk.sizing {
                    kreuzberg::ChunkSizing::Tokenizer { model, .. } => Some(model.clone()),
                    _ => None,
                },
                sizing_cache_dir: match &chunk.sizing {
                    kreuzberg::ChunkSizing::Tokenizer { cache_dir, .. } => {
                        cache_dir.as_ref().and_then(|p| p.to_str().map(String::from))
                    }
                    _ => None,
                },
                prepend_heading_context: Some(chunk.prepend_heading_context),
                topic_threshold: chunk.topic_threshold.map(|t| t as f64),
            }),
            images: val.images.map(|img| JsImageExtractionConfig {
                extract_images: Some(img.extract_images),
                target_dpi: Some(img.target_dpi),
                max_image_dimension: Some(img.max_image_dimension),
                inject_placeholders: Some(img.inject_placeholders),
                auto_adjust_dpi: Some(img.auto_adjust_dpi),
                min_dpi: Some(img.min_dpi),
                max_dpi: Some(img.max_dpi),
                max_images_per_page: img.max_images_per_page,
            }),
            pdf_options: val.pdf_options.map(|pdf| JsPdfConfig {
                extract_images: Some(pdf.extract_images),
                passwords: pdf.passwords,
                extract_metadata: Some(pdf.extract_metadata),
                hierarchy: pdf.hierarchy.map(|h| JsHierarchyConfig {
                    enabled: Some(h.enabled),
                    k_clusters: Some(h.k_clusters as i32),
                    include_bbox: Some(h.include_bbox),
                    ocr_coverage_threshold: h.ocr_coverage_threshold.map(|v| v as f64),
                }),
                extract_annotations: Some(pdf.extract_annotations),
                top_margin_fraction: pdf.top_margin_fraction.map(|v| v as f64),
                bottom_margin_fraction: pdf.bottom_margin_fraction.map(|v| v as f64),
                allow_single_column_tables: Some(pdf.allow_single_column_tables),
            }),
            token_reduction: val.token_reduction.map(|tr| JsTokenReductionConfig {
                mode: Some(tr.mode),
                preserve_important_words: Some(tr.preserve_important_words),
            }),
            language_detection: val.language_detection.map(|ld| JsLanguageDetectionConfig {
                enabled: Some(ld.enabled),
                min_confidence: Some(ld.min_confidence),
                detect_multiple: Some(ld.detect_multiple),
            }),
            postprocessor: val.postprocessor.map(|pp| JsPostProcessorConfig {
                enabled: Some(pp.enabled),
                enabled_processors: pp.enabled_processors,
                disabled_processors: pp.disabled_processors,
            }),
            keywords: val.keywords.map(JsKeywordConfig::from),
            html_options: val.html_options.as_ref().map(JsHtmlOptions::from),
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as u32),
            pages: val.pages.map(JsPageConfig::from),
            output_format: Some(val.output_format.to_string()),
            result_format: Some(match val.result_format {
                kreuzberg::types::OutputFormat::Unified => "unified".to_string(),
                kreuzberg::types::OutputFormat::ElementBased => "element_based".to_string(),
            }),
            include_document_structure: Some(val.include_document_structure),
            layout: val.layout.map(JsLayoutDetectionConfig::from),
            email: val.email.map(|e| JsEmailConfig {
                msg_fallback_codepage: e.msg_fallback_codepage,
            }),
            acceleration: val.acceleration.map(|a| JsAccelerationConfig {
                provider: Some(match a.provider {
                    RustExecutionProviderType::Auto => "auto".to_string(),
                    RustExecutionProviderType::Cpu => "cpu".to_string(),
                    RustExecutionProviderType::CoreMl => "coreml".to_string(),
                    RustExecutionProviderType::Cuda => "cuda".to_string(),
                    RustExecutionProviderType::TensorRt => "tensorrt".to_string(),
                }),
                device_id: Some(a.device_id),
            }),
            security_limits: val.security_limits.map(|sl| JsSecurityLimits {
                max_archive_size: Some(sl.max_archive_size as u32),
                max_compression_ratio: Some(sl.max_compression_ratio as u32),
                max_files_in_archive: Some(sl.max_files_in_archive as u32),
                max_nesting_depth: Some(sl.max_nesting_depth as u32),
                max_entity_length: Some(sl.max_entity_length as u32),
                max_content_size: Some(sl.max_content_size as u32),
                max_iterations: Some(sl.max_iterations as u32),
                max_xml_depth: Some(sl.max_xml_depth as u32),
                max_table_cells: Some(sl.max_table_cells as u32),
            }),
            concurrency: val.concurrency.map(|c| JsConcurrencyConfig {
                max_threads: c.max_threads.map(|v| v as u32),
            }),
            cache_namespace: val.cache_namespace,
            cache_ttl_secs: val.cache_ttl_secs.map(|v| v as u32),
            max_archive_depth: Some(val.max_archive_depth as u32),
            extraction_timeout_secs: val.extraction_timeout_secs.map(|v| v as u32),
            tree_sitter: val.tree_sitter.map(JsTreeSitterConfig::from),
            structured_extraction: val.structured_extraction.map(JsStructuredExtractionConfig::from),
            content_filter: val.content_filter.map(JsContentFilterConfig::from),
            html_output: val.html_output.map(JsHtmlOutputConfig::from),
        })
    }
}

/// Load extraction configuration from a file.
///
/// Automatically detects the file format based on extension:
/// - `.toml` - TOML format
/// - `.yaml` - YAML format
/// - `.json` - JSON format
///
/// # Parameters
///
/// * `file_path` - Path to the configuration file (absolute or relative)
///
/// # Returns
///
/// `JsExtractionConfig` object with loaded configuration.
///
/// # Errors
///
/// Throws an error if:
/// - File does not exist or is not accessible
/// - File content is not valid TOML/YAML/JSON
/// - Configuration structure is invalid
///
/// # Example
///
/// ```typescript
/// import { loadExtractionConfigFromFile } from 'kreuzberg';
///
/// // Load from TOML file
/// const config = loadExtractionConfigFromFile('kreuzberg.toml');
///
/// // Load from YAML file
/// const config2 = loadExtractionConfigFromFile('./config.yaml');
///
/// // Use with extraction
/// const result = await extractFile('document.pdf', null, config);
/// ```
#[napi(js_name = "loadExtractionConfigFromFile")]
pub fn load_extraction_config_from_file(file_path: String) -> Result<JsExtractionConfig> {
    use crate::error_handling::convert_error;

    let path = std::path::Path::new(&file_path);

    let ext = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        napi::Error::new(
            napi::Status::InvalidArg,
            "File path must have an extension (.toml, .yaml, .json)",
        )
    })?;

    let rust_config = match ext.to_lowercase().as_str() {
        "toml" => ExtractionConfig::from_toml_file(path).map_err(convert_error)?,
        "yaml" | "yml" => ExtractionConfig::from_yaml_file(path).map_err(convert_error)?,
        "json" => ExtractionConfig::from_json_file(path).map_err(convert_error)?,
        _ => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                format!("Unsupported file extension: '{}'. Supported: .toml, .yaml, .json", ext),
            ));
        }
    };

    JsExtractionConfig::try_from(rust_config)
}

/// Discover extraction configuration file in current directory or parent directories.
///
/// Searches for configuration files in the following order:
/// 1. `kreuzberg.toml`
/// 2. `kreuzberg.yaml` / `kreuzberg.yml`
/// 3. `kreuzberg.json`
/// 4. Searches parent directories up to the filesystem root
///
/// Returns the first configuration file found or throws an error if none found.
///
/// # Returns
///
/// `JsExtractionConfig` object with discovered configuration.
///
/// # Errors
///
/// Throws an error if no configuration file is found.
///
/// # Example
///
/// ```typescript
/// import { discoverExtractionConfig } from 'kreuzberg';
///
/// // Automatically finds kreuzberg.toml or kreuzberg.yaml in current or parent directories
/// const config = discoverExtractionConfig();
/// const result = await extractFile('document.pdf', null, config);
/// ```
#[napi(js_name = "discoverExtractionConfig")]
pub fn discover_extraction_config() -> Result<Option<JsExtractionConfig>> {
    use crate::error_handling::convert_error;

    let rust_config = ExtractionConfig::discover().map_err(convert_error)?;

    match rust_config {
        Some(config) => Ok(Some(JsExtractionConfig::try_from(config)?)),
        None => Ok(None),
    }
}

#[napi(object)]
pub struct JsFileExtractionConfig {
    pub enable_quality_processing: Option<bool>,
    pub ocr: Option<JsOcrConfig>,
    pub force_ocr: Option<bool>,
    /// Disable OCR entirely — image files return empty content instead of errors
    pub disable_ocr: Option<bool>,
    /// List of 1-indexed page numbers to force OCR on (None = use force_ocr setting)
    pub force_ocr_pages: Option<Vec<u32>>,
    pub chunking: Option<JsChunkingConfig>,
    pub images: Option<JsImageExtractionConfig>,
    pub pdf_options: Option<JsPdfConfig>,
    pub token_reduction: Option<JsTokenReductionConfig>,
    pub language_detection: Option<JsLanguageDetectionConfig>,
    pub postprocessor: Option<JsPostProcessorConfig>,
    pub keywords: Option<JsKeywordConfig>,
    pub html_options: Option<JsHtmlOptions>,
    pub pages: Option<JsPageConfig>,
    /// Output text format: "plain" | "markdown" | "djot" | "html"
    pub output_format: Option<String>,
    /// Result structure format: "unified" | "element_based"
    pub result_format: Option<String>,
    /// Include document structure in extraction result
    pub include_document_structure: Option<bool>,
    /// Layout detection configuration (None = layout detection disabled)
    pub layout: Option<JsLayoutDetectionConfig>,
    /// Per-file extraction timeout in seconds
    pub timeout_secs: Option<u32>,
    /// Tree-sitter language pack configuration for code analysis
    pub tree_sitter: Option<JsTreeSitterConfig>,
    /// Structured extraction configuration for LLM-based data extraction
    pub structured_extraction: Option<JsStructuredExtractionConfig>,
    /// Content filtering configuration for headers/footers/watermarks
    pub content_filter: Option<JsContentFilterConfig>,
}

impl TryFrom<JsFileExtractionConfig> for FileExtractionConfig {
    type Error = Error;

    fn try_from(val: JsFileExtractionConfig) -> Result<Self> {
        let html_options = match val.html_options {
            Some(options) => Some(ConversionOptions::try_from(options)?),
            None => None,
        };

        let keywords = match val.keywords {
            Some(config) => Some(RustKeywordConfig::try_from(config)?),
            None => None,
        };

        Ok(FileExtractionConfig {
            enable_quality_processing: val.enable_quality_processing,
            ocr: val.ocr.map(Into::into),
            force_ocr: val.force_ocr,
            disable_ocr: val.disable_ocr,
            force_ocr_pages: val.force_ocr_pages.map(|v| v.into_iter().map(|p| p as usize).collect()),
            chunking: val.chunking.map(Into::into),
            content_filter: val.content_filter.map(Into::into),
            images: val.images.map(Into::into),
            pdf_options: val.pdf_options.map(Into::into),
            token_reduction: val.token_reduction.map(Into::into),
            language_detection: val.language_detection.map(Into::into),
            keywords,
            postprocessor: val.postprocessor.map(Into::into),
            html_options,
            pages: val.pages.map(|p| p.try_into()).transpose()?,
            output_format: val
                .output_format
                .map(|s| s.parse())
                .transpose()
                .map_err(|e: String| Error::new(Status::InvalidArg, e))?,
            result_format: val
                .result_format
                .map(|s| match s.as_str() {
                    "unified" => Ok(kreuzberg::types::OutputFormat::Unified),
                    "element_based" => Ok(kreuzberg::types::OutputFormat::ElementBased),
                    other => Err(Error::new(
                        Status::InvalidArg,
                        format!(
                            "Invalid result_format: {}. Expected 'unified' or 'element_based'",
                            other
                        ),
                    )),
                })
                .transpose()?,
            include_document_structure: val.include_document_structure,
            layout: val.layout.map(Into::into),
            timeout_secs: val.timeout_secs.map(|v| v as u64),
            tree_sitter: val.tree_sitter.map(Into::into),
            structured_extraction: val.structured_extraction.map(Into::into),
        })
    }
}

impl TryFrom<FileExtractionConfig> for JsFileExtractionConfig {
    type Error = napi::Error;

    fn try_from(val: FileExtractionConfig) -> Result<Self> {
        Ok(JsFileExtractionConfig {
            enable_quality_processing: val.enable_quality_processing,
            ocr: val.ocr.map(|ocr| JsOcrConfig {
                enabled: Some(ocr.enabled),
                backend: ocr.backend,
                language: Some(ocr.language),
                tesseract_config: ocr.tesseract_config.map(|tc| JsTesseractConfig {
                    psm: Some(tc.psm),
                    enable_table_detection: Some(tc.enable_table_detection),
                    tessedit_char_whitelist: if tc.tessedit_char_whitelist.is_empty() {
                        None
                    } else {
                        Some(tc.tessedit_char_whitelist)
                    },
                }),
                paddle_ocr_config: ocr
                    .paddle_ocr_config
                    .and_then(|v| serde_json::from_value::<kreuzberg::PaddleOcrConfig>(v).ok())
                    .map(|p| JsPaddleOcrConfig {
                        cache_dir: p.cache_dir.map(|p| p.to_string_lossy().into_owned()),
                        use_angle_cls: Some(p.use_angle_cls),
                        enable_table_detection: Some(p.enable_table_detection),
                        det_db_thresh: Some(p.det_db_thresh as f64),
                        det_db_box_thresh: Some(p.det_db_box_thresh as f64),
                        det_db_unclip_ratio: Some(p.det_db_unclip_ratio as f64),
                        det_limit_side_len: Some(p.det_limit_side_len),
                        rec_batch_num: Some(p.rec_batch_num),
                        min_confidence: None,
                        output_format: None,
                    }),
                element_config: ocr.element_config.map(|ec| JsOcrElementConfig {
                    include_elements: Some(ec.include_elements),
                    min_level: Some(match ec.min_level {
                        kreuzberg::OcrElementLevel::Word => "word".to_string(),
                        kreuzberg::OcrElementLevel::Line => "line".to_string(),
                        kreuzberg::OcrElementLevel::Block => "block".to_string(),
                        kreuzberg::OcrElementLevel::Page => "page".to_string(),
                    }),
                    min_confidence: Some(ec.min_confidence),
                    build_hierarchy: Some(ec.build_hierarchy),
                }),
                vlm_config: ocr.vlm_config.map(JsLlmConfig::from),
                vlm_prompt: ocr.vlm_prompt,
            }),
            force_ocr: val.force_ocr,
            disable_ocr: val.disable_ocr,
            force_ocr_pages: val.force_ocr_pages.map(|v| v.into_iter().map(|p| p as u32).collect()),
            chunking: val.chunking.map(|chunk| JsChunkingConfig {
                max_chars: Some(chunk.max_characters as u32),
                max_overlap: Some(chunk.overlap as u32),
                embedding: chunk.embedding.map(|emb| JsEmbeddingConfig {
                    model: Some(JsEmbeddingModelType {
                        model_type: match emb.model {
                            RustEmbeddingModelType::Preset { .. } => "preset".to_string(),
                            RustEmbeddingModelType::Custom { .. } => "custom".to_string(),
                            RustEmbeddingModelType::Llm { .. } => "llm".to_string(),
                        },
                        value: match &emb.model {
                            RustEmbeddingModelType::Preset { name } => name.clone(),
                            RustEmbeddingModelType::Custom { model_id, .. } => model_id.clone(),
                            RustEmbeddingModelType::Llm { llm } => llm.model.clone(),
                        },
                        dimensions: match emb.model {
                            RustEmbeddingModelType::Custom { dimensions, .. } => Some(dimensions as u32),
                            _ => None,
                        },
                    }),
                    normalize: Some(emb.normalize),
                    batch_size: Some(emb.batch_size as u32),
                    show_download_progress: Some(emb.show_download_progress),
                    cache_dir: emb.cache_dir.and_then(|p| p.to_str().map(String::from)),
                    acceleration: emb.acceleration.map(|a| JsAccelerationConfig {
                        provider: Some(match a.provider {
                            RustExecutionProviderType::Auto => "auto".to_string(),
                            RustExecutionProviderType::Cpu => "cpu".to_string(),
                            RustExecutionProviderType::CoreMl => "coreml".to_string(),
                            RustExecutionProviderType::Cuda => "cuda".to_string(),
                            RustExecutionProviderType::TensorRt => "tensorrt".to_string(),
                        }),
                        device_id: Some(a.device_id),
                    }),
                }),
                preset: chunk.preset,
                chunker_type: match chunk.chunker_type {
                    ChunkerType::Text => None,
                    ChunkerType::Markdown => Some("markdown".to_string()),
                    ChunkerType::Yaml => Some("yaml".to_string()),
                    ChunkerType::Semantic => Some("semantic".to_string()),
                },
                sizing_type: match &chunk.sizing {
                    kreuzberg::ChunkSizing::Characters => None,
                    kreuzberg::ChunkSizing::Tokenizer { .. } => Some("tokenizer".to_string()),
                },
                sizing_model: match &chunk.sizing {
                    kreuzberg::ChunkSizing::Tokenizer { model, .. } => Some(model.clone()),
                    _ => None,
                },
                sizing_cache_dir: match &chunk.sizing {
                    kreuzberg::ChunkSizing::Tokenizer { cache_dir, .. } => {
                        cache_dir.as_ref().and_then(|p| p.to_str().map(String::from))
                    }
                    _ => None,
                },
                prepend_heading_context: Some(chunk.prepend_heading_context),
                topic_threshold: chunk.topic_threshold.map(|t| t as f64),
            }),
            images: val.images.map(|img| JsImageExtractionConfig {
                extract_images: Some(img.extract_images),
                target_dpi: Some(img.target_dpi),
                max_image_dimension: Some(img.max_image_dimension),
                inject_placeholders: Some(img.inject_placeholders),
                auto_adjust_dpi: Some(img.auto_adjust_dpi),
                min_dpi: Some(img.min_dpi),
                max_dpi: Some(img.max_dpi),
                max_images_per_page: img.max_images_per_page,
            }),
            pdf_options: val.pdf_options.map(|pdf| JsPdfConfig {
                extract_images: Some(pdf.extract_images),
                passwords: pdf.passwords,
                extract_metadata: Some(pdf.extract_metadata),
                hierarchy: pdf.hierarchy.map(|h| JsHierarchyConfig {
                    enabled: Some(h.enabled),
                    k_clusters: Some(h.k_clusters as i32),
                    include_bbox: Some(h.include_bbox),
                    ocr_coverage_threshold: h.ocr_coverage_threshold.map(|v| v as f64),
                }),
                extract_annotations: Some(pdf.extract_annotations),
                top_margin_fraction: pdf.top_margin_fraction.map(|v| v as f64),
                bottom_margin_fraction: pdf.bottom_margin_fraction.map(|v| v as f64),
                allow_single_column_tables: Some(pdf.allow_single_column_tables),
            }),
            token_reduction: val.token_reduction.map(|tr| JsTokenReductionConfig {
                mode: Some(tr.mode),
                preserve_important_words: Some(tr.preserve_important_words),
            }),
            language_detection: val.language_detection.map(|ld| JsLanguageDetectionConfig {
                enabled: Some(ld.enabled),
                min_confidence: Some(ld.min_confidence),
                detect_multiple: Some(ld.detect_multiple),
            }),
            postprocessor: val.postprocessor.map(|pp| JsPostProcessorConfig {
                enabled: Some(pp.enabled),
                enabled_processors: pp.enabled_processors,
                disabled_processors: pp.disabled_processors,
            }),
            keywords: val.keywords.map(JsKeywordConfig::from),
            html_options: val.html_options.as_ref().map(JsHtmlOptions::from),
            pages: val.pages.map(JsPageConfig::from),
            output_format: val.output_format.map(|f| f.to_string()),
            result_format: val.result_format.map(|f| match f {
                kreuzberg::types::OutputFormat::Unified => "unified".to_string(),
                kreuzberg::types::OutputFormat::ElementBased => "element_based".to_string(),
            }),
            include_document_structure: val.include_document_structure,
            layout: val.layout.map(JsLayoutDetectionConfig::from),
            timeout_secs: val.timeout_secs.map(|v| v as u32),
            tree_sitter: val.tree_sitter.map(JsTreeSitterConfig::from),
            structured_extraction: val.structured_extraction.map(JsStructuredExtractionConfig::from),
            content_filter: val.content_filter.map(JsContentFilterConfig::from),
        })
    }
}
