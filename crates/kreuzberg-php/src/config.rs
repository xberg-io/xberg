//! Configuration type bindings
//!
//! Provides PHP-friendly wrappers around the Rust configuration structs.

use ext_php_rs::prelude::*;

/// Main extraction configuration.
///
/// Controls all aspects of document extraction including OCR, PDF rendering,
/// chunking, caching, and post-processing.
///
/// # Example
///
/// ```php
/// $config = new ExtractionConfig();
/// $config->use_cache = true;
/// $config->force_ocr = false;
/// $config->ocr = new OcrConfig();
/// $config->ocr->language = "eng";
/// ```
#[php_class]
#[derive(Clone, Default)]
pub struct ExtractionConfig {
    pub use_cache: bool,
    pub enable_quality_processing: bool,
    pub force_ocr: bool,
    pub ocr: Option<OcrConfig>,
    pub pdf_options: Option<PdfConfig>,
    pub chunking: Option<ChunkingConfig>,
    pub images: Option<ImageExtractionConfig>,
    pub token_reduction: Option<TokenReductionConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub keywords: Option<KeywordConfig>,
    pub postprocessor: Option<PostProcessorConfig>,
    pub max_concurrent_extractions: Option<usize>,
    pub pages: Option<PageConfig>,
}

#[php_impl]
impl ExtractionConfig {
    /// Create a new default configuration.
    pub fn __construct() -> Self {
        Self {
            use_cache: true,
            enable_quality_processing: true,
            force_ocr: false,
            ocr: None,
            pdf_options: None,
            chunking: None,
            images: None,
            token_reduction: None,
            language_detection: None,
            keywords: None,
            postprocessor: None,
            max_concurrent_extractions: None,
            pages: None,
        }
    }

    /// Load configuration from a TOML file.
    pub fn from_file(path: String) -> PhpResult<Self> {
        let rust_config =
            kreuzberg::ExtractionConfig::from_file(&path).map_err(|e| format!("Failed to load config: {}", e))?;
        Ok(Self::from_rust(rust_config))
    }

    /// Discover configuration file in current or parent directories.
    pub fn discover() -> PhpResult<Self> {
        let rust_config = kreuzberg::ExtractionConfig::discover()
            .map_err(|e| format!("Failed to discover config: {}", e))?
            .unwrap_or_default();
        Ok(Self::from_rust(rust_config))
    }
}

impl ExtractionConfig {
    /// Convert from PHP config to Rust config.
    pub fn to_rust(&self) -> kreuzberg::ExtractionConfig {
        kreuzberg::ExtractionConfig {
            use_cache: self.use_cache,
            enable_quality_processing: self.enable_quality_processing,
            ocr: self.ocr.as_ref().map(|c| c.to_rust()),
            force_ocr: self.force_ocr,
            chunking: self.chunking.as_ref().map(|c| c.to_rust()),
            images: self.images.as_ref().map(|c| c.to_rust()),
            pdf_options: self.pdf_options.as_ref().map(|c| c.to_rust()),
            token_reduction: self.token_reduction.as_ref().map(|c| c.to_rust()),
            language_detection: self.language_detection.as_ref().map(|c| c.to_rust()),
            keywords: self.keywords.as_ref().map(|c| c.to_rust()),
            postprocessor: self.postprocessor.as_ref().map(|c| c.to_rust()),
            html_options: None,
            max_concurrent_extractions: self.max_concurrent_extractions,
            pages: self.pages.as_ref().map(|c| c.to_rust()),
        }
    }

    /// Convert from Rust config to PHP config.
    pub fn from_rust(config: kreuzberg::ExtractionConfig) -> Self {
        Self {
            use_cache: config.use_cache,
            enable_quality_processing: config.enable_quality_processing,
            force_ocr: config.force_ocr,
            ocr: config.ocr.map(OcrConfig::from_rust),
            pdf_options: config.pdf_options.map(PdfConfig::from_rust),
            chunking: config.chunking.map(ChunkingConfig::from_rust),
            images: config.images.map(ImageExtractionConfig::from_rust),
            token_reduction: config.token_reduction.map(TokenReductionConfig::from_rust),
            language_detection: config.language_detection.map(LanguageDetectionConfig::from_rust),
            keywords: config.keywords.map(KeywordConfig::from_rust),
            postprocessor: config.postprocessor.map(PostProcessorConfig::from_rust),
            max_concurrent_extractions: config.max_concurrent_extractions,
            pages: config.pages.map(PageConfig::from_rust),
        }
    }
}

