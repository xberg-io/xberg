//! CLI extraction overrides via `#[derive(clap::Args)]`.
//!
//! Provides `ExtractionOverrides`, a flattened clap struct that captures all
//! optional CLI flags for extraction configuration. Call `validate()` then
//! `apply()` to layer these overrides onto an `ExtractionConfig`.

use anyhow::{Result, bail};
use kreuzberg::{ChunkingConfig, ExecutionProviderType, ExtractionConfig, LanguageDetectionConfig, OcrConfig};

use crate::ContentOutputFormatArg;

/// Hardware acceleration provider for ONNX Runtime models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum AccelerationArg {
    /// Auto-detect best provider per platform.
    Auto,
    /// CPU execution provider (always available).
    Cpu,
    /// Apple CoreML (macOS/iOS Neural Engine + GPU).
    #[value(name = "coreml")]
    CoreMl,
    /// NVIDIA CUDA GPU acceleration.
    Cuda,
    /// NVIDIA TensorRT (optimized CUDA inference).
    #[value(name = "tensorrt")]
    TensorRt,
}

impl From<AccelerationArg> for ExecutionProviderType {
    fn from(arg: AccelerationArg) -> Self {
        match arg {
            AccelerationArg::Auto => ExecutionProviderType::Auto,
            AccelerationArg::Cpu => ExecutionProviderType::Cpu,
            AccelerationArg::CoreMl => ExecutionProviderType::CoreMl,
            AccelerationArg::Cuda => ExecutionProviderType::Cuda,
            AccelerationArg::TensorRt => ExecutionProviderType::TensorRt,
        }
    }
}

/// Token reduction intensity level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ReductionLevelArg {
    /// Disable token reduction.
    Off,
    /// Remove only the most obvious filler.
    Light,
    /// Balanced reduction (default when enabled).
    Moderate,
    /// Heavy reduction, may lose some nuance.
    Aggressive,
    /// Maximum compression, lossy.
    Maximum,
}

impl ReductionLevelArg {
    /// Convert to the string mode expected by `TokenReductionConfig`.
    fn as_mode_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Moderate => "moderate",
            Self::Aggressive => "aggressive",
            Self::Maximum => "maximum",
        }
    }
}

/// Optional CLI flags that override fields in `ExtractionConfig`.
///
/// Every field is `Option<T>` (or `Vec<T>` for repeatable flags) so that
/// only explicitly-provided flags take effect. Flatten this struct into any
/// clap command with `#[command(flatten)]`.
#[derive(Debug, Default, clap::Args)]
pub struct ExtractionOverrides {
    // ── OCR ───────────────────────────────────────────────────────────
    /// Enable or disable OCR. When true, configures an OCR backend
    /// (default: tesseract). When false, removes any OCR configuration.
    #[arg(long)]
    pub ocr: Option<bool>,

    /// OCR backend to use when --ocr is enabled (tesseract, paddle-ocr, easyocr).
    #[arg(long)]
    pub ocr_backend: Option<String>,

    /// OCR language code. Tesseract uses ISO 639-3 (eng, fra, deu).
    /// PaddleOCR/EasyOCR use short codes (en, ch, french, korean).
    #[arg(long)]
    pub ocr_language: Option<String>,

    /// Force OCR even if text extraction succeeds.
    #[arg(long)]
    pub force_ocr: Option<bool>,

    /// Disable extraction result caching.
    #[arg(long)]
    pub no_cache: Option<bool>,

    /// Enable automatic image rotation before OCR based on detected orientation.
    #[arg(long)]
    pub ocr_auto_rotate: Option<bool>,

    // ── Chunking ─────────────────────────────────────────────────────
    /// Enable or disable text chunking.
    #[arg(long)]
    pub chunk: Option<bool>,

    /// Maximum chunk size in characters.
    #[arg(long)]
    pub chunk_size: Option<usize>,

    /// Overlap between consecutive chunks in characters.
    #[arg(long)]
    pub chunk_overlap: Option<usize>,

    /// Tokenizer model for token-based chunk sizing (e.g. "Xenova/gpt-4o").
    /// Implicitly enables chunking. Requires the chunking-tokenizers feature.
    #[arg(long)]
    pub chunking_tokenizer: Option<String>,