/// Convert an ExtractionConfig to JSON string.
///
/// # Parameters
///
/// - `config` (ExtractionConfig): The configuration to serialize
///
/// # Returns
///
/// JSON string representation of the configuration
///
/// # Throws
///
/// - Exception: If serialization fails
///
/// # Example
///
/// ```php
/// $config = new ExtractionConfig();
/// $config->use_cache = false;
/// $json = kreuzberg_config_to_json($config);
/// echo $json;
/// ```
#[php_function]
pub fn kreuzberg_config_to_json(config: &ExtractionConfig) -> PhpResult<String> {
    let rust_config = config.to_rust();
    serde_json::to_string_pretty(&rust_config).map_err(|e| format!("Failed to serialize config: {}", e).into())
}

/// Get a specific field from an ExtractionConfig as JSON.
///
/// # Parameters
///
/// - `config` (ExtractionConfig): The configuration object
/// - `field_name` (string): The field name to retrieve
///
/// # Returns
///
/// JSON string representation of the field value, or NULL if field doesn't exist
///
/// # Example
///
/// ```php
/// $config = new ExtractionConfig();
/// $value = kreuzberg_config_get_field($config, "use_cache");
/// // Returns: "true"
/// ```
#[php_function]
pub fn kreuzberg_config_get_field(config: &ExtractionConfig, field_name: String) -> Option<String> {
    let rust_config = config.to_rust();
    let json = serde_json::to_value(&rust_config).ok()?;
    let obj = json.as_object()?;
    let field = obj.get(&field_name)?;
    serde_json::to_string(field).ok()
}

/// Merge two ExtractionConfig objects.
///
/// Values from `override_config` will override values in `base`.
///
/// # Parameters
///
/// - `base` (ExtractionConfig): The base configuration
/// - `override_config` (ExtractionConfig): The configuration with override values
///
/// # Returns
///
/// New ExtractionConfig with merged values
///
/// # Example
///
/// ```php
/// $base = new ExtractionConfig();
/// $base->use_cache = true;
///
/// $override = new ExtractionConfig();
/// $override->use_cache = false;
///
/// $merged = kreuzberg_config_merge($base, $override);
/// // $merged->use_cache === false
/// ```
#[php_function]
pub fn kreuzberg_config_merge(
    base: &ExtractionConfig,
    override_config: &ExtractionConfig,
) -> PhpResult<ExtractionConfig> {
    let mut base_rust = base.to_rust();
    let override_rust = override_config.to_rust();

    base_rust.use_cache = override_rust.use_cache;
    base_rust.enable_quality_processing = override_rust.enable_quality_processing;
    base_rust.force_ocr = override_rust.force_ocr;

    if override_rust.ocr.is_some() {
        base_rust.ocr = override_rust.ocr;
    }
    if override_rust.pdf_options.is_some() {
        base_rust.pdf_options = override_rust.pdf_options;
    }
    if override_rust.chunking.is_some() {
        base_rust.chunking = override_rust.chunking;
    }
    if override_rust.images.is_some() {
        base_rust.images = override_rust.images;
    }
    if override_rust.token_reduction.is_some() {
        base_rust.token_reduction = override_rust.token_reduction;
    }
    if override_rust.language_detection.is_some() {
        base_rust.language_detection = override_rust.language_detection;
    }
    if override_rust.keywords.is_some() {
        base_rust.keywords = override_rust.keywords;
    }
    if override_rust.postprocessor.is_some() {
        base_rust.postprocessor = override_rust.postprocessor;
    }
    if override_rust.max_concurrent_extractions.is_some() {
        base_rust.max_concurrent_extractions = override_rust.max_concurrent_extractions;
    }
    if override_rust.pages.is_some() {
        base_rust.pages = override_rust.pages;
    }

    Ok(ExtractionConfig::from_rust(base_rust))
}

/// OCR configuration.
///
/// # Example
///
/// ```php
/// $ocr = new OcrConfig();
/// $ocr->backend = "tesseract";
/// $ocr->language = "eng";
/// ```
#[php_class]
#[derive(Clone)]
pub struct OcrConfig {
    pub backend: String,
    pub language: String,
    pub tesseract_config: Option<TesseractConfig>,
}

#[php_impl]
impl OcrConfig {
    pub fn __construct() -> Self {
        Self {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }
    }
}

impl OcrConfig {
    pub fn to_rust(&self) -> kreuzberg::OcrConfig {
        kreuzberg::OcrConfig {
            backend: self.backend.clone(),
            language: self.language.clone(),
            tesseract_config: self.tesseract_config.as_ref().map(|c| c.to_rust()),
        }
    }

    pub fn from_rust(config: kreuzberg::OcrConfig) -> Self {
        Self {
            backend: config.backend,
            language: config.language,
            tesseract_config: config.tesseract_config.map(TesseractConfig::from_rust),
        }
    }
}

/// PDF-specific configuration.
#[php_class]
#[derive(Clone)]
pub struct PdfConfig {
    pub extract_images: bool,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: bool,
}

#[php_impl]
impl PdfConfig {
    pub fn __construct() -> Self {
        Self {
            extract_images: false,
            passwords: None,
            extract_metadata: true,
        }
    }
}

impl PdfConfig {
    pub fn to_rust(&self) -> kreuzberg::PdfConfig {
        kreuzberg::PdfConfig {
            extract_images: self.extract_images,
            passwords: self.passwords.clone(),
            extract_metadata: self.extract_metadata,
        }
    }

    pub fn from_rust(config: kreuzberg::PdfConfig) -> Self {
        Self {
            extract_images: config.extract_images,
            passwords: config.passwords,
            extract_metadata: config.extract_metadata,
        }
    }
}

/// Chunking configuration.
#[php_class]
#[derive(Clone)]
pub struct ChunkingConfig {
    pub max_chars: usize,
    pub max_overlap: usize,
    pub embedding: Option<EmbeddingConfig>,
    pub preset: Option<String>,
}

#[php_impl]
impl ChunkingConfig {
    pub fn __construct() -> Self {
        Self {
            max_chars: 1000,
            max_overlap: 200,
            embedding: None,
            preset: None,
        }
    }
}

impl ChunkingConfig {
    pub fn to_rust(&self) -> kreuzberg::ChunkingConfig {
        kreuzberg::ChunkingConfig {
            max_chars: self.max_chars,
            max_overlap: self.max_overlap,
            embedding: self.embedding.as_ref().map(|c| c.to_rust()),
            preset: self.preset.clone(),
        }
    }

    pub fn from_rust(config: kreuzberg::ChunkingConfig) -> Self {
        Self {
            max_chars: config.max_chars,
            max_overlap: config.max_overlap,
            embedding: config.embedding.map(EmbeddingConfig::from_rust),
            preset: config.preset,
        }
    }
}

/// Embedding model type.
///
/// Specifies which model to use for embedding generation.
///
/// # Available Presets
///
/// - "fast": AllMiniLML6V2Q (384 dimensions) - Quick prototyping, low-latency
/// - "balanced": BGEBaseENV15 (768 dimensions) - General-purpose RAG (default)
/// - "quality": BGELargeENV15 (1024 dimensions) - High-quality embeddings
/// - "multilingual": MultilingualE5Base (768 dimensions) - Multi-language support
///
/// # Example
///
/// ```php
/// // Use a preset
/// $model = EmbeddingModelType::preset("balanced");
///
/// // Use a specific FastEmbed model
/// $model = EmbeddingModelType::fastembed("BGEBaseENV15", 768);
///
/// // Use a custom ONNX model
/// $model = EmbeddingModelType::custom("my-model", 512);
/// ```
#[php_class]
#[derive(Clone)]
pub struct EmbeddingModelType {
    model_type: String,
    name: String,
    dimensions: Option<usize>,
}