    // ── Output ────────────────────────────────────────────────────────
    /// Content output format (plain, markdown, djot, html).
    /// Controls the format of extracted content, not the CLI display format.
    #[arg(long, value_enum)]
    pub output_format: Option<ContentOutputFormatArg>,

    /// Content output format (DEPRECATED: use --output-format instead).
    #[arg(long, value_enum, hide = true)]
    pub content_format: Option<ContentOutputFormatArg>,

    /// Include hierarchical document structure in results.
    #[arg(long)]
    pub include_structure: Option<bool>,

    // ── Quality & detection ──────────────────────────────────────────
    /// Enable quality post-processing.
    #[arg(long)]
    pub quality: Option<bool>,

    /// Enable language detection on extracted text.
    #[arg(long)]
    pub detect_language: Option<bool>,

    // ── Layout detection ─────────────────────────────────────────────
    /// Layout detection preset (e.g. "accurate").
    #[cfg(feature = "layout-detection")]
    #[arg(long)]
    pub layout_preset: Option<String>,

    /// Layout detection confidence threshold (0.0 - 1.0).
    #[cfg(feature = "layout-detection")]
    #[arg(long)]
    pub layout_confidence: Option<f32>,

    // ── Acceleration & concurrency ───────────────────────────────────
    /// ONNX Runtime execution provider for model inference.
    #[arg(long, value_enum)]
    pub acceleration: Option<AccelerationArg>,

    /// Maximum number of concurrent extractions in batch mode.
    #[arg(long, help = "Limit parallel extractions in batch mode")]
    pub max_concurrent: Option<usize>,

    /// Cap all internal thread pools (Rayon, ONNX intra-op, batch semaphore).
    #[arg(long, help = "Limit total threads for constrained environments")]
    pub max_threads: Option<usize>,

    // ── Pages ─────────────────────────────────────────────────────────
    /// Extract pages as a separate array in results.
    #[arg(long)]
    pub extract_pages: Option<bool>,

    /// Insert page marker comments into the main content string.
    #[arg(long)]
    pub page_markers: Option<bool>,

    // ── Images ────────────────────────────────────────────────────────
    /// Enable image extraction from documents.
    #[arg(long)]
    pub extract_images: Option<bool>,

    /// Target DPI for image normalisation (e.g. 150, 300, 600).
    #[arg(long)]
    pub target_dpi: Option<i32>,

    // ── PDF ───────────────────────────────────────────────────────────
    /// Password(s) for encrypted PDFs. Can be specified multiple times.
    #[arg(long)]
    pub pdf_password: Vec<String>,

    /// Extract images embedded in PDF pages.
    #[cfg(any(feature = "bundled-pdfium", feature = "static-pdfium"))]
    #[arg(long)]
    pub pdf_extract_images: Option<bool>,

    /// Extract PDF metadata (title, author, etc.).
    #[cfg(any(feature = "bundled-pdfium", feature = "static-pdfium"))]
    #[arg(long)]
    pub pdf_extract_metadata: Option<bool>,

    // ── Token reduction ──────────────────────────────────────────────
    /// Token reduction level (off, light, moderate, aggressive, maximum).
    #[arg(long, value_enum)]
    pub token_reduction: Option<ReductionLevelArg>,

    // ── Email ─────────────────────────────────────────────────────────
    /// Windows codepage fallback for MSG files without codepage metadata.
    /// Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western).
    #[arg(long)]
    pub msg_codepage: Option<u32>,

    // ── Cache ─────────────────────────────────────────────────────────
    /// Cache namespace for tenant isolation.
    #[arg(long)]
    pub cache_namespace: Option<String>,

    /// Per-request cache TTL in seconds (0 = skip cache).
    #[arg(long)]
    pub cache_ttl_secs: Option<u64>,
}