#[php_impl]
impl EmbeddingModelType {
    /// Create a model type from a preset name.
    ///
    /// # Parameters
    ///
    /// - `name` (string): Preset name ("fast", "balanced", "quality", "multilingual")
    ///
    /// # Example
    ///
    /// ```php
    /// $model = EmbeddingModelType::preset("balanced");
    /// ```
    pub fn preset(name: String) -> Self {
        Self {
            model_type: "preset".to_string(),
            name,
            dimensions: None,
        }
    }

    /// Create a model type from a FastEmbed model name.
    ///
    /// # Parameters
    ///
    /// - `model` (string): FastEmbed model name
    /// - `dimensions` (int): Embedding dimensions
    ///
    /// # Example
    ///
    /// ```php
    /// $model = EmbeddingModelType::fastembed("BGEBaseENV15", 768);
    /// ```
    pub fn fastembed(model: String, dimensions: i64) -> Self {
        Self {
            model_type: "fastembed".to_string(),
            name: model,
            dimensions: Some(dimensions as usize),
        }
    }

    /// Create a custom ONNX model type.
    ///
    /// # Parameters
    ///
    /// - `model_id` (string): Model identifier
    /// - `dimensions` (int): Embedding dimensions
    ///
    /// # Example
    ///
    /// ```php
    /// $model = EmbeddingModelType::custom("my-custom-model", 512);
    /// ```
    pub fn custom(model_id: String, dimensions: i64) -> Self {
        Self {
            model_type: "custom".to_string(),
            name: model_id,
            dimensions: Some(dimensions as usize),
        }
    }
}

impl EmbeddingModelType {
    pub fn to_rust(&self) -> kreuzberg::EmbeddingModelType {
        match self.model_type.as_str() {
            "preset" => kreuzberg::EmbeddingModelType::Preset {
                name: self.name.clone(),
            },
            "fastembed" => kreuzberg::EmbeddingModelType::FastEmbed {
                model: self.name.clone(),
                dimensions: self.dimensions.unwrap_or(768),
            },
            "custom" => kreuzberg::EmbeddingModelType::Custom {
                model_id: self.name.clone(),
                dimensions: self.dimensions.unwrap_or(768),
            },
            _ => kreuzberg::EmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
        }
    }

    pub fn from_rust(model: kreuzberg::EmbeddingModelType) -> Self {
        match model {
            kreuzberg::EmbeddingModelType::Preset { name } => Self {
                model_type: "preset".to_string(),
                name,
                dimensions: None,
            },
            kreuzberg::EmbeddingModelType::FastEmbed { model, dimensions } => Self {
                model_type: "fastembed".to_string(),
                name: model,
                dimensions: Some(dimensions),
            },
            kreuzberg::EmbeddingModelType::Custom { model_id, dimensions } => Self {
                model_type: "custom".to_string(),
                name: model_id,
                dimensions: Some(dimensions),
            },
        }
    }
}

/// Embedding configuration.
///
/// Controls embedding generation for text chunks.
///
/// # Example
///
/// ```php
/// $config = new EmbeddingConfig();
/// $config->model = EmbeddingModelType::preset("balanced");
/// $config->normalize = true;
/// $config->batch_size = 32;
/// ```
#[php_class]
#[derive(Clone)]
pub struct EmbeddingConfig {
    pub model: Option<EmbeddingModelType>,
    pub normalize: bool,
    pub batch_size: usize,
    pub show_download_progress: bool,
}

#[php_impl]
impl EmbeddingConfig {
    pub fn __construct() -> Self {
        Self {
            model: Some(EmbeddingModelType::preset("balanced".to_string())),
            normalize: true,
            batch_size: 32,
            show_download_progress: false,
        }
    }
}

impl EmbeddingConfig {
    pub fn to_rust(&self) -> kreuzberg::EmbeddingConfig {
        kreuzberg::EmbeddingConfig {
            model: self
                .model
                .as_ref()
                .map(|m| m.to_rust())
                .unwrap_or(kreuzberg::EmbeddingModelType::Preset {
                    name: "balanced".to_string(),
                }),
            normalize: self.normalize,
            batch_size: self.batch_size,
            show_download_progress: self.show_download_progress,
            cache_dir: None,
        }
    }

    pub fn from_rust(config: kreuzberg::EmbeddingConfig) -> Self {
        Self {
            model: Some(EmbeddingModelType::from_rust(config.model)),
            normalize: config.normalize,
            batch_size: config.batch_size,
            show_download_progress: config.show_download_progress,
        }
    }
}

/// Image extraction configuration.
#[php_class]
#[derive(Clone)]
pub struct ImageExtractionConfig {
    pub extract_images: bool,
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}

#[php_impl]
impl ImageExtractionConfig {
    pub fn __construct() -> Self {
        Self {
            extract_images: true,
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        }
    }
}

impl ImageExtractionConfig {
    pub fn to_rust(&self) -> kreuzberg::ImageExtractionConfig {
        kreuzberg::ImageExtractionConfig {
            extract_images: self.extract_images,
            target_dpi: self.target_dpi,
            max_image_dimension: self.max_image_dimension,
            auto_adjust_dpi: self.auto_adjust_dpi,
            min_dpi: self.min_dpi,
            max_dpi: self.max_dpi,
        }
    }

    pub fn from_rust(config: kreuzberg::ImageExtractionConfig) -> Self {
        Self {
            extract_images: config.extract_images,
            target_dpi: config.target_dpi,
            max_image_dimension: config.max_image_dimension,
            auto_adjust_dpi: config.auto_adjust_dpi,
            min_dpi: config.min_dpi,
            max_dpi: config.max_dpi,
        }
    }
}

/// Token reduction configuration.
#[php_class]
#[derive(Clone)]
pub struct TokenReductionConfig {
    pub mode: String,
    pub preserve_important_words: bool,
}

#[php_impl]
impl TokenReductionConfig {
    pub fn __construct() -> Self {
        Self {
            mode: "off".to_string(),
            preserve_important_words: true,
        }
    }
}

impl TokenReductionConfig {
    pub fn to_rust(&self) -> kreuzberg::TokenReductionConfig {
        kreuzberg::TokenReductionConfig {
            mode: self.mode.clone(),
            preserve_important_words: self.preserve_important_words,
        }
    }

    pub fn from_rust(config: kreuzberg::TokenReductionConfig) -> Self {
        Self {
            mode: config.mode,
            preserve_important_words: config.preserve_important_words,
        }
    }
}

/// Language detection configuration.
#[php_class]
#[derive(Clone)]
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub min_confidence: f64,
    pub detect_multiple: bool,
}

#[php_impl]
impl LanguageDetectionConfig {
    pub fn __construct() -> Self {
        Self {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        }
    }
}

impl LanguageDetectionConfig {
    pub fn to_rust(&self) -> kreuzberg::LanguageDetectionConfig {
        kreuzberg::LanguageDetectionConfig {
            enabled: self.enabled,
            min_confidence: self.min_confidence,
            detect_multiple: self.detect_multiple,
        }
    }

    pub fn from_rust(config: kreuzberg::LanguageDetectionConfig) -> Self {
        Self {
            enabled: config.enabled,
            min_confidence: config.min_confidence,
            detect_multiple: config.detect_multiple,
        }
    }
}

/// Keyword extraction configuration.
#[php_class]
#[derive(Clone)]
pub struct KeywordConfig {
    pub max_keywords: usize,
    pub min_score: f32,
    pub language: Option<String>,
}

#[php_impl]
impl KeywordConfig {
    pub fn __construct() -> Self {
        Self {
            max_keywords: 10,
            min_score: 0.0,
            language: Some("en".to_string()),
        }
    }
}