impl ExtractionOverrides {
    /// Validate flag combinations before applying.
    ///
    /// Call this before `apply()` to surface user-friendly errors for
    /// invalid or contradictory options.
    pub fn validate(&self) -> Result<()> {
        // Chunking validation
        if let Some(size) = self.chunk_size {
            if size == 0 {
                bail!("Invalid chunk size: {size}. Chunk size must be greater than 0.");
            }
            if size > 1_000_000 {
                bail!(
                    "Invalid chunk size: {size}. Chunk size must be less than 1,000,000 characters to avoid excessive memory usage."
                );
            }
        }

        if let Some(overlap) = self.chunk_overlap
            && let Some(size) = self.chunk_size
            && overlap >= size
        {
            bail!("Invalid chunk overlap: {overlap}. Overlap ({overlap}) must be less than chunk size ({size}).");
        }

        // Target DPI validation
        if let Some(dpi) = self.target_dpi
            && (!(36..=2400).contains(&dpi))
        {
            bail!("Invalid target DPI: {dpi}. Value must be between 36 and 2400.");
        }

        // Layout confidence validation
        #[cfg(feature = "layout-detection")]
        if let Some(conf) = self.layout_confidence
            && !(0.0..=1.0).contains(&conf)
        {
            bail!("Invalid layout confidence: {conf}. Value must be between 0.0 and 1.0.");
        }

        // Chunking tokenizer feature validation
        #[cfg(not(feature = "chunking-tokenizers"))]
        if self.chunking_tokenizer.is_some() {
            bail!(
                "--chunking-tokenizer requires the chunking-tokenizers feature. \
                 Rebuild with --features chunking-tokenizers"
            );
        }

        // OCR backend validation
        if let Some(ref backend) = self.ocr_backend
            && !["tesseract", "paddle-ocr", "easyocr"].contains(&backend.as_str())
        {
            bail!(
                "Invalid OCR backend '{}'. Valid backends: tesseract, paddle-ocr, easyocr",
                backend
            );
        }

        // Concurrency validation
        if let Some(0) = self.max_concurrent {
            bail!("--max-concurrent must be at least 1");
        }
        if let Some(0) = self.max_threads {
            bail!("--max-threads must be at least 1");
        }

        Ok(())
    }