impl KeywordConfig {
    pub fn to_rust(&self) -> kreuzberg::keywords::KeywordConfig {
        kreuzberg::keywords::KeywordConfig {
            algorithm: kreuzberg::keywords::KeywordAlgorithm::Yake,
            max_keywords: self.max_keywords,
            min_score: self.min_score,
            ngram_range: (1, 3),
            language: self.language.clone(),
            yake_params: None,
            rake_params: None,
        }
    }

    pub fn from_rust(config: kreuzberg::keywords::KeywordConfig) -> Self {
        Self {
            max_keywords: config.max_keywords,
            min_score: config.min_score,
            language: config.language,
        }
    }
}

/// Post-processor configuration.
#[php_class]
#[derive(Clone)]
pub struct PostProcessorConfig {
    pub enabled: bool,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}

#[php_impl]
impl PostProcessorConfig {
    pub fn __construct() -> Self {
        Self {
            enabled: true,
            enabled_processors: None,
            disabled_processors: None,
        }
    }
}

impl PostProcessorConfig {
    pub fn to_rust(&self) -> kreuzberg::PostProcessorConfig {
        let enabled_set = self
            .enabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());
        let disabled_set = self
            .disabled_processors
            .as_ref()
            .map(|procs| procs.iter().cloned().collect());

        kreuzberg::PostProcessorConfig {
            enabled: self.enabled,
            enabled_processors: self.enabled_processors.clone(),
            disabled_processors: self.disabled_processors.clone(),
            enabled_set,
            disabled_set,
        }
    }

    pub fn from_rust(config: kreuzberg::PostProcessorConfig) -> Self {
        Self {
            enabled: config.enabled,
            enabled_processors: config.enabled_processors,
            disabled_processors: config.disabled_processors,
        }
    }
}

/// Tesseract-specific configuration.
#[php_class]
#[derive(Clone)]
pub struct TesseractConfig {
    pub language: String,
    pub psm: i32,
    pub output_format: String,
    pub oem: i32,
    pub min_confidence: f64,
    pub enable_table_detection: bool,
}

#[php_impl]
impl TesseractConfig {
    pub fn __construct() -> Self {
        Self {
            language: "eng".to_string(),
            psm: 3,
            output_format: "markdown".to_string(),
            oem: 3,
            min_confidence: 0.0,
            enable_table_detection: true,
        }
    }
}

impl TesseractConfig {
    pub fn to_rust(&self) -> kreuzberg::types::TesseractConfig {
        kreuzberg::types::TesseractConfig {
            language: self.language.clone(),
            psm: self.psm,
            output_format: self.output_format.clone(),
            oem: self.oem,
            min_confidence: self.min_confidence,
            preprocessing: None,
            enable_table_detection: self.enable_table_detection,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: false,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_char_blacklist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        }
    }

    pub fn from_rust(config: kreuzberg::types::TesseractConfig) -> Self {
        Self {
            language: config.language,
            psm: config.psm,
            output_format: config.output_format,
            oem: config.oem,
            min_confidence: config.min_confidence,
            enable_table_detection: config.enable_table_detection,
        }
    }
}

/// Page extraction configuration.
#[php_class]
#[derive(Clone)]
pub struct PageConfig {
    pub extract_pages: bool,
    pub insert_page_markers: bool,
    pub marker_format: String,
}

#[php_impl]
impl PageConfig {
    pub fn __construct() -> Self {
        Self {
            extract_pages: false,
            insert_page_markers: false,
            marker_format: "\n\n<!-- PAGE {page_num} -->\n\n".to_string(),
        }
    }
}

impl PageConfig {
    pub fn to_rust(&self) -> kreuzberg::core::config::PageConfig {
        kreuzberg::core::config::PageConfig {
            extract_pages: self.extract_pages,
            insert_page_markers: self.insert_page_markers,
            marker_format: self.marker_format.clone(),
        }
    }

    pub fn from_rust(config: kreuzberg::core::config::PageConfig) -> Self {
        Self {
            extract_pages: config.extract_pages,
            insert_page_markers: config.insert_page_markers,
            marker_format: config.marker_format,
        }
    }
}