    /// Apply these overrides onto an existing `ExtractionConfig`.
    ///
    /// Only fields that were explicitly provided on the command line take
    /// effect; everything else is left untouched.
    pub fn apply(self, config: &mut ExtractionConfig) {
        self.apply_ocr(config);
        self.apply_chunking(config);
        self.apply_quality_and_detection(config);
        self.apply_output_format(config);
        self.apply_include_structure(config);
        self.apply_layout(config);
        self.apply_acceleration(config);
        self.apply_concurrency(config);
        self.apply_pages(config);
        self.apply_images(config);
        self.apply_pdf(config);
        self.apply_token_reduction(config);
        self.apply_email(config);
        self.apply_cache(config);
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn apply_ocr(&self, config: &mut ExtractionConfig) {
        if let Some(ocr_flag) = self.ocr {
            if ocr_flag {
                let backend = match self.ocr_backend.as_deref() {
                    Some("paddle-ocr") => "paddle-ocr",
                    Some("easyocr") => "easyocr",
                    _ => "tesseract",
                };
                let language = match &self.ocr_language {
                    Some(lang) => lang.clone(),
                    None => match backend {
                        "paddle-ocr" | "easyocr" => "en".to_string(),
                        _ => "eng".to_string(),
                    },
                };
                // Preserve existing paddle_ocr_config and element_config from config file/inline JSON
                let existing_paddle_config = config.ocr.as_ref().and_then(|o| o.paddle_ocr_config.clone());
                let existing_element_config = config.ocr.as_ref().and_then(|o| o.element_config.clone());
                let auto_rotate = self.ocr_auto_rotate.unwrap_or(false);
                config.ocr = Some(OcrConfig {
                    backend: backend.to_string(),
                    language,
                    tesseract_config: None,
                    output_format: None,
                    paddle_ocr_config: existing_paddle_config,
                    element_config: existing_element_config,
                    quality_thresholds: None,
                    pipeline: None,
                    auto_rotate,
                });
            } else {
                config.ocr = None;
            }
        }

        // Override language on existing OCR config when --ocr-language is used without --ocr
        if self.ocr.is_none()
            && let Some(ref lang) = self.ocr_language
            && let Some(ref mut existing_ocr) = config.ocr
        {
            existing_ocr.language = lang.clone();
        }

        // Override auto_rotate on existing OCR config when used without --ocr
        if self.ocr.is_none()
            && let Some(rotate) = self.ocr_auto_rotate
            && let Some(ref mut existing_ocr) = config.ocr
        {
            existing_ocr.auto_rotate = rotate;
        }

        if let Some(force_ocr_flag) = self.force_ocr {
            config.force_ocr = force_ocr_flag;
        }
        if let Some(no_cache_flag) = self.no_cache {
            config.use_cache = !no_cache_flag;
        }
    }

    fn apply_chunking(&self, config: &mut ExtractionConfig) {
        // --chunking-tokenizer implicitly enables chunking
        let chunk = if self.chunking_tokenizer.is_some() && self.chunk.is_none() {
            Some(true)
        } else {
            self.chunk
        };

        if let Some(chunk_flag) = chunk {
            if chunk_flag {
                let max_characters = self.chunk_size.unwrap_or(1000);
                let overlap = self.chunk_overlap.unwrap_or(200);
                let mut chunking_config = ChunkingConfig {
                    max_characters,
                    overlap,
                    trim: true,
                    chunker_type: kreuzberg::ChunkerType::Text,
                    ..Default::default()
                };

                #[cfg(feature = "chunking-tokenizers")]
                if let Some(ref model) = self.chunking_tokenizer {
                    chunking_config.sizing = kreuzberg::ChunkSizing::Tokenizer {
                        model: model.clone(),
                        cache_dir: None,
                    };
                }

                config.chunking = Some(chunking_config);
            } else {
                config.chunking = None;
            }
        } else if let Some(ref mut chunking) = config.chunking {
            if let Some(max_characters) = self.chunk_size {
                chunking.max_characters = max_characters;
            }
            if let Some(overlap) = self.chunk_overlap {
                chunking.overlap = overlap;
            }

            // Clamp overlap when it exceeds max_characters (can happen when
            // only --chunk-overlap is provided against an existing config).
            if chunking.overlap >= chunking.max_characters {
                chunking.overlap = chunking.max_characters / 4;
            }

            #[cfg(feature = "chunking-tokenizers")]
            if let Some(ref model) = self.chunking_tokenizer {
                chunking.sizing = kreuzberg::ChunkSizing::Tokenizer {
                    model: model.clone(),
                    cache_dir: None,
                };
            }
        }
    }

    fn apply_quality_and_detection(&self, config: &mut ExtractionConfig) {
        if let Some(quality_flag) = self.quality {
            config.enable_quality_processing = quality_flag;
        }
        if let Some(detect_language_flag) = self.detect_language {
            if detect_language_flag {
                config.language_detection = Some(LanguageDetectionConfig {
                    enabled: true,
                    min_confidence: 0.8,
                    detect_multiple: false,
                });
            } else {
                config.language_detection = None;
            }
        }
    }

    fn apply_output_format(&self, config: &mut ExtractionConfig) {
        let final_output_format = self.output_format.or_else(|| {
            if self.content_format.is_some() {
                eprintln!("warning: '--content-format' is deprecated since 4.2.0, use '--output-format' instead");
            }
            self.content_format
        });

        if let Some(content_fmt) = final_output_format {
            config.output_format = content_fmt.into();
        }
    }

    fn apply_include_structure(&self, config: &mut ExtractionConfig) {
        if let Some(flag) = self.include_structure {
            config.include_document_structure = flag;
        }
    }

    #[allow(unused_variables)]
    fn apply_layout(&self, config: &mut ExtractionConfig) {
        #[cfg(feature = "layout-detection")]
        {
            let has_layout_flag = self.layout_preset.is_some() || self.layout_confidence.is_some();
            if has_layout_flag {
                let mut layout = config.layout.clone().unwrap_or_default();
                if let Some(ref preset) = self.layout_preset {
                    layout.preset = preset.clone();
                }
                if let Some(confidence) = self.layout_confidence {
                    layout.confidence_threshold = Some(confidence);
                }
                config.layout = Some(layout);
            }
        }
    }

    fn apply_acceleration(&self, config: &mut ExtractionConfig) {
        if let Some(accel) = self.acceleration {
            let mut accel_config = config.acceleration.clone().unwrap_or_default();
            accel_config.provider = accel.into();
            config.acceleration = Some(accel_config);
        }
    }

    fn apply_concurrency(&self, config: &mut ExtractionConfig) {
        if let Some(max_concurrent) = self.max_concurrent {
            config.max_concurrent_extractions = Some(max_concurrent);
        }
        if let Some(max_threads) = self.max_threads {
            let concurrency = config.concurrency.get_or_insert_with(Default::default);
            concurrency.max_threads = Some(max_threads);
        }
    }

    fn apply_pages(&self, config: &mut ExtractionConfig) {
        let has_page_flag = self.extract_pages.is_some() || self.page_markers.is_some();
        if has_page_flag {
            let mut page_config = config.pages.clone().unwrap_or_default();
            if let Some(extract) = self.extract_pages {
                page_config.extract_pages = extract;
            }
            if let Some(markers) = self.page_markers {
                page_config.insert_page_markers = markers;
            }
            config.pages = Some(page_config);
        }
    }

    fn apply_images(&self, config: &mut ExtractionConfig) {
        let has_image_flag = self.extract_images.is_some() || self.target_dpi.is_some();
        if has_image_flag {
            let mut img = config.images.clone().unwrap_or_default();
            if let Some(extract) = self.extract_images {
                img.extract_images = extract;
            }
            if let Some(dpi) = self.target_dpi {
                img.target_dpi = dpi;
            }
            config.images = Some(img);
        }
    }

    #[allow(unused_variables)]
    fn apply_pdf(&self, config: &mut ExtractionConfig) {
        #[cfg(any(feature = "bundled-pdfium", feature = "static-pdfium"))]
        {
            let has_pdf_flag = self.pdf_extract_images.is_some()
                || self.pdf_extract_metadata.is_some()
                || !self.pdf_password.is_empty();
            if has_pdf_flag {
                let pdf_opts = config.pdf_options.get_or_insert_with(Default::default);
                if let Some(extract_img) = self.pdf_extract_images {
                    pdf_opts.extract_images = extract_img;
                }
                if let Some(extract_meta) = self.pdf_extract_metadata {
                    pdf_opts.extract_metadata = extract_meta;
                }
                if !self.pdf_password.is_empty() {
                    pdf_opts.passwords = Some(self.pdf_password.clone());
                }
            }
        }

        // Handle pdf_password even without pdfium features for the
        // common case where pdf is enabled through other means.
        #[cfg(not(any(feature = "bundled-pdfium", feature = "static-pdfium")))]
        if !self.pdf_password.is_empty() {
            let pdf_opts = config.pdf_options.get_or_insert_with(Default::default);
            pdf_opts.passwords = Some(self.pdf_password.clone());
        }
    }

    fn apply_token_reduction(&self, config: &mut ExtractionConfig) {
        if let Some(level) = self.token_reduction {
            config.token_reduction = Some(kreuzberg::TokenReductionConfig {
                mode: level.as_mode_str().to_string(),
                preserve_important_words: true,
            });
        }
    }

    fn apply_email(&self, config: &mut ExtractionConfig) {
        if let Some(codepage) = self.msg_codepage {
            let email = config.email.get_or_insert_with(Default::default);
            email.msg_fallback_codepage = Some(codepage);
        }
    }

    fn apply_cache(&self, config: &mut ExtractionConfig) {
        if let Some(ns) = &self.cache_namespace {
            config.cache_namespace = Some(ns.clone());
        }
        if let Some(ttl) = self.cache_ttl_secs {
            config.cache_ttl_secs = Some(ttl);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kreuzberg::ExtractionConfig;

    fn default_overrides() -> ExtractionOverrides {
        ExtractionOverrides::default()
    }

    // ── OCR tests ────────────────────────────────────────────────────

    #[test]
    fn test_ocr_default_language_tesseract() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "eng");
    }

    #[test]
    fn test_ocr_default_language_paddleocr() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("paddle-ocr".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "paddle-ocr");
        assert_eq!(ocr.language, "en");
    }

    #[test]
    fn test_ocr_default_language_easyocr() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("easyocr".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "easyocr");
        assert_eq!(ocr.language, "en");
    }

    #[test]
    fn test_ocr_language_override_tesseract() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_language: Some("fra".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "fra");
    }

    #[test]
    fn test_ocr_language_override_paddleocr() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("paddle-ocr".to_string()),
            ocr_language: Some("ch".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "paddle-ocr");
        assert_eq!(ocr.language, "ch");
    }

    #[test]
    fn test_ocr_language_without_ocr_flag_no_existing_config() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        // No OCR config exists, so --ocr-language alone doesn't create one
        assert!(config.ocr.is_none());
    }

    #[test]
    fn test_ocr_language_without_ocr_flag_existing_config() {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
            }),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "deu");
    }

    #[test]
    fn test_ocr_disabled_ignores_language() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(false),
            ocr_language: Some("fra".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        assert!(config.ocr.is_none());
    }

    // ── Chunking tests ───────────────────────────────────────────────

    #[test]
    fn test_chunking_enabled_defaults() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            chunk: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let chunking = config.chunking.unwrap();
        assert_eq!(chunking.max_characters, 1000);
        assert_eq!(chunking.overlap, 200);
    }

    #[test]
    fn test_chunking_custom_size() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            chunk: Some(true),
            chunk_size: Some(500),
            chunk_overlap: Some(50),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let chunking = config.chunking.unwrap();
        assert_eq!(chunking.max_characters, 500);
        assert_eq!(chunking.overlap, 50);
    }

    #[test]
    fn test_chunking_disabled() {
        let mut config = ExtractionConfig {
            chunking: Some(ChunkingConfig::default()),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            chunk: Some(false),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        assert!(config.chunking.is_none());
    }

    // ── Validation tests ─────────────────────────────────────────────

    #[test]
    fn test_validate_chunk_size_zero() {
        let overrides = ExtractionOverrides {
            chunk_size: Some(0),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[test]
    fn test_validate_chunk_size_too_large() {
        let overrides = ExtractionOverrides {
            chunk_size: Some(2_000_000),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[test]
    fn test_validate_overlap_exceeds_size() {
        let overrides = ExtractionOverrides {
            chunk_size: Some(100),
            chunk_overlap: Some(200),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[test]
    fn test_validate_target_dpi_out_of_range() {
        let overrides = ExtractionOverrides {
            target_dpi: Some(5),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());

        let overrides = ExtractionOverrides {
            target_dpi: Some(5000),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[test]
    fn test_validate_target_dpi_valid() {
        let overrides = ExtractionOverrides {
            target_dpi: Some(300),
            ..default_overrides()
        };
        assert!(overrides.validate().is_ok());
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_validate_layout_confidence_out_of_range() {
        let overrides = ExtractionOverrides {
            layout_confidence: Some(1.5),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());

        let overrides = ExtractionOverrides {
            layout_confidence: Some(-0.1),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_validate_layout_confidence_valid() {
        let overrides = ExtractionOverrides {
            layout_confidence: Some(0.5),
            ..default_overrides()
        };
        assert!(overrides.validate().is_ok());
    }

    // ── Layout tests ─────────────────────────────────────────────────

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_layout_preset_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            layout_preset: Some("accurate".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let layout = config.layout.unwrap();
        assert_eq!(layout.preset, "accurate");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_layout_confidence_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            layout_confidence: Some(0.7),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let layout = config.layout.unwrap();
        assert_eq!(layout.confidence_threshold, Some(0.7));
    }

    // ── Acceleration tests ───────────────────────────────────────────

    #[test]
    fn test_acceleration_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            acceleration: Some(AccelerationArg::Cpu),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let accel = config.acceleration.unwrap();
        assert_eq!(accel.provider, ExecutionProviderType::Cpu);
    }

    // ── Pages tests ──────────────────────────────────────────────────

    #[test]
    fn test_extract_pages_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            extract_pages: Some(true),
            page_markers: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let pages = config.pages.unwrap();
        assert!(pages.extract_pages);
        assert!(pages.insert_page_markers);
    }

    // ── Images tests ─────────────────────────────────────────────────

    #[test]
    fn test_extract_images_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            extract_images: Some(true),
            target_dpi: Some(150),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let images = config.images.unwrap();
        assert!(images.extract_images);
        assert_eq!(images.target_dpi, 150);
    }

    // ── Token reduction tests ────────────────────────────────────────

    #[test]
    fn test_token_reduction_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            token_reduction: Some(ReductionLevelArg::Aggressive),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let reduction = config.token_reduction.unwrap();
        assert_eq!(reduction.mode, "aggressive");
    }

    // ── Email tests ──────────────────────────────────────────────────

    #[test]
    fn test_msg_codepage_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            msg_codepage: Some(1251),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let email = config.email.unwrap();
        assert_eq!(email.msg_fallback_codepage, Some(1251));
    }

    // ── Concurrency tests ────────────────────────────────────────────

    #[test]
    fn test_max_concurrent_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            max_concurrent: Some(4),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        assert_eq!(config.max_concurrent_extractions, Some(4));
    }

    #[test]
    fn test_max_threads_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            max_threads: Some(2),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let concurrency = config.concurrency.unwrap();
        assert_eq!(concurrency.max_threads, Some(2));
    }

    // ── Include structure tests ──────────────────────────────────────

    #[test]
    fn test_include_structure_applied() {
        let mut config = ExtractionConfig::default();
        assert!(!config.include_document_structure);
        let overrides = ExtractionOverrides {
            include_structure: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        assert!(config.include_document_structure);
    }

    #[test]
    fn test_validate_invalid_ocr_backend() {
        let overrides = ExtractionOverrides {
            ocr_backend: Some("invalid-backend".to_string()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(err.to_string().contains("Invalid OCR backend"));
    }

    #[test]
    fn test_validate_max_concurrent_zero() {
        let overrides = ExtractionOverrides {
            max_concurrent: Some(0),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(err.to_string().contains("--max-concurrent must be at least 1"));
    }

    #[test]
    fn test_validate_max_threads_zero() {
        let overrides = ExtractionOverrides {
            max_threads: Some(0),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(err.to_string().contains("--max-threads must be at least 1"));
    }

    #[test]
    fn test_validate_valid_ocr_backends() {
        for backend in &["tesseract", "paddle-ocr", "easyocr"] {
            let overrides = ExtractionOverrides {
                ocr_backend: Some(backend.to_string()),
                ..default_overrides()
            };
            assert!(overrides.validate().is_ok(), "Expected backend '{backend}' to be valid");
        }
    }

    // ── No-op when no flags provided ─────────────────────────────────

    // ── Overlap clamping tests ─────────────────────────────────────

    #[test]
    fn test_chunk_overlap_clamped_on_existing_config() {
        let mut config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 800,
                overlap: 100,
                ..Default::default()
            }),
            ..Default::default()
        };
        // Provide only --chunk-overlap with a value exceeding max_characters
        let overrides = ExtractionOverrides {
            chunk_overlap: Some(1500),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let chunking = config.chunking.unwrap();
        // overlap should be clamped to max_characters / 4
        assert_eq!(chunking.overlap, 800 / 4);
        assert_eq!(chunking.max_characters, 800);
    }

    #[test]
    fn test_chunk_overlap_valid_on_existing_config() {
        let mut config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 800,
                overlap: 100,
                ..Default::default()
            }),
            ..Default::default()
        };
        // Provide a valid overlap that is less than max_characters
        let overrides = ExtractionOverrides {
            chunk_overlap: Some(200),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let chunking = config.chunking.unwrap();
        assert_eq!(chunking.overlap, 200);
        assert_eq!(chunking.max_characters, 800);
    }

    // ── Chunking tokenizer feature validation ───────────────────────

    #[cfg(not(feature = "chunking-tokenizers"))]
    #[test]
    fn test_validate_chunking_tokenizer_requires_feature() {
        let overrides = ExtractionOverrides {
            chunking_tokenizer: Some("Xenova/gpt-4o".to_string()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("--chunking-tokenizer requires the chunking-tokenizers feature")
        );
    }

    // ── No-op when no flags provided ─────────────────────────────────

    #[test]
    fn test_no_overrides_leaves_config_unchanged() {
        let original = ExtractionConfig::default();
        let mut config = original.clone();
        let overrides = default_overrides();
        overrides.apply(&mut config);

        // Spot-check critical fields remain at defaults
        assert!(config.ocr.is_none());
        assert!(config.chunking.is_none());
        assert!(config.use_cache);
        assert!(config.enable_quality_processing);
        assert!(!config.force_ocr);
        assert!(config.language_detection.is_none());
        assert!(config.pages.is_none());
        assert!(config.images.is_none());
        assert!(config.token_reduction.is_none());
        assert!(config.email.is_none());
        assert!(config.acceleration.is_none());
        assert!(config.concurrency.is_none());
        assert!(!config.include_document_structure);
    }
}
