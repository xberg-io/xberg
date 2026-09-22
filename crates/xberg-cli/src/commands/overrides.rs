//! CLI extraction overrides via `#[derive(clap::Args)]`.
//!
//! Provides `ExtractionOverrides`, a flattened clap struct that captures all
//! optional CLI flags for extraction configuration. Call `validate()` then
//! `apply()` to layer these overrides onto an `ExtractionConfig`.

#[cfg(feature = "ocr-surface")]
use anyhow::Context as _;
use anyhow::{Result, bail};
#[cfg(any(feature = "core-cli", feature = "analysis"))]
use xberg::ChunkingConfig;
#[cfg(feature = "analysis")]
use xberg::LanguageDetectionConfig;
#[cfg(feature = "ocr-surface")]
use xberg::OcrConfig;
#[cfg(feature = "pdf-surface")]
use xberg::PdfBackend;
use xberg::{ExecutionProviderType, ExtractionConfig, LlmConfig};

use xberg::JupyterCellRendering;

use crate::ContentOutputFormatArg;

/// Which parts of a Jupyter code cell to render during extraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum JupyterCellRenderingArg {
    /// Render only the code source; omit saved outputs.
    Source,
    /// Render only the saved cell outputs; omit the code source.
    Outputs,
    /// Render both the code source and the saved outputs (default).
    Both,
}

impl From<JupyterCellRenderingArg> for JupyterCellRendering {
    fn from(arg: JupyterCellRenderingArg) -> Self {
        match arg {
            JupyterCellRenderingArg::Source => JupyterCellRendering::Source,
            JupyterCellRenderingArg::Outputs => JupyterCellRendering::Outputs,
            JupyterCellRenderingArg::Both => JupyterCellRendering::Both,
        }
    }
}

/// Accepted values for `--ocr-backend`.
#[cfg(feature = "ocr-surface")]
const VALID_OCR_BACKENDS: &[&str] = &[
    "tesseract",
    "paddle-ocr",
    "sceptre",
    "vlm",
    "candle-trocr",
    "candle-paddleocr-vl",
    "candle-glm-ocr",
    "candle-deepseek-ocr",
];

/// Language code used when neither the config nor `--ocr-language` names one.
#[cfg(feature = "ocr-surface")]
const DEFAULT_OCR_LANGUAGE: &str = "eng";

/// Language code the PaddleOCR-family backends expect instead of [`DEFAULT_OCR_LANGUAGE`].
#[cfg(feature = "ocr-surface")]
const DEFAULT_PADDLE_OCR_LANGUAGE: &str = "en";

/// Backends that use short ISO 639-1 style language codes rather than ISO 639-3.
///
/// `candle-deepseek-ocr` belongs here with the other two candle VLM backends: its
/// `supported_languages()` body is byte-identical to theirs
/// (`crates/xberg/src/candle_ocr/{deepseek,glm}_ocr_backend.rs`), accepting both the
/// ISO 639-3 and ISO 639-1 form of every language. Omitting it made the default reported
/// as `"eng"` where its two siblings report `"en"`, for no reason either backend can see:
/// none of the three consumes `config.language` for inference, so the difference was
/// purely in emitted metadata.
#[cfg(feature = "ocr-surface")]
const PADDLE_LANGUAGE_BACKENDS: &[&str] = &[
    "paddle-ocr",
    "candle-paddleocr-vl",
    "candle-glm-ocr",
    "candle-deepseek-ocr",
];

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
#[cfg(feature = "analysis")]
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

#[cfg(feature = "analysis")]
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
    /// Enable or disable OCR. When true, configures an OCR backend
    /// (default: tesseract). When false, hard-disables OCR and removes its configuration.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr: Option<bool>,

    /// OCR backend to use when --ocr is enabled (tesseract, paddle-ocr, sceptre, vlm, or candle-*).
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_backend: Option<String>,

    /// OCR language code. Tesseract uses ISO 639-3 (eng, fra, deu).
    /// PaddleOCR uses short codes (en, ch, french, korean).
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_language: Option<String>,

    /// Force OCR even if text extraction succeeds.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub force_ocr: Option<bool>,

    /// OCR pages that look like scans, keeping native text elsewhere.
    ///
    /// Detects pages that are full-page images, including scans whose hidden
    /// text layer would otherwise pass the default quality check.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_scanned_pages: bool,

    /// Minimum scan confidence (0.0-1.0) for --ocr-scanned-pages. Default: 0.7.
    ///
    /// A threshold of 0.50 or lower also OCRs born-digital slides that use a
    /// full-bleed background image.
    #[cfg(feature = "ocr-surface")]
    #[arg(long, requires = "ocr_scanned_pages")]
    pub scanned_min_confidence: Option<f64>,

    /// Disable OCR entirely (even for images)
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub disable_ocr: Option<bool>,

    /// Disable extraction result caching.
    #[arg(long)]
    pub no_cache: Option<bool>,

    /// Enable automatic image rotation before OCR based on detected orientation.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_auto_rotate: Option<bool>,

    /// Bypass the on-disk OCR result cache for this run: neither read nor write it.
    ///
    /// Unlike `--no-cache` (which controls the whole-extraction-result cache), this
    /// only affects the Tesseract OCR cache keyed by image + engine config
    /// (`TesseractConfig::use_cache`, `crates/xberg/src/ocr/processor/execution.rs`).
    /// Use it to force a fresh OCR pass while iterating on OCR settings, without
    /// clearing the cache directory by hand.
    ///
    /// Only takes effect when `ocr.tesseract_config` is already set (e.g. by a loaded
    /// config file). When it is still unset, this flag currently has no effect and logs
    /// a warning instead of running: materialising `tesseract_config` here — even just to
    /// carry `use_cache: false` — would flip it from `None` to `Some(..)`, and several
    /// call sites in `crates/xberg/src/extractors/image.rs`
    /// (`apply_default_tesseract_psm`, `is_implicit_horizontal_tesseract`,
    /// `should_retry_sparse_image_ocr`) treat `tesseract_config.is_none()` as "no explicit
    /// Tesseract config yet" and use it to decide whether to install their own PSM/element
    /// defaults (e.g. `WHOLE_IMAGE_TESSERACT_PSM = 11` for whole-page image OCR, vs.
    /// `TesseractConfig::default()`'s `psm = 3`). Materialising a default-valued
    /// `TesseractConfig` here would silently disarm all of that and change what Tesseract
    /// actually recognises, which this flag's contract forbids (issue #693).
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_no_cache: Option<bool>,

    /// JSON object of per-backend OCR options (e.g. `{"layout_mode":"whole_page"}`).
    #[cfg(feature = "ocr-surface")]
    #[arg(long, value_name = "JSON")]
    pub ocr_backend_options: Option<String>,

    /// VLM model for OCR (implies --ocr-backend vlm). Uses liter-llm routing format
    /// (e.g., "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514").
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub vlm_model: Option<String>,

    /// VLM API key for OCR
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub vlm_api_key: Option<String>,

    /// Default LLM API key shared across every LLM-backed feature
    /// (VLM OCR, structured extraction, translation, classification, captioning,
    /// summarisation, NER). Lower precedence than `--vlm-api-key` and any
    /// `api_key` set in the loaded config file, higher precedence than the
    /// `XBERG_LLM_API_KEY` environment variable.
    #[arg(long, value_name = "KEY")]
    pub api_key: Option<String>,

    /// Custom VLM OCR prompt template (Jinja2)
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub vlm_prompt: Option<String>,

    /// Enable or disable text chunking.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunk: Option<bool>,

    /// Maximum chunk size in characters.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunk_size: Option<usize>,

    /// Overlap between consecutive chunks in characters.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunk_overlap: Option<usize>,

    /// Tokenizer model for token-based chunk sizing (e.g. "Xenova/gpt-4o").
    /// Implicitly enables chunking. Requires the chunking-tokenizers feature.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunking_tokenizer: Option<String>,

    /// Content rendering format (plain, markdown, djot, html).
    /// Controls the format of extracted content.
    #[arg(long, value_enum)]
    pub content_format: Option<ContentOutputFormatArg>,

    /// Content rendering format (DEPRECATED: use --content-format instead).
    #[arg(long, value_enum, hide = true)]
    pub output_format: Option<ContentOutputFormatArg>,

    /// Include hierarchical document structure in results.
    #[arg(long)]
    pub include_structure: Option<bool>,

    /// For Jupyter notebooks: render code cells as source, outputs, or both (default: both).
    /// Cells are never executed — outputs come only from those saved in the notebook.
    #[arg(long, value_enum)]
    pub jupyter_cell_rendering: Option<JupyterCellRenderingArg>,

    /// Enable quality post-processing.
    #[cfg(feature = "analysis")]
    #[arg(long)]
    pub quality: Option<bool>,

    /// Enable language detection on extracted text.
    #[cfg(feature = "analysis")]
    #[arg(long)]
    pub detect_language: Option<bool>,

    /// Enable layout detection with default model settings (RT-DETR v2).
    /// Use `--layout` to enable or `--layout false` to explicitly disable.
    #[cfg(feature = "layout-detection")]
    #[arg(long, default_missing_value = "true", num_args = 0..=1)]
    pub layout: Option<bool>,

    /// Layout detection confidence threshold (0.0 - 1.0).
    #[cfg(feature = "layout-detection")]
    #[arg(long)]
    pub layout_confidence: Option<f32>,

    /// Which pages the layout model runs on: always (default, every page) or
    /// auto (pre-screen each page and skip the model where it cannot help).
    #[cfg(feature = "layout-detection")]
    #[arg(
        long,
        help = "Layout page selection: always (default, every page) or auto (pre-screen pages)"
    )]
    pub layout_strategy: Option<String>,

    /// Table structure model: tatr (default), slanet_wired, slanet_wireless, slanet_plus, slanet_auto, disabled.
    #[cfg(feature = "layout-detection")]
    #[arg(
        long,
        help = "Table structure model: tatr (default), slanet_wired, slanet_wireless, slanet_plus, slanet_auto, disabled"
    )]
    pub layout_table_model: Option<String>,

    /// Formula recognition model for layout-detected formula regions: latex_ocr.
    #[cfg(feature = "formula-recognition")]
    #[arg(
        long,
        help = "Formula recognition model for layout-detected formula regions: latex_ocr"
    )]
    pub layout_formula_model: Option<String>,

    /// Feed layout detection regions into the non-OCR markdown pipeline to improve
    /// heading/table/list/figure structure. Requires `--layout` to be enabled.
    /// Default: false.
    #[cfg(feature = "layout-detection")]
    #[arg(long)]
    pub use_layout_for_markdown: bool,

    /// ONNX Runtime execution provider for model inference.
    #[arg(long, value_enum)]
    pub acceleration: Option<AccelerationArg>,

    /// Maximum number of concurrent extractions in batch mode.
    #[arg(long, help = "Limit parallel extractions in batch mode")]
    pub max_concurrent: Option<usize>,

    /// Cap all internal thread pools (Rayon, ONNX intra-op, batch semaphore).
    #[arg(long, help = "Limit total threads for constrained environments")]
    pub max_threads: Option<usize>,

    /// Set concurrent Tesseract recognition sessions directly. The value is applied as given and is not capped by the
    /// thread budget. The first extraction in a process fixes it for that process.
    #[arg(
        long,
        help = "Set concurrent OCR sessions directly, instead of following the thread budget"
    )]
    pub max_concurrent_ocr: Option<usize>,

    /// Extract pages as a separate array in results.
    #[arg(long)]
    pub extract_pages: Option<bool>,

    /// Insert page marker comments into the main content string.
    #[arg(long)]
    pub page_markers: Option<bool>,

    /// Enable image extraction from documents.
    #[arg(long)]
    pub extract_images: Option<bool>,

    /// Target DPI for image normalisation (e.g. 150, 300, 600).
    #[arg(long)]
    pub target_dpi: Option<i32>,

    /// Password(s) for encrypted PDFs. Can be specified multiple times.
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_password: Vec<String>,

    /// Extract images embedded in PDF pages.
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_extract_images: Option<bool>,

    /// Extract tables from PDF (native engine grid + heuristic text-layer fallback).
    /// Default: true.
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_extract_tables: Option<bool>,

    /// OCR extracted inline images and inject results into the document.
    #[cfg(all(feature = "pdf-surface", feature = "ocr-surface"))]
    #[arg(long)]
    pub pdf_ocr_inline_images: Option<bool>,

    /// Extract PDF metadata (title, author, etc.).
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_extract_metadata: Option<bool>,

    /// PDF extraction backend to use: "native" (default) or "pdfium".
    ///
    /// "pdfium" requires the CLI to be built with the `pdf-pdfium-surface`
    /// feature, and requires the pdfium shared library to be loadable at run
    /// time (system library search path, or `PDFIUM_DYNAMIC_LIB_PATH`).
    /// Selecting it on a build without that feature is rejected with an error
    /// rather than silently falling back to native.
    ///
    /// The pdfium engine is deliberately narrower than native: page count,
    /// per-page plain text, and Info-dictionary metadata only -- no tables,
    /// layout detection, annotations, form fields, embedded files, or OCR
    /// fallback. It is not a drop-in replacement for the native backend.
    #[cfg(feature = "pdf-surface")]
    #[arg(long, value_name = "BACKEND")]
    pub pdf_backend: Option<String>,

    /// Token reduction level (off, light, moderate, aggressive, maximum).
    #[cfg(feature = "analysis")]
    #[arg(long, value_enum)]
    pub token_reduction: Option<ReductionLevelArg>,

    /// Windows codepage fallback for MSG files without codepage metadata.
    /// Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western).
    #[arg(long)]
    pub msg_codepage: Option<u32>,

    /// Cache namespace for tenant isolation.
    #[arg(long)]
    pub cache_namespace: Option<String>,

    /// Per-request cache TTL in seconds (0 = skip cache).
    #[arg(long)]
    pub cache_ttl_secs: Option<u64>,

    /// Built-in colour theme for styled HTML output (default, github, dark, light, unstyled).
    /// Implies --content-format html and enables the styled HTML renderer.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "THEME")]
    pub html_theme: Option<String>,

    /// Inline CSS string appended after the theme stylesheet in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "CSS")]
    pub html_css: Option<String>,

    /// Path to a CSS file loaded once and appended after the theme stylesheet in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "PATH")]
    pub html_css_file: Option<std::path::PathBuf>,

    /// CSS class prefix used on every emitted class name (default: "kb-").
    #[cfg(feature = "html")]
    #[arg(long, value_name = "PREFIX")]
    pub html_class_prefix: Option<String>,

    /// Suppress the embedded `<style>` block in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long)]
    pub html_no_embed_css: bool,

    /// CSV/TSV field delimiter (single ASCII character, e.g. ";", "|", "\t").
    /// When unset, the delimiter is auto-detected from the file.
    #[arg(long, value_name = "CHAR")]
    pub csv_delimiter: Option<String>,

    /// Line prefix marking a CSV/TSV comment line to skip entirely (e.g. "#").
    /// Can be specified multiple times. Default: no comment filtering.
    #[arg(long, value_name = "PREFIX")]
    pub csv_comment_prefix: Vec<String>,
}

impl ExtractionOverrides {
    /// Validate flag combinations before applying.
    ///
    /// Call this before `apply()` to surface user-friendly errors for
    /// invalid or contradictory options.
    pub fn validate(&self) -> Result<()> {
        #[cfg(any(feature = "core-cli", feature = "analysis"))]
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

        #[cfg(any(feature = "core-cli", feature = "analysis"))]
        if let Some(overlap) = self.chunk_overlap
            && let Some(size) = self.chunk_size
            && overlap >= size
        {
            bail!("Invalid chunk overlap: {overlap}. Overlap ({overlap}) must be less than chunk size ({size}).");
        }

        if let Some(dpi) = self.target_dpi
            && (!(36..=2400).contains(&dpi))
        {
            bail!("Invalid target DPI: {dpi}. Value must be between 36 and 2400.");
        }

        #[cfg(feature = "layout-detection")]
        {
            if let Some(conf) = self.layout_confidence
                && !(0.0..=1.0).contains(&conf)
            {
                bail!("Invalid layout confidence: {conf}. Value must be between 0.0 and 1.0.");
            }
            if self.layout == Some(false) && (self.layout_confidence.is_some() || self.layout_table_model.is_some()) {
                bail!("--layout false cannot be combined with --layout-confidence or --layout-table-model");
            }
            if self.layout == Some(false) && self.layout_strategy.is_some() {
                bail!("--layout false cannot be combined with --layout-strategy");
            }
            if let Some(ref strategy) = self.layout_strategy
                && strategy.parse::<xberg::LayoutStrategy>().is_err()
            {
                bail!("Invalid layout strategy: '{strategy}'. Valid: always, auto.");
            }
        }

        #[cfg(feature = "ocr-surface")]
        {
            if self.ocr == Some(false) && self.ocr_scanned_pages {
                bail!("--ocr false cannot be combined with --ocr-scanned-pages");
            }
            if self.ocr == Some(false) && self.force_ocr == Some(true) {
                bail!("--ocr false cannot be combined with --force-ocr true");
            }
            if let (Some(ocr), Some(disable_ocr)) = (self.ocr, self.disable_ocr)
                && ocr == disable_ocr
            {
                bail!("--ocr and --disable-ocr specify contradictory values");
            }
            if self.ocr_scanned_pages && self.disable_ocr == Some(true) {
                bail!("--ocr-scanned-pages cannot be combined with --disable-ocr");
            }
            if let Some(confidence) = self.scanned_min_confidence
                && !(0.0..=1.0).contains(&confidence)
            {
                bail!("Invalid scan confidence: {confidence}. Value must be between 0.0 and 1.0.");
            }
            if self.force_ocr == Some(true) && self.disable_ocr == Some(true) {
                bail!("--force-ocr and --disable-ocr cannot both be true");
            }

            if let Some(ref backend) = self.ocr_backend
                && !VALID_OCR_BACKENDS.contains(&backend.as_str())
            {
                bail!(
                    "Invalid OCR backend '{}'. Valid backends: {}",
                    backend,
                    VALID_OCR_BACKENDS.join(", ")
                );
            }

            self.parsed_backend_options()?;

            if self.vlm_api_key.is_some() && self.vlm_model.is_none() {
                bail!("--vlm-api-key requires --vlm-model to be specified");
            }
            if self.vlm_prompt.is_some() && self.vlm_model.is_none() {
                bail!("--vlm-prompt requires --vlm-model to be specified");
            }
            if self.ocr_backend.as_deref() == Some("vlm") && self.vlm_model.is_none() {
                bail!("--ocr-backend vlm requires --vlm-model to be specified");
            }
        }

        #[cfg(all(
            any(feature = "core-cli", feature = "analysis"),
            not(feature = "chunking-tokenizers")
        ))]
        if self.chunking_tokenizer.is_some() {
            bail!(
                "--chunking-tokenizer requires the chunking-tokenizers feature. \
                 Rebuild with --features chunking-tokenizers"
            );
        }

        if let Some(0) = self.max_concurrent {
            bail!("--max-concurrent must be at least 1");
        }
        if let Some(0) = self.max_threads {
            bail!("--max-threads must be at least 1");
        }
        if let Some(0) = self.max_concurrent_ocr {
            bail!("--max-concurrent-ocr must be at least 1");
        }

        #[cfg(feature = "pdf-surface")]
        if let Some(ref backend) = self.pdf_backend {
            match backend.parse::<PdfBackend>() {
                Ok(PdfBackend::Native) => {}
                #[cfg(feature = "pdf-pdfium-surface")]
                Ok(PdfBackend::Pdfium) => {}
                #[cfg(not(feature = "pdf-pdfium-surface"))]
                Ok(PdfBackend::Pdfium) => {
                    bail!(
                        "--pdf-backend pdfium requires the pdf-pdfium-surface feature, which this \
                         binary was not built with. Rebuild with --features pdf-pdfium-surface. \
                         Note that the pdfium engine also loads the pdfium shared library at run \
                         time: install it on the system library search path, or point \
                         PDFIUM_DYNAMIC_LIB_PATH at a directory containing it."
                    );
                }
                Err(_) => {
                    bail!("Invalid PDF backend '{}'. Valid values: native, pdfium.", backend);
                }
            }
        }

        if let Some(ref delimiter) = self.csv_delimiter
            && !(delimiter.len() == 1 && delimiter.is_ascii())
        {
            bail!(
                "Invalid CSV delimiter '{}'. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|').",
                delimiter
            );
        }

        Ok(())
    }

    /// Apply these overrides onto an existing `ExtractionConfig`.
    ///
    /// Only fields that were explicitly provided on the command line take
    /// effect; everything else is left untouched.
    pub fn apply(self, config: &mut ExtractionConfig) {
        let resolved_api_key = resolve_llm_api_key(self.api_key.as_deref());
        #[cfg(feature = "ocr-surface")]
        self.apply_ocr(config);
        #[cfg(feature = "ocr-surface")]
        self.apply_vlm_ocr(config);
        #[cfg(any(feature = "core-cli", feature = "analysis"))]
        self.apply_chunking(config);
        #[cfg(feature = "analysis")]
        self.apply_quality_and_detection(config);
        self.apply_output_format(config);
        self.apply_include_structure(config);
        self.apply_jupyter_cell_rendering(config);
        self.apply_layout(config);
        self.apply_acceleration(config);
        self.apply_concurrency(config);
        self.apply_pages(config);
        self.apply_images(config);
        #[cfg(feature = "pdf-surface")]
        self.apply_pdf(config);
        #[cfg(feature = "analysis")]
        self.apply_token_reduction(config);
        self.apply_email(config);
        self.apply_cache(config);
        self.apply_html_styled(config);
        self.apply_csv(config);
        if let Some(key) = resolved_api_key {
            apply_llm_api_key(config, &key);
        }
        // Last: every prior `apply_*` that can touch `config.layout` (`apply_layout`) or
        // `config.output_format` (`apply_output_format`, `apply_html_styled`) has already run,
        // so this observes the fully resolved combination rather than an intermediate one.
        #[cfg(feature = "layout-detection")]
        self.warn_layout_wastes_plain_output(config);
    }

    /// Warn when the final resolved configuration enables layout detection while
    /// `output_format` stays `Plain` (contract point 4 of the OCR/layout structure
    /// contract — see `xberg::core::config_validation::layout_wastes_plain_output`).
    ///
    /// This is a warning, not a validation error and not a coercion: `Plain` remains the
    /// default output format and layout remains off by default. A caller who set both
    /// deliberately still gets exactly what they asked for; they are only told that the
    /// layout pass (20s-202s depending on backend, per the WP-E measurements) will run and
    /// its output will be discarded, since no renderer at `Plain` consumes structure.
    #[cfg(feature = "layout-detection")]
    fn warn_layout_wastes_plain_output(&self, config: &ExtractionConfig) {
        if xberg::core::config_validation::layout_wastes_plain_output(config.layout.is_some(), &config.output_format) {
            tracing::warn!(
                "layout detection is enabled but the output format is 'plain'; the layout pass \
                 will run and the headings/lists/tables it detects will be discarded because \
                 plain output never renders structure. Pass --content-format markdown (or \
                 html/djot/json) to use the detected structure, or omit --layout to skip the \
                 extra work."
            );
        }
    }

    /// Apply the `--ocr*` flags onto `config.ocr`.
    ///
    /// Each flag mutates only the field it names. Fields with no CLI flag
    /// (`quality_thresholds`, `pipeline`, `tesseract_config`, `vlm_config`, …)
    /// keep whatever the config file or `--config-json` set for them.
    ///
    /// Naming any `--ocr-*` field flag (`--ocr-backend`, `--ocr-backend-options`,
    /// `--ocr-auto-rotate`, `--ocr-language`) is on its own enough to materialise
    /// `config.ocr` when it is still `None`, matching the `has_*_flag` pattern used
    /// by the other `apply_*` methods below. This matters because the flags that
    /// make OCR actually run (`--force-ocr`, `--ocr-scanned-pages`) do not require
    /// `config.ocr` to exist first: without this, a named backend/option/language
    /// was silently discarded while OCR still ran, using whatever backend the
    /// default config carries instead of the one requested.
    #[cfg(feature = "ocr-surface")]
    fn apply_ocr(&self, config: &mut ExtractionConfig) {
        if self.ocr == Some(false) {
            config.ocr = None;
            config.disable_ocr = true;
            config.force_ocr = false;
            config.ocr_strategy = xberg::OcrStrategy::Auto;
            config.force_ocr_pages = None;
        } else {
            if self.ocr == Some(true) {
                config.ocr.get_or_insert_with(OcrConfig::default).enabled = true;
                config.disable_ocr = false;
            } else if self.has_ocr_field_flag() {
                config.ocr.get_or_insert_with(OcrConfig::default);
            }
            if let Some(ocr) = config.ocr.as_mut() {
                self.apply_ocr_fields(ocr);
            }
        }

        if self.ocr != Some(false)
            && let Some(force_ocr_flag) = self.force_ocr
        {
            config.force_ocr = force_ocr_flag;
        }
        if self.ocr.is_none()
            && let Some(disable_ocr_flag) = self.disable_ocr
        {
            config.disable_ocr = disable_ocr_flag;
        }
        if self.ocr != Some(false) && self.ocr_scanned_pages {
            config.ocr_strategy = xberg::OcrStrategy::ScannedPages {
                min_confidence: self
                    .scanned_min_confidence
                    .unwrap_or(xberg::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE),
            };
            // The mixed OCR route (`extract_mixed_ocr_native`) needs per-page byte boundaries
            // to locate a detected scan within the native text and splice OCR output back in.
            // Boundaries are only tracked by the PDF backend when `config.pages` is
            // materialised (`pdf::native::text::extract_text_from_native_document`'s
            // `page_config` branch); without `--extract-pages`, `config.pages` stayed `None`,
            // so this route always hit its "no page boundaries available" fallback and
            // returned the native text untouched -- empty, for a scan, i.e. a zero-byte,
            // exit-0 result (#656). `PageConfig::default()` keeps `extract_pages: false`, so
            // this turns boundary tracking on without adding the `pages` array to the result
            // unless the caller separately asked for it.
            config.pages.get_or_insert_with(Default::default);
        }
    }

    /// Whether any `--ocr-*` field flag (backend, backend options, auto-rotate,
    /// language) was given, independent of `--ocr`/`--ocr true`.
    ///
    /// Deliberately excludes `--ocr-no-cache`: `apply_ocr_no_cache` only ever mutates an
    /// already-materialised `tesseract_config` (see its doc comment), so naming
    /// `--ocr-no-cache` alone has nothing to do once `config.ocr` exists, and must not be
    /// the thing that materialises `config.ocr` in the first place. Doing so would flip
    /// `config.ocr` from `None` to `Some(OcrConfig::default())` purely as a side effect of
    /// a caching flag, which can itself change behaviour downstream (e.g.
    /// `should_use_layout_ocr` in `crates/xberg/src/extractors/image.rs` branches on
    /// `config.ocr.is_some()`).
    #[cfg(feature = "ocr-surface")]
    fn has_ocr_field_flag(&self) -> bool {
        self.ocr_backend.is_some()
            || self.ocr_backend_options.is_some()
            || self.ocr_auto_rotate.is_some()
            || self.ocr_language.is_some()
    }

    /// Mutate the individual OCR fields that have a CLI flag, in place.
    ///
    /// Every assignment is guarded by the presence of its own flag, so sibling
    /// fields on `ocr` survive untouched.
    #[cfg(feature = "ocr-surface")]
    fn apply_ocr_fields(&self, ocr: &mut OcrConfig) {
        if let Some(ref backend) = self.ocr_backend {
            ocr.backend = backend.clone();
        }
        if let Some(options) = self.parsed_backend_options().ok().flatten() {
            ocr.backend_options = Some(options);
        }
        if let Some(rotate) = self.ocr_auto_rotate {
            ocr.auto_rotate = rotate;
        }
        if let Some(no_cache) = self.ocr_no_cache {
            apply_ocr_no_cache(ocr, no_cache);
        }

        if let Some(ref language) = self.ocr_language {
            set_ocr_language(ocr, vec![language.clone()]);
            return;
        }

        // No `--ocr-language`. When the caller selected a backend (via `--ocr true` or
        // `--ocr-backend`) and the config still carries the untouched default language,
        // substitute the code that backend expects. An explicitly configured language is
        // never rewritten, and a run with no OCR flags at all is a no-op.
        let backend_selected = self.ocr == Some(true) || self.ocr_backend.is_some();
        let backend_default = default_language_for_backend(&ocr.backend);
        if backend_selected && backend_default != DEFAULT_OCR_LANGUAGE && is_default_ocr_language(&ocr.language) {
            ocr.language = vec![backend_default.to_string()];
        }
    }

    #[cfg(feature = "ocr-surface")]
    fn apply_vlm_ocr(&self, config: &mut ExtractionConfig) {
        if let Some(ref vlm_model) = self.vlm_model {
            let vlm_llm_config = LlmConfig {
                model: vlm_model.clone(),
                api_key: self.vlm_api_key.clone(),
                ..Default::default()
            };

            let backend_options = self.parsed_backend_options().ok().flatten();
            let ocr = config.ocr.get_or_insert_with(|| OcrConfig {
                enabled: true,
                backend: "vlm".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
                vlm_config: None,
                vlm_fallback: Default::default(),
                vlm_prompt: None,
                acceleration: None,
                security_limits: None,
                tessdata_bytes: None,
                tessdata_path: None,
                backend_options,
            });

            ocr.backend = "vlm".to_string();
            ocr.vlm_config = Some(vlm_llm_config);

            if let Some(ref prompt) = self.vlm_prompt {
                ocr.vlm_prompt = Some(prompt.clone());
            }
        }
    }

    /// Parse `--ocr-backend-options` into a `serde_json::Value`, enforcing that it is a JSON object.
    #[cfg(feature = "ocr-surface")]
    fn parsed_backend_options(&self) -> Result<Option<serde_json::Value>> {
        let Some(ref s) = self.ocr_backend_options else {
            return Ok(None);
        };
        let value: serde_json::Value =
            serde_json::from_str(s).with_context(|| format!("invalid --ocr-backend-options JSON: {s}"))?;
        if !value.is_object() {
            bail!("--ocr-backend-options must be a JSON object");
        }
        Ok(Some(value))
    }

    /// Apply the `--chunk*` flags onto `config.chunking`.
    ///
    /// Naming any `--chunk-*` field flag (`--chunk-size` or `--chunk-overlap`)
    /// is on its own enough to materialise `config.chunking`
    /// when it is still `None`, matching the `has_*_flag` idiom `apply_ocr` uses for
    /// `--ocr-*` field flags (fixed for OCR in `5921a7cc23`). Before this, a field flag given
    /// without `--chunk true`, `--chunking-tokenizer`, or a config file that already set
    /// `chunking` was silently dropped: `config.chunking` stayed `None`, so the early return
    /// below skipped every field assignment with no warning and no error.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    fn apply_chunking(&self, config: &mut ExtractionConfig) {
        let chunk = if self.chunking_tokenizer.is_some() && self.chunk.is_none() {
            Some(true)
        } else {
            self.chunk
        };

        if chunk == Some(false) {
            config.chunking = None;
            return;
        }
        if (chunk == Some(true) || self.has_chunk_field_flag()) && config.chunking.is_none() {
            config.chunking = Some(ChunkingConfig::default());
        }

        let Some(chunking) = config.chunking.as_mut() else {
            return;
        };

        if let Some(max_characters) = self.chunk_size {
            chunking.max_characters = max_characters;
        }
        if let Some(overlap) = self.chunk_overlap {
            chunking.overlap = overlap;
        }

        if chunking.overlap >= chunking.max_characters {
            chunking.overlap = chunking.max_characters / 4;
        }

        #[cfg(feature = "chunking-tokenizers")]
        if let Some(ref model) = self.chunking_tokenizer {
            chunking.sizing = xberg::ChunkSizing::Tokenizer {
                model: model.clone(),
                cache_dir: None,
            };
        }
    }

    /// Whether any `--chunk-*` field flag (size or overlap) was given,
    /// independent of `--chunk`/`--chunk true` and `--chunking-tokenizer`.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    fn has_chunk_field_flag(&self) -> bool {
        self.chunk_size.is_some() || self.chunk_overlap.is_some()
    }

    #[cfg(feature = "analysis")]
    fn apply_quality_and_detection(&self, config: &mut ExtractionConfig) {
        if let Some(quality_flag) = self.quality {
            config.enable_quality_processing = quality_flag;
        }
        if let Some(detect_language_flag) = self.detect_language {
            if detect_language_flag {
                config
                    .language_detection
                    .get_or_insert_with(LanguageDetectionConfig::default)
                    .enabled = true;
            } else {
                config.language_detection = None;
            }
        }
    }

    fn apply_output_format(&self, config: &mut ExtractionConfig) {
        let final_format = self.content_format.or_else(|| {
            if self.output_format.is_some() {
                tracing::warn!("'--output-format' is deprecated, use '--content-format' instead");
            }
            self.output_format
        });

        if let Some(content_fmt) = final_format {
            config.output_format = content_fmt.into();
        }
    }

    fn apply_include_structure(&self, config: &mut ExtractionConfig) {
        if let Some(flag) = self.include_structure {
            config.include_document_structure = flag;
        }
    }

    fn apply_jupyter_cell_rendering(&self, config: &mut ExtractionConfig) {
        if let Some(rendering) = self.jupyter_cell_rendering {
            config.jupyter_cell_rendering = rendering.into();
        }
    }

    #[allow(unused_variables)]
    fn apply_layout(&self, config: &mut ExtractionConfig) {
        #[cfg(feature = "layout-detection")]
        {
            if self.layout == Some(false) {
                config.layout = None;
                return;
            }

            #[cfg(feature = "formula-recognition")]
            let has_formula_flag = self.layout_formula_model.is_some();
            #[cfg(not(feature = "formula-recognition"))]
            let has_formula_flag = false;
            let has_layout_flag = self.layout == Some(true)
                || self.layout_confidence.is_some()
                || self.layout_table_model.is_some()
                || self.layout_strategy.is_some()
                || self.use_layout_for_markdown
                || has_formula_flag;
            if has_layout_flag {
                let mut layout = config.layout.clone().unwrap_or_default();
                if let Some(confidence) = self.layout_confidence {
                    layout.confidence_threshold = Some(confidence);
                }
                if let Some(ref table_model) = self.layout_table_model {
                    layout.table_model = table_model.parse().unwrap_or_default();
                }
                if let Some(ref strategy) = self.layout_strategy {
                    layout.strategy = strategy.parse().unwrap_or_default();
                }
                #[cfg(feature = "formula-recognition")]
                if let Some(ref formula_model) = self.layout_formula_model {
                    match formula_model.parse() {
                        Ok(model) => layout.formula_model = Some(model),
                        Err(error) => tracing::warn!("{error}; ignoring --layout-formula-model"),
                    }
                }
                config.layout = Some(layout);
            }
            if self.use_layout_for_markdown {
                config.use_layout_for_markdown = true;
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
        if let Some(max_concurrent_ocr) = self.max_concurrent_ocr {
            let concurrency = config.concurrency.get_or_insert_with(Default::default);
            concurrency.max_concurrent_ocr = Some(max_concurrent_ocr);
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

    #[cfg(feature = "pdf-surface")]
    fn apply_pdf(&self, config: &mut ExtractionConfig) {
        let has_pdf_flag = self.pdf_extract_images.is_some()
            || self.pdf_extract_tables.is_some()
            || self.pdf_extract_metadata.is_some()
            || !self.pdf_password.is_empty()
            || self.pdf_backend.is_some();
        #[cfg(feature = "ocr-surface")]
        let has_pdf_flag = has_pdf_flag || self.pdf_ocr_inline_images.is_some();
        if has_pdf_flag {
            let pdf_opts = config.pdf_options.get_or_insert_with(Default::default);
            if let Some(extract_img) = self.pdf_extract_images {
                pdf_opts.extract_images = extract_img;
            }
            if let Some(extract_tables) = self.pdf_extract_tables {
                pdf_opts.extract_tables = extract_tables;
            }
            #[cfg(feature = "ocr-surface")]
            if let Some(ocr_img) = self.pdf_ocr_inline_images {
                pdf_opts.ocr_inline_images = ocr_img;
            }
            if let Some(extract_meta) = self.pdf_extract_metadata {
                pdf_opts.extract_metadata = extract_meta;
            }
            if !self.pdf_password.is_empty() {
                pdf_opts.passwords = Some(self.pdf_password.clone());
            }
            // `validate()` runs before `apply()` (see main.rs) and already rejected any
            // value that fails to parse, so `unwrap_or_default()` here mirrors the
            // established --layout-strategy / --layout-table-model pattern: it is
            // unreachable in practice, never a silent behavior change.
            if let Some(ref backend) = self.pdf_backend {
                pdf_opts.backend = backend.parse().unwrap_or_default();
            }
        }
    }

    #[cfg(feature = "analysis")]
    fn apply_token_reduction(&self, config: &mut ExtractionConfig) {
        if let Some(level) = self.token_reduction {
            config
                .token_reduction
                .get_or_insert_with(xberg::TokenReductionOptions::default)
                .mode = level.as_mode_str().to_string();
        }
    }

    fn apply_email(&self, config: &mut ExtractionConfig) {
        if let Some(codepage) = self.msg_codepage {
            let email = config.email.get_or_insert_with(Default::default);
            email.msg_fallback_codepage = Some(codepage);
        }
    }

    fn apply_cache(&self, config: &mut ExtractionConfig) {
        if let Some(no_cache_flag) = self.no_cache {
            config.use_cache = !no_cache_flag;
        }
        if let Some(ns) = &self.cache_namespace {
            config.cache_namespace = Some(ns.clone());
        }
        if let Some(ttl) = self.cache_ttl_secs {
            config.cache_ttl_secs = Some(ttl);
        }
    }

    #[allow(unused_variables)]
    fn apply_html_styled(&self, config: &mut ExtractionConfig) {
        #[cfg(feature = "html")]
        {
            let has_flag = self.html_theme.is_some()
                || self.html_css.is_some()
                || self.html_css_file.is_some()
                || self.html_class_prefix.is_some()
                || self.html_no_embed_css;

            if has_flag {
                config.output_format = xberg::OutputFormat::Html;

                let mut html_cfg = config.html_output.clone().unwrap_or_default();

                if let Some(ref theme_str) = self.html_theme {
                    html_cfg.theme = match theme_str.to_lowercase().as_str() {
                        "github" => xberg::HtmlTheme::GitHub,
                        "dark" => xberg::HtmlTheme::Dark,
                        "light" => xberg::HtmlTheme::Light,
                        "unstyled" => xberg::HtmlTheme::Unstyled,
                        _ => xberg::HtmlTheme::Default,
                    };
                }

                if let Some(ref css) = self.html_css {
                    html_cfg.css = Some(css.clone());
                }

                if let Some(ref path) = self.html_css_file {
                    html_cfg.css_file = Some(path.clone());
                }

                if let Some(ref prefix) = self.html_class_prefix {
                    html_cfg.class_prefix = prefix.clone();
                }

                if self.html_no_embed_css {
                    html_cfg.embed_css = false;
                }

                config.html_output = Some(html_cfg);
            }
        }
    }

    fn apply_csv(&self, config: &mut ExtractionConfig) {
        let has_flag = self.csv_delimiter.is_some() || !self.csv_comment_prefix.is_empty();
        if has_flag {
            let mut csv_cfg = config.csv.clone().unwrap_or_default();
            if let Some(ref delimiter) = self.csv_delimiter {
                csv_cfg.delimiter = Some(delimiter.clone());
            }
            if !self.csv_comment_prefix.is_empty() {
                csv_cfg.comment_prefixes = self.csv_comment_prefix.clone();
            }
            config.csv = Some(csv_cfg);
        }
    }
}

/// The default OCR language code for `backend`.
#[cfg(feature = "ocr-surface")]
fn default_language_for_backend(backend: &str) -> &'static str {
    if PADDLE_LANGUAGE_BACKENDS.contains(&backend) {
        DEFAULT_PADDLE_OCR_LANGUAGE
    } else {
        DEFAULT_OCR_LANGUAGE
    }
}

/// Whether `language` is still the untouched compiled-in default.
#[cfg(feature = "ocr-surface")]
fn is_default_ocr_language(language: &[String]) -> bool {
    matches!(language, [only] if only == DEFAULT_OCR_LANGUAGE)
}

/// Force the Tesseract OCR result cache on or off for this run, without perturbing any
/// other Tesseract setting.
///
/// `TesseractConfig::use_cache` (`xberg::TesseractConfig`) already exists and already
/// gates the OCR cache end to end (`process_image_resolved` in
/// `crates/xberg/src/ocr/processor/execution.rs`) — the CLI simply had no flag wired
/// to it before `--ocr-no-cache`.
///
/// This only mutates an *already-materialised* `tesseract_config` (e.g. one set by a
/// loaded config file). It deliberately does **not** call `get_or_insert_with` to create
/// one when `ocr.tesseract_config` is still `None` (as a first version of this function
/// did — see #693): `tesseract_config.is_none()` is a load-bearing sentinel downstream.
/// `crates/xberg/src/extractors/image.rs::apply_default_tesseract_psm` (and its siblings
/// `is_implicit_horizontal_tesseract`, `should_retry_sparse_image_ocr`) only install their
/// own whole-image/vertical/sparse-retry PSM and element defaults when `tesseract_config`
/// is still `None` by the time OCR runs; once it is `Some(..)`, those call sites treat it
/// as "the caller already made an explicit choice" and skip their own defaulting,
/// leaving `TesseractConfig::default()`'s `psm = 3` in place instead of e.g.
/// `WHOLE_IMAGE_TESSERACT_PSM = 11`. Measured effect on a real scan: 217 recognised words
/// without the flag vs. 194 with it, from a changed PSM alone — a caching flag must never
/// change what Tesseract recognises.
///
/// Closing this properly for the "no `tesseract_config` yet" case needs a cache-bypass
/// signal that lives outside `TesseractConfig`'s `Option` sentinel — for example a new
/// `OcrConfig::bypass_ocr_cache: bool` field (in `crates/xberg/src/core/config/ocr.rs`)
/// that `TesseractBackend::config_to_tesseract`
/// (`crates/xberg/src/ocr/tesseract_backend.rs`) ORs into the internal
/// `TesseractConfig::use_cache` it builds, independent of whether `tesseract_config` was
/// supplied. That change is outside this file's scope; until it lands, this flag is a
/// no-op (with a warning) unless `tesseract_config` is already set.
#[cfg(feature = "ocr-surface")]
fn apply_ocr_no_cache(ocr: &mut OcrConfig, no_cache: bool) {
    let Some(tesseract_config) = ocr.tesseract_config.as_mut() else {
        tracing::warn!(
            "--ocr-no-cache has no effect: no `tesseract_config` is set yet (e.g. via a \
             config file's `ocr.tesseract_config`). Materialising one just to carry \
             `use_cache: false` would also silently change Tesseract's PSM and other \
             defaults for this run (see issue #693), so this flag is a no-op here instead \
             of risking that. Clear the on-disk OCR cache directory instead, or set \
             `ocr.tesseract_config` explicitly before using --ocr-no-cache."
        );
        return;
    };
    tesseract_config.use_cache = !no_cache;
}

/// Set `language` on an OCR config and on every nested Tesseract-flavoured
/// config that carries its own copy of it.
#[cfg(feature = "ocr-surface")]
fn set_ocr_language(ocr: &mut OcrConfig, language: Vec<String>) {
    ocr.language = language.clone();
    if let Some(tesseract_config) = ocr.tesseract_config.as_mut() {
        tesseract_config.language = language.clone();
    }
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        for stage in &mut pipeline.stages {
            if stage.backend != "tesseract" {
                continue;
            }
            stage.language = Some(language.clone());
            if let Some(tesseract_config) = stage.tesseract_config.as_mut() {
                tesseract_config.language = language.clone();
            }
        }
    }
}

/// Resolve the LLM API key the CLI should propagate to every `LlmConfig` slot.
///
/// Precedence (highest first):
/// 1. The `--api-key` CLI flag (`cli_api_key`).
/// 2. The `XBERG_LLM_API_KEY` environment variable.
/// 3. `None` — keep whatever the loaded config / inline JSON / overrides set.
///
/// Returns `None` when neither the CLI flag nor the environment variable
/// supplies a non-empty value. In that case [`apply_llm_api_key`] is not
/// called and liter-llm's per-provider env-var fallback runs at request time.
///
/// The resolved source is logged at `info!` level; the key value itself is
/// never logged.
pub(crate) fn resolve_llm_api_key(cli_api_key: Option<&str>) -> Option<String> {
    if let Some(key) = cli_api_key.map(str::trim).filter(|s| !s.is_empty()) {
        tracing::info!(source = "cli_flag", "Resolved LLM API key from --api-key flag");
        return Some(key.to_string());
    }
    if let Ok(value) = std::env::var("XBERG_LLM_API_KEY")
        && !value.is_empty()
    {
        tracing::info!(source = "xberg_env", "Resolved LLM API key from XBERG_LLM_API_KEY");
        return Some(value);
    }
    None
}

/// Write `key` into every [`LlmConfig`] field of `config` whose `api_key` is
/// `None`. Existing non-`None` values (from the loaded config file or inline
/// JSON) take precedence over the resolved key — the CLI never silently
/// overrides explicit configuration.
pub(crate) fn apply_llm_api_key(config: &mut ExtractionConfig, key: &str) {
    fn fill(slot: &mut LlmConfig, key: &str) {
        if slot.api_key.is_none() {
            slot.api_key = Some(key.to_string());
        }
    }

    if let Some(ocr) = config.ocr.as_mut()
        && let Some(vlm) = ocr.vlm_config.as_mut()
    {
        fill(vlm, key);
    }

    if let Some(ext) = config.structured_extraction.as_mut() {
        fill(&mut ext.llm, key);
    }

    if let Some(chunking) = config.chunking.as_mut()
        && let Some(embedding) = chunking.embedding.as_mut()
        && let xberg::EmbeddingModelType::Llm { llm } = &mut embedding.model
    {
        fill(llm, key);
    }

    if let Some(translation) = config.translation.as_mut() {
        fill(&mut translation.llm, key);
    }

    if let Some(pc) = config.page_classification.as_mut() {
        fill(&mut pc.llm, key);
    }

    if let Some(cap) = config.captioning.as_mut() {
        fill(&mut cap.llm, key);
    }

    if let Some(sum) = config.summarization.as_mut()
        && let Some(llm) = sum.llm.as_mut()
    {
        fill(llm, key);
    }

    if let Some(ner) = config.ner.as_mut()
        && let Some(llm) = ner.llm.as_mut()
    {
        fill(llm, key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xberg::ExtractionConfig;

    fn default_overrides() -> ExtractionOverrides {
        ExtractionOverrides::default()
    }

    #[cfg(feature = "ocr-surface")]
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
        assert_eq!(ocr.language, vec!["eng".to_string()]);
    }

    #[cfg(feature = "ocr-surface")]
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
        assert_eq!(ocr.language, vec!["en".to_string()]);
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_default_language_sceptre() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("sceptre".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.expect("OCR config should be set");
        assert_eq!(ocr.backend, "sceptre");
        assert_eq!(ocr.language, vec!["eng".to_string()]);
    }

    /// Regression test for the empirically observed CLI defect: running
    /// `xberg extract doc.pdf --ocr-scanned-pages --ocr-backend sceptre` (no
    /// `--ocr true`) silently ran tesseract instead of sceptre, because
    /// `apply_ocr` only materialised `config.ocr` on `--ocr true`, so
    /// `apply_ocr_fields` — which assigns `backend` — never ran, while
    /// `--ocr-scanned-pages` set `config.ocr_strategy` unconditionally and
    /// triggered OCR to run anyway with the default backend. Before the fix
    /// (i.e. reverting `has_ocr_field_flag` back to just `self.ocr ==
    /// Some(true)`), `config.ocr` stays `None` here — `ocr.backend` is never
    /// even reachable — so this assertion fails without the fix.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_flag_selects_backend_without_ocr_true_flag() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr_backend: Some("sceptre".to_string()),
            ocr_scanned_pages: true,
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config
            .ocr
            .expect("--ocr-backend must materialise an OCR config even without --ocr true");
        assert_eq!(ocr.backend, "sceptre");
        assert_eq!(
            config.ocr_strategy,
            xberg::OcrStrategy::ScannedPages {
                min_confidence: xberg::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE
            }
        );
    }

    /// Regression test for #656: `--ocr-scanned-pages` alone (no `--extract-pages`, no
    /// `--ocr true`, no other `--ocr-*` field flag) previously left `config.pages` as `None`.
    /// The PDF backend only tracks per-page byte boundaries when `config.pages` is `Some`
    /// (`pdf::native::text::extract_text_from_native_document`'s `page_config` branch), and the
    /// mixed OCR route needs those boundaries to splice OCR text back into the native text
    /// (`extractors/pdf/mod.rs`). Without them, that route always fell back to "no page
    /// boundaries available; using native text" -- an empty result for a scan, i.e. a
    /// zero-byte, exit-0 extraction. Before the fix (i.e. removing the
    /// `config.pages.get_or_insert_with(Default::default)` call), `config.pages` stays `None`
    /// here and this assertion fails.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_scanned_pages_alone_materialises_page_boundary_tracking() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr_scanned_pages: true,
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let pages = config
            .pages
            .expect("--ocr-scanned-pages must materialise page-boundary tracking on its own");
        assert!(
            !pages.extract_pages,
            "boundary tracking must not silently turn on the `pages` output array"
        );
    }

    /// Companion guard: materialising `config.pages` for boundary tracking must not clobber an
    /// explicit `--extract-pages true` given alongside `--ocr-scanned-pages`.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_scanned_pages_does_not_override_explicit_extract_pages_flag() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr_scanned_pages: true,
            extract_pages: Some(true),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let pages = config.pages.expect("pages config should be set");
        assert!(pages.extract_pages, "--extract-pages true must still take effect");
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_validate_unknown_ocr_backend_rejected() {
        let overrides = ExtractionOverrides {
            ocr_backend: Some("unsupported-ocr".to_string()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(err.to_string().contains("Invalid OCR backend"));
    }

    #[cfg(feature = "ocr-surface")]
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
        assert_eq!(ocr.language, vec!["fra".to_string()]);
    }

    #[cfg(feature = "ocr-surface")]
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
        assert_eq!(ocr.language, vec!["ch".to_string()]);
    }

    /// `--ocr-language` alone (no `--ocr true`, no pre-existing `config.ocr`) must
    /// still materialise an OCR config carrying the requested language — naming a
    /// field is enough to select it, exactly like `--ocr-backend`. Before the fix,
    /// this flag was silently discarded whenever `config.ocr` was still `None`.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_language_without_ocr_flag_no_existing_config() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.expect("--ocr-language alone must materialise an OCR config");
        assert_eq!(ocr.language, vec!["deu".to_string()]);
        assert_eq!(ocr.backend, "tesseract", "backend keeps its compiled-in default");
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_language_without_ocr_flag_existing_config() {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: true,
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
                vlm_config: None,
                vlm_fallback: Default::default(),
                vlm_prompt: None,
                acceleration: None,
                security_limits: None,
                tessdata_bytes: None,
                tessdata_path: None,
                backend_options: None,
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
        assert_eq!(ocr.language, vec!["deu".to_string()]);
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_language_updates_existing_nested_tesseract_config() {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: true,
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: Some(xberg::TesseractConfig {
                    language: vec!["eng".to_string()],
                    use_cache: false,
                    ..Default::default()
                }),
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
                vlm_config: None,
                vlm_fallback: Default::default(),
                vlm_prompt: None,
                acceleration: None,
                security_limits: None,
                tessdata_bytes: None,
                tessdata_path: None,
                backend_options: None,
            }),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.language, vec!["deu".to_string()]);
        let tesseract = ocr.tesseract_config.unwrap();
        assert_eq!(tesseract.language, vec!["deu".to_string()]);
        assert!(!tesseract.use_cache);
    }

    /// `--ocr-no-cache` alone (no `--ocr true`, no pre-existing `config.ocr`) must be a
    /// no-op: `config.ocr` must stay `None`.
    ///
    /// Regression test for #693. The flag's entire contract is "bypass the Tesseract OCR
    /// cache, and nothing else" (see its doc comment), but a prior version of
    /// `apply_ocr_no_cache` materialised `tesseract_config` from `TesseractConfig::default()`
    /// whenever it was still `None`, purely to have somewhere to write `use_cache: false`.
    /// That flipped `tesseract_config` from `None` to `Some(..)`, which
    /// `crates/xberg/src/extractors/image.rs::apply_default_tesseract_psm` (and its siblings)
    /// treat as "the caller already made an explicit PSM choice" — disarming the
    /// `WHOLE_IMAGE_TESSERACT_PSM = 11` default for whole-page image OCR and leaving
    /// `TesseractConfig::default()`'s `psm = 3` instead. Measured on a real scan: 217
    /// recognised words without `--ocr-no-cache` vs. 194 with it, from that PSM change alone.
    ///
    /// Against the code before this fix (i.e. `apply_ocr_no_cache` using
    /// `ocr.tesseract_config.get_or_insert_with(|| TesseractConfig { language, ..Default::default() })`
    /// and `has_ocr_field_flag` including `self.ocr_no_cache.is_some()`), this assertion
    /// fails: `config.ocr` is `Some(..)` with `tesseract_config` also `Some(TesseractConfig {
    /// psm: 3, use_cache: false, .. })` instead of `None`.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_no_cache_alone_is_a_no_op_without_existing_tesseract_config() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr_no_cache: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);

        assert!(
            config.ocr.is_none(),
            "--ocr-no-cache alone must not materialise config.ocr: doing so (even just to \
             carry use_cache) would disarm the whole-image PSM default in \
             extractors/image.rs::apply_default_tesseract_psm and silently change what \
             Tesseract recognises"
        );
    }

    /// Companion to the no-op test above for the case where `config.ocr` already exists
    /// (e.g. set by `--ocr true`) but `tesseract_config` itself does not. `--ocr-no-cache`
    /// must still leave `tesseract_config` as `None` rather than materialising it.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_no_cache_leaves_tesseract_config_none_when_ocr_config_exists_without_it() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_no_cache: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must materialise config.ocr");
        assert!(
            ocr.tesseract_config.is_none(),
            "--ocr-no-cache must not materialise tesseract_config on its own, even when \
             config.ocr already exists from --ocr true"
        );
    }

    /// When `tesseract_config` is already set (e.g. by a loaded config file),
    /// `--ocr-no-cache` must flip `use_cache` and change *nothing else*: every other field
    /// of `TesseractConfig` that reaches the OCR engine must come out identical.
    ///
    /// Regression test for #693's core claim ("a flag whose entire contract is caching must
    /// not move recognition output"), and for the class of bug a `use_cache`-only assertion
    /// would miss. Against the code before this fix, this assertion still happens to pass
    /// for this particular case (tesseract_config already `Some`, so the old
    /// `get_or_insert_with` was a no-op and only `use_cache` changed) — the failure mode
    /// this fix targets is exercised by
    /// `test_ocr_no_cache_alone_is_a_no_op_without_existing_tesseract_config` above, which
    /// covers the case this test cannot: `tesseract_config` starting out `None`.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_no_cache_changes_only_use_cache_when_tesseract_config_already_set() {
        let non_default_tesseract_config = xberg::TesseractConfig {
            language: vec!["fra".to_string(), "deu".to_string()],
            psm: Some(11),
            output_format: "hocr".to_string(),
            oem: 1,
            min_confidence: 42.5,
            preprocessing: Some(xberg::ImagePreprocessingConfig {
                target_dpi: 600,
                auto_rotate: true,
                deskew: false,
                denoise: true,
                contrast_enhance: true,
                binarization_method: "sauvola".to_string(),
                invert_colors: true,
            }),
            enable_table_detection: false,
            table_min_confidence: 0.75,
            table_column_threshold: 12,
            table_row_threshold_ratio: 0.9,
            use_cache: true,
            classify_use_pre_adapted_templates: false,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: false,
            tessedit_dont_rowrej_good_wds: false,
            tessedit_enable_dict_correction: false,
            tessedit_char_whitelist: "0123456789".to_string(),
            tessedit_char_blacklist: "@#".to_string(),
            tessedit_use_primary_params_model: false,
            textord_space_size_is_variable: false,
            thresholding_method: true,
        };
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: true,
                backend: "tesseract".to_string(),
                language: vec!["fra".to_string(), "deu".to_string()],
                tesseract_config: Some(non_default_tesseract_config.clone()),
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
                vlm_config: None,
                vlm_fallback: Default::default(),
                vlm_prompt: None,
                acceleration: None,
                security_limits: None,
                tessdata_bytes: None,
                tessdata_path: None,
                backend_options: None,
            }),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            ocr_no_cache: Some(true),
            ..default_overrides()
        };
        overrides.apply(&mut config);

        let tesseract = config.ocr.unwrap().tesseract_config.unwrap();
        assert!(!tesseract.use_cache, "--ocr-no-cache true must disable use_cache");
        assert_eq!(tesseract.language, non_default_tesseract_config.language);
        assert_eq!(tesseract.psm, non_default_tesseract_config.psm);
        assert_eq!(tesseract.output_format, non_default_tesseract_config.output_format);
        assert_eq!(tesseract.oem, non_default_tesseract_config.oem);
        assert_eq!(tesseract.min_confidence, non_default_tesseract_config.min_confidence);
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.target_dpi),
            non_default_tesseract_config
                .preprocessing
                .as_ref()
                .map(|p| p.target_dpi)
        );
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.auto_rotate),
            non_default_tesseract_config
                .preprocessing
                .as_ref()
                .map(|p| p.auto_rotate)
        );
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.deskew),
            non_default_tesseract_config.preprocessing.as_ref().map(|p| p.deskew)
        );
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.denoise),
            non_default_tesseract_config.preprocessing.as_ref().map(|p| p.denoise)
        );
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.contrast_enhance),
            non_default_tesseract_config
                .preprocessing
                .as_ref()
                .map(|p| p.contrast_enhance)
        );
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.binarization_method.clone()),
            non_default_tesseract_config
                .preprocessing
                .as_ref()
                .map(|p| p.binarization_method.clone())
        );
        assert_eq!(
            tesseract.preprocessing.as_ref().map(|p| p.invert_colors),
            non_default_tesseract_config
                .preprocessing
                .as_ref()
                .map(|p| p.invert_colors)
        );
        assert_eq!(
            tesseract.enable_table_detection,
            non_default_tesseract_config.enable_table_detection
        );
        assert_eq!(
            tesseract.table_min_confidence,
            non_default_tesseract_config.table_min_confidence
        );
        assert_eq!(
            tesseract.table_column_threshold,
            non_default_tesseract_config.table_column_threshold
        );
        assert_eq!(
            tesseract.table_row_threshold_ratio,
            non_default_tesseract_config.table_row_threshold_ratio
        );
        assert_eq!(
            tesseract.classify_use_pre_adapted_templates,
            non_default_tesseract_config.classify_use_pre_adapted_templates
        );
        assert_eq!(
            tesseract.language_model_ngram_on,
            non_default_tesseract_config.language_model_ngram_on
        );
        assert_eq!(
            tesseract.tessedit_dont_blkrej_good_wds,
            non_default_tesseract_config.tessedit_dont_blkrej_good_wds
        );
        assert_eq!(
            tesseract.tessedit_dont_rowrej_good_wds,
            non_default_tesseract_config.tessedit_dont_rowrej_good_wds
        );
        assert_eq!(
            tesseract.tessedit_enable_dict_correction,
            non_default_tesseract_config.tessedit_enable_dict_correction
        );
        assert_eq!(
            tesseract.tessedit_char_whitelist,
            non_default_tesseract_config.tessedit_char_whitelist
        );
        assert_eq!(
            tesseract.tessedit_char_blacklist,
            non_default_tesseract_config.tessedit_char_blacklist
        );
        assert_eq!(
            tesseract.tessedit_use_primary_params_model,
            non_default_tesseract_config.tessedit_use_primary_params_model
        );
        assert_eq!(
            tesseract.textord_space_size_is_variable,
            non_default_tesseract_config.textord_space_size_is_variable
        );
        assert_eq!(
            tesseract.thresholding_method,
            non_default_tesseract_config.thresholding_method
        );
    }

    /// `--ocr-no-cache false` (explicitly re-enabling) must flip an already-disabled
    /// `tesseract_config.use_cache` back to `true` rather than being a one-way switch.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_no_cache_false_re_enables_an_already_disabled_cache() {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: true,
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: Some(xberg::TesseractConfig {
                    language: vec!["eng".to_string()],
                    use_cache: false,
                    ..Default::default()
                }),
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
                quality_thresholds: None,
                pipeline: None,
                auto_rotate: false,
                vlm_config: None,
                vlm_fallback: Default::default(),
                vlm_prompt: None,
                acceleration: None,
                security_limits: None,
                tessdata_bytes: None,
                tessdata_path: None,
                backend_options: None,
            }),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            ocr_no_cache: Some(false),
            ..default_overrides()
        };
        overrides.apply(&mut config);

        let ocr = config.ocr.unwrap();
        assert!(ocr.tesseract_config.unwrap().use_cache);
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_language_updates_tesseract_pipeline_stages() {
        let tesseract_config = xberg::TesseractConfig {
            language: vec!["eng".to_string()],
            use_cache: false,
            ..Default::default()
        };
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(xberg::OcrPipelineConfig {
                    stages: vec![
                        xberg::OcrPipelineStage {
                            backend: "tesseract".to_string(),
                            priority: 100,
                            language: Some(vec!["eng".to_string()]),
                            tesseract_config: Some(tesseract_config),
                            paddle_ocr_config: None,
                            vlm_config: None,
                            backend_options: None,
                        },
                        xberg::OcrPipelineStage {
                            backend: "paddle-ocr".to_string(),
                            priority: 90,
                            language: Some(vec!["en".to_string()]),
                            tesseract_config: None,
                            paddle_ocr_config: None,
                            vlm_config: None,
                            backend_options: None,
                        },
                    ],
                    quality_thresholds: Default::default(),
                }),
                ..OcrConfig::default()
            }),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let stages = &config.ocr.unwrap().pipeline.unwrap().stages;
        assert_eq!(stages[0].language, Some(vec!["deu".to_string()]));
        assert_eq!(
            stages[0].tesseract_config.as_ref().unwrap().language,
            vec!["deu".to_string()]
        );
        assert_eq!(stages[1].backend, "paddle-ocr");
        assert_eq!(stages[1].priority, 90);
        assert_eq!(stages[1].language, Some(vec!["en".to_string()]));
        assert!(stages[1].tesseract_config.is_none());
        assert!(stages[1].paddle_ocr_config.is_none());
        assert!(stages[1].vlm_config.is_none());
        assert!(stages[1].backend_options.is_none());
    }

    #[cfg(feature = "ocr-surface")]
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

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_enable_ocr_when_cli_true_overrides_loaded_disable() {
        let mut config = ExtractionConfig {
            disable_ocr: true,
            ..ExtractionConfig::default()
        };
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        assert!(
            !config.disable_ocr,
            "explicit --ocr true should override loaded disable_ocr=true"
        );
        assert!(config.ocr.expect("--ocr true should configure OCR").enabled);
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_preserve_loaded_disable_ocr_when_ocr_flag_is_absent() {
        let mut config = ExtractionConfig {
            disable_ocr: true,
            ..ExtractionConfig::default()
        };

        default_overrides().apply(&mut config);

        assert!(config.disable_ocr, "an absent OCR flag should preserve loaded config");
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_clear_loaded_ocr_routing_when_cli_false_hard_disables_ocr() {
        let mut config = ExtractionConfig {
            force_ocr: true,
            ocr_strategy: xberg::OcrStrategy::ScannedPages { min_confidence: 0.8 },
            force_ocr_pages: Some(vec![2, 4]),
            ..ExtractionConfig::default()
        };
        let overrides = ExtractionOverrides {
            ocr: Some(false),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        assert!(!config.force_ocr, "--ocr false should override loaded force_ocr=true");
        assert_eq!(config.ocr_strategy, xberg::OcrStrategy::Auto);
        assert!(
            config.force_ocr_pages.is_none(),
            "--ocr false should clear forced page selection"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_reject_ocr_false_with_force_ocr_true() {
        let overrides = ExtractionOverrides {
            ocr: Some(false),
            force_ocr: Some(true),
            ..default_overrides()
        };

        assert_eq!(
            overrides
                .validate()
                .expect_err("disabling and forcing OCR should conflict")
                .to_string(),
            "--ocr false cannot be combined with --force-ocr true"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_reject_ocr_false_with_scanned_pages() {
        let overrides = ExtractionOverrides {
            ocr: Some(false),
            ocr_scanned_pages: true,
            ..default_overrides()
        };

        assert_eq!(
            overrides
                .validate()
                .expect_err("disabling OCR and selecting scanned pages should conflict")
                .to_string(),
            "--ocr false cannot be combined with --ocr-scanned-pages"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_reject_contradictory_ocr_and_disable_ocr_values() {
        for (ocr, disable_ocr) in [(true, true), (false, false)] {
            let overrides = ExtractionOverrides {
                ocr: Some(ocr),
                disable_ocr: Some(disable_ocr),
                ..default_overrides()
            };

            assert_eq!(
                overrides
                    .validate()
                    .expect_err("contradictory OCR flags should be rejected")
                    .to_string(),
                "--ocr and --disable-ocr specify contradictory values"
            );
        }
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_options_parsed_and_applied() {
        let mut config = ExtractionConfig::default();
        let backend_options_json = r#"{"task":"chart","layout_mode":"whole_page"}"#;
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend_options: Some(backend_options_json.to_string()),
            ..default_overrides()
        };

        assert!(overrides.validate().is_ok());

        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();

        assert!(ocr.backend_options.is_some());
        let opts = ocr.backend_options.unwrap();
        assert!(opts.is_object());
        assert_eq!(opts.get("task").and_then(|v| v.as_str()), Some("chart"));
        assert_eq!(opts.get("layout_mode").and_then(|v| v.as_str()), Some("whole_page"));
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_options_invalid_json_fails_validation() {
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend_options: Some("not-valid-json".to_string()),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_options_not_object_fails_validation() {
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend_options: Some(r#"["array", "not", "object"]"#.to_string()),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    /// Mirrors `main.rs`: config file/defaults -> `--config-json` -> individual CLI flags.
    #[cfg(any(feature = "ocr-surface", feature = "core-cli", feature = "analysis"))]
    fn config_from_json(json: &str) -> ExtractionConfig {
        let mut config = ExtractionConfig::default();
        crate::input::apply_json_overrides(&mut config, Some(json.to_string()), None)
            .expect("--config-json should merge into the base config");
        config
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_keep_config_json_quality_thresholds_when_ocr_flags_are_also_given() {
        let mut config =
            config_from_json(r#"{"ocr":{"quality_thresholds":{"max_ocr_output_fragmented_word_ratio":0.99}}}"#);
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("tesseract".to_string()),
            ocr_language: Some("eng".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, vec!["eng".to_string()]);
        let thresholds = ocr
            .quality_thresholds
            .expect("quality_thresholds has no CLI flag, so --config-json must remain its only source");
        assert_eq!(thresholds.max_ocr_output_fragmented_word_ratio, 0.99);
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_keep_config_json_tessdata_path_when_ocr_flags_are_also_given() {
        let mut config = config_from_json(r#"{"ocr":{"tessdata_path":"/opt/tessdata"}}"#);
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("tesseract".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert_eq!(
            ocr.tessdata_path,
            Some(std::path::PathBuf::from("/opt/tessdata")),
            "tessdata_path has no CLI flag and must survive --ocr/--ocr-backend"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_keep_every_non_flag_ocr_field_when_ocr_flags_are_also_given() {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                tesseract_config: Some(xberg::TesseractConfig {
                    language: vec!["eng".to_string()],
                    use_cache: false,
                    ..Default::default()
                }),
                quality_thresholds: Some(xberg::OcrQualityThresholds {
                    max_ocr_output_fragmented_word_ratio: 0.99,
                    ..Default::default()
                }),
                pipeline: Some(xberg::OcrPipelineConfig {
                    stages: vec![xberg::OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: Some(vec!["eng".to_string()]),
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        vlm_config: None,
                        backend_options: None,
                    }],
                    quality_thresholds: Default::default(),
                }),
                vlm_config: Some(LlmConfig {
                    model: "openai/gpt-4o".to_string(),
                    ..Default::default()
                }),
                vlm_fallback: xberg::VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 },
                vlm_prompt: Some("custom prompt".to_string()),
                paddle_ocr_config: Some(serde_json::json!({"model_version": "pp-ocrv5"})),
                backend_options: Some(serde_json::json!({"mode": "fast"})),
                tessdata_path: Some(std::path::PathBuf::from("/opt/tessdata")),
                ..OcrConfig::default()
            }),
            ..Default::default()
        };
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("tesseract".to_string()),
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert_eq!(ocr.language, vec!["deu".to_string()]);

        let tesseract = ocr.tesseract_config.clone().expect("tesseract_config must survive");
        assert_eq!(
            tesseract.language,
            vec!["deu".to_string()],
            "--ocr-language must propagate into the preserved nested Tesseract config"
        );
        assert!(
            !tesseract.use_cache,
            "fields no CLI flag names must stay exactly as configured"
        );

        let thresholds = ocr.quality_thresholds.clone().expect("quality_thresholds must survive");
        assert_eq!(thresholds.max_ocr_output_fragmented_word_ratio, 0.99);

        let pipeline = ocr.pipeline.clone().expect("pipeline must survive");
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].language, Some(vec!["deu".to_string()]));

        let vlm = ocr.vlm_config.clone().expect("vlm_config must survive");
        assert_eq!(vlm.model, "openai/gpt-4o");
        assert_eq!(
            ocr.vlm_fallback,
            xberg::VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 }
        );
        assert_eq!(ocr.vlm_prompt.as_deref(), Some("custom prompt"));
        assert_eq!(
            ocr.paddle_ocr_config,
            Some(serde_json::json!({"model_version": "pp-ocrv5"}))
        );
        assert_eq!(ocr.backend_options, Some(serde_json::json!({"mode": "fast"})));
        assert_eq!(ocr.tessdata_path, Some(std::path::PathBuf::from("/opt/tessdata")));
    }

    /// Guards the other half of the precedence rule: the fix must not make
    /// `--config-json` win over the flag. Passes before and after the fix.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_let_ocr_backend_flag_win_over_config_json_backend() {
        let mut config = config_from_json(r#"{"ocr":{"backend":"sceptre"}}"#);
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("tesseract".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert_eq!(ocr.backend, "tesseract");
    }

    /// Same guard for `--ocr-language`. Passes before and after the fix.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_let_ocr_language_flag_win_over_config_json_language() {
        let mut config = config_from_json(r#"{"ocr":{"language":["fra"]}}"#);
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_language: Some("deu".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert_eq!(ocr.language, vec!["deu".to_string()]);
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_keep_config_json_auto_rotate_when_no_auto_rotate_flag_is_given() {
        let mut config = config_from_json(r#"{"ocr":{"auto_rotate":true}}"#);
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("tesseract".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert!(
            ocr.auto_rotate,
            "auto_rotate came from --config-json, and no --ocr-auto-rotate flag was given"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn should_keep_config_json_language_when_only_the_ocr_flag_is_given() {
        let mut config = config_from_json(r#"{"ocr":{"language":["deu","fra"]}}"#);
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("--ocr true must leave an OCR config in place");
        assert_eq!(ocr.language, vec!["deu".to_string(), "fra".to_string()]);
    }

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
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

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
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

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
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

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[test]
    fn should_keep_config_json_chunking_siblings_when_chunk_flags_are_also_given() {
        let mut config =
            config_from_json(r#"{"chunking":{"chunker_type":"markdown","table_chunking":"repeat_header"}}"#);
        let overrides = ExtractionOverrides {
            chunk: Some(true),
            chunk_size: Some(512),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let chunking = config
            .chunking
            .expect("--chunk true must leave a chunking config in place");
        assert_eq!(chunking.max_characters, 512);
        assert_eq!(
            chunking.chunker_type,
            xberg::ChunkerType::Markdown,
            "chunker_type has no CLI flag and must survive --chunk/--chunk-size"
        );
        assert_eq!(chunking.table_chunking, xberg::TableChunkingMode::RepeatHeader);
    }

    /// Guards the other half of the precedence rule for chunking. Passes before
    /// and after the fix.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[test]
    fn should_let_chunk_size_flag_win_over_config_json_max_chars() {
        let mut config = config_from_json(r#"{"chunking":{"max_chars":4000}}"#);
        let overrides = ExtractionOverrides {
            chunk: Some(true),
            chunk_size: Some(512),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let chunking = config
            .chunking
            .expect("--chunk true must leave a chunking config in place");
        assert_eq!(chunking.max_characters, 512);
    }

    /// Regression test for the field-drop shape #654 describes, structurally identical to the
    /// `--ocr-backend` defect fixed in `5921a7cc23`: `apply_chunking` only materialised
    /// `config.chunking` on `--chunk true`, `--chunking-tokenizer`, or a config file that
    /// already set it, so `--chunk-size 512` alone (no `--chunk true`) hit the `let Some(chunking)
    /// = config.chunking.as_mut() else { return; }` early return and was silently dropped --
    /// no warning, no error, just an unchanged config. Before the fix (i.e. reverting
    /// `has_chunk_field_flag` back out of the materialisation condition), `config.chunking`
    /// stays `None` here and this assertion fails on `.expect(..)`.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[test]
    fn test_chunk_size_flag_materialises_chunking_without_chunk_true_flag() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            chunk_size: Some(512),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let chunking = config
            .chunking
            .expect("--chunk-size must materialise a chunking config even without --chunk true");
        assert_eq!(chunking.max_characters, 512);
    }

    /// Same defect, `--chunk-overlap` alone.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[test]
    fn test_chunk_overlap_flag_materialises_chunking_without_chunk_true_flag() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            chunk_overlap: Some(50),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let chunking = config
            .chunking
            .expect("--chunk-overlap must materialise a chunking config even without --chunk true");
        assert_eq!(chunking.overlap, 50);
    }

    #[cfg(feature = "analysis")]
    #[test]
    fn should_keep_config_json_language_detection_siblings_when_detect_language_flag_is_given() {
        let mut config = config_from_json(r#"{"language_detection":{"min_confidence":0.5,"detect_multiple":true}}"#);
        let overrides = ExtractionOverrides {
            detect_language: Some(true),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let detection = config
            .language_detection
            .expect("--detect-language true must leave a language-detection config in place");
        assert!(detection.enabled);
        assert_eq!(
            detection.min_confidence, 0.5,
            "min_confidence has no CLI flag of its own and must survive --detect-language"
        );
        assert!(detection.detect_multiple);
    }

    #[cfg(feature = "analysis")]
    #[test]
    fn should_keep_config_json_token_reduction_siblings_when_token_reduction_flag_is_given() {
        let mut config = config_from_json(r#"{"token_reduction":{"preserve_important_words":false}}"#);
        let overrides = ExtractionOverrides {
            token_reduction: Some(ReductionLevelArg::Aggressive),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        let reduction = config.token_reduction.expect("--token-reduction must set a config");
        assert_eq!(reduction.mode, "aggressive");
        assert!(
            !reduction.preserve_important_words,
            "preserve_important_words has no CLI flag and must survive --token-reduction"
        );
    }

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[test]
    fn test_validate_chunk_size_zero() {
        let overrides = ExtractionOverrides {
            chunk_size: Some(0),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[test]
    fn test_validate_chunk_size_too_large() {
        let overrides = ExtractionOverrides {
            chunk_size: Some(2_000_000),
            ..default_overrides()
        };
        assert!(overrides.validate().is_err());
    }

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
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

    #[test]
    fn test_validate_csv_delimiter_valid() {
        let overrides = ExtractionOverrides {
            csv_delimiter: Some(";".to_string()),
            ..default_overrides()
        };
        assert!(overrides.validate().is_ok());
    }

    #[test]
    fn test_validate_csv_delimiter_empty_rejected() {
        let overrides = ExtractionOverrides {
            csv_delimiter: Some(String::new()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid CSV delimiter ''. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|')."
        );
    }

    #[test]
    fn test_validate_csv_delimiter_multi_byte_rejected() {
        let overrides = ExtractionOverrides {
            csv_delimiter: Some("::".to_string()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid CSV delimiter '::'. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|')."
        );
    }

    #[test]
    fn test_apply_csv_delimiter_and_comment_prefixes() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            csv_delimiter: Some(";".to_string()),
            csv_comment_prefix: vec!["#".to_string(), "//".to_string()],
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let csv = config.csv.expect("csv config should be set");
        assert_eq!(csv.delimiter.as_deref(), Some(";"));
        assert_eq!(csv.comment_prefixes, vec!["#".to_string(), "//".to_string()]);
    }

    #[test]
    fn test_apply_csv_no_flags_leaves_config_untouched() {
        let mut config = ExtractionConfig::default();
        let overrides = default_overrides();
        overrides.apply(&mut config);
        assert!(config.csv.is_none());
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

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_layout_table_model_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            layout_table_model: Some("slanet_wired".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let layout = config.layout.unwrap();
        assert_eq!(layout.table_model, xberg::TableModel::SlanetWired);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_layout_strategy_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            layout_strategy: Some("auto".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let layout = config.layout.unwrap();
        assert_eq!(layout.strategy, xberg::LayoutStrategy::Auto);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_layout_strategy_rejects_unknown_value() {
        let overrides = ExtractionOverrides {
            layout_strategy: Some("adaptive".to_string()),
            ..default_overrides()
        };
        let error = overrides.validate().expect_err("unknown strategy must fail");
        assert!(error.to_string().contains("Invalid layout strategy"));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_layout_strategy_conflicts_with_layout_false() {
        let overrides = ExtractionOverrides {
            layout: Some(false),
            layout_strategy: Some("auto".to_string()),
            ..default_overrides()
        };
        let error = overrides.validate().expect_err("conflicting flags must fail");
        assert!(error.to_string().contains("--layout-strategy"));
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

    /// Make every callsite in the process permanently interesting, once, so a
    /// `tracing::warn!` reached on some other test's thread first isn't cached as
    /// `Interest::never()` for the whole process and lost to `capture_logs` below.
    /// Mirrors the pattern documented at `crates/xberg/src/cache/mod.rs` (#272/#301).
    #[cfg(feature = "layout-detection")]
    fn install_permissive_global_subscriber() {
        struct AlwaysInterested;

        impl tracing::Subscriber for AlwaysInterested {
            fn register_callsite(&self, _: &'static tracing::Metadata<'static>) -> tracing::subscriber::Interest {
                tracing::subscriber::Interest::always()
            }

            fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
                true
            }

            fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
                Some(tracing::level_filters::LevelFilter::TRACE)
            }

            fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::Id {
                tracing::Id::from_u64(1)
            }

            fn record(&self, _: &tracing::Id, _: &tracing::span::Record<'_>) {}
            fn record_follows_from(&self, _: &tracing::Id, _: &tracing::Id) {}
            fn event(&self, _: &tracing::Event<'_>) {}
            fn enter(&self, _: &tracing::Id) {}
            fn exit(&self, _: &tracing::Id) {}
        }

        static INSTALLED: std::sync::Once = std::sync::Once::new();
        INSTALLED.call_once(|| {
            let _ = tracing::subscriber::set_global_default(AlwaysInterested);
            tracing::callsite::rebuild_interest_cache();
        });
    }

    /// Capture `tracing` output emitted on this thread while `body` runs.
    #[cfg(feature = "layout-detection")]
    fn capture_logs<T>(body: impl FnOnce() -> T) -> (T, String) {
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct Capture(Arc<Mutex<Vec<u8>>>);

        impl std::io::Write for Capture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().expect("log buffer poisoned").write_all(buf)?;
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let capture = Capture(Arc::clone(&buffer));
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::WARN)
            .with_writer(move || capture.clone())
            .finish();

        install_permissive_global_subscriber();

        let value = tracing::subscriber::with_default(subscriber, body);
        let logs =
            String::from_utf8(buffer.lock().expect("log buffer poisoned").clone()).expect("log output must be UTF-8");
        (value, logs)
    }

    /// Regression test for contract point 4: enabling layout detection while
    /// `output_format` stays `Plain` (the default) wastes the layout pass -- the extraction
    /// still pays the model's cost (20s-202s per the WP-E measurements) and `Plain` never
    /// renders the headings/lists/tables it detects. Before this warning was wired in
    /// (i.e. removing the `warn_layout_wastes_plain_output` call from `apply`), `apply_layout`
    /// never read `output_format` at all, so no warning was logged and this assertion on the
    /// captured log output fails.
    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_warns_when_layout_enabled_with_plain_output_format() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            layout: Some(true),
            ..default_overrides()
        };

        let (_, logs) = capture_logs(|| overrides.apply(&mut config));

        assert!(config.layout.is_some(), "--layout must still enable layout detection");
        assert_eq!(
            config.output_format,
            xberg::OutputFormat::Plain,
            "Plain must stay the default"
        );
        assert!(
            logs.contains("layout detection is enabled but the output format is 'plain'"),
            "expected a layout/Plain contract warning in the captured log, got: {logs}"
        );
    }

    /// Companion guard: layout combined with a structured output format must not warn.
    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_does_not_warn_when_layout_enabled_with_markdown_output_format() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            layout: Some(true),
            content_format: Some(ContentOutputFormatArg::Markdown),
            ..default_overrides()
        };

        let (_, logs) = capture_logs(|| overrides.apply(&mut config));

        assert!(config.layout.is_some());
        assert!(
            !logs.contains("layout detection is enabled but the output format is 'plain'"),
            "no wasted-layout warning expected for markdown output, got: {logs}"
        );
    }

    /// Companion guard: `Plain` output alone, with layout left off (the double default),
    /// must not warn.
    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_does_not_warn_when_layout_disabled_with_plain_output_format() {
        let mut config = ExtractionConfig::default();
        let overrides = default_overrides();

        let (_, logs) = capture_logs(|| overrides.apply(&mut config));

        assert!(config.layout.is_none(), "layout must stay off by default");
        assert!(
            !logs.contains("layout detection is enabled but the output format is 'plain'"),
            "no warning expected when layout is off, got: {logs}"
        );
    }

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

    #[cfg(feature = "analysis")]
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

    #[test]
    fn test_max_concurrent_ocr_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            max_threads: Some(16),
            max_concurrent_ocr: Some(4),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let concurrency = config.concurrency.unwrap();
        assert_eq!(concurrency.max_threads, Some(16));
        assert_eq!(concurrency.max_concurrent_ocr, Some(4));
    }

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

    #[cfg(feature = "ocr-surface")]
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
    fn test_validate_max_concurrent_ocr_zero() {
        let overrides = ExtractionOverrides {
            max_concurrent_ocr: Some(0),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(err.to_string().contains("--max-concurrent-ocr must be at least 1"));
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_validate_valid_ocr_backends() {
        for backend in &["tesseract", "paddle-ocr", "sceptre"] {
            let overrides = ExtractionOverrides {
                ocr_backend: Some(backend.to_string()),
                ..default_overrides()
            };
            assert!(overrides.validate().is_ok(), "Expected backend '{backend}' to be valid");
        }
    }

    /// The three candle VLM backends declare identical `supported_languages()` sets, so
    /// they must all default to the same short code. `candle-deepseek-ocr` was missing
    /// from `PADDLE_LANGUAGE_BACKENDS`, defaulting to `"eng"` where its siblings use
    /// `"en"`. Against unfixed code the deepseek row below returns `"eng"`.
    #[cfg(feature = "ocr-surface")]
    #[test]
    fn every_candle_vlm_backend_defaults_to_the_same_short_language_code() {
        for backend in ["candle-paddleocr-vl", "candle-glm-ocr", "candle-deepseek-ocr"] {
            assert_eq!(
                default_language_for_backend(backend),
                DEFAULT_PADDLE_OCR_LANGUAGE,
                "{backend} must default to the short ISO 639-1 code its siblings use"
            );
        }
        assert_eq!(
            default_language_for_backend("tesseract"),
            DEFAULT_OCR_LANGUAGE,
            "a backend outside the family must keep the ISO 639-3 default"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_apply_ocr_preserves_candle_deepseek_backend() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("candle-deepseek-ocr".to_string()),
            ..default_overrides()
        };

        overrides.apply(&mut config);

        assert_eq!(
            config.ocr.expect("OCR config should be set").backend,
            "candle-deepseek-ocr"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_validate_vlm_backend_requires_model() {
        let overrides = ExtractionOverrides {
            ocr_backend: Some("vlm".to_string()),
            ..default_overrides()
        };

        let error = overrides
            .validate()
            .expect_err("VLM backend without a model should fail");

        assert_eq!(
            error.to_string(),
            "--ocr-backend vlm requires --vlm-model to be specified"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_options_threaded_into_config() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("candle-glm-ocr".to_string()),
            ocr_backend_options: Some(r#"{"layout_mode":"whole_page"}"#.to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "candle-glm-ocr");
        let opts = ocr.backend_options.expect("backend_options should be Some");
        assert_eq!(opts, serde_json::json!({"layout_mode": "whole_page"}));
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_validate_rejects_non_object_backend_options() {
        let overrides = ExtractionOverrides {
            ocr_backend_options: Some(r#"["not","an","object"]"#.to_string()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(
            err.to_string().contains("--ocr-backend-options must be a JSON object"),
            "unexpected error: {err}"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_validate_rejects_invalid_json_backend_options() {
        let overrides = ExtractionOverrides {
            ocr_backend_options: Some("not-json".to_string()),
            ..default_overrides()
        };
        let err = overrides.validate().unwrap_err();
        assert!(
            err.to_string().contains("invalid --ocr-backend-options JSON"),
            "unexpected error: {err}"
        );
    }

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_options_none_when_absent() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            ocr: Some(true),
            ocr_backend: Some("candle-glm-ocr".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let ocr = config.ocr.unwrap();
        assert!(ocr.backend_options.is_none());
    }

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
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
        let overrides = ExtractionOverrides {
            chunk_overlap: Some(1500),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let chunking = config.chunking.unwrap();
        assert_eq!(chunking.overlap, 800 / 4);
        assert_eq!(chunking.max_characters, 800);
    }

    #[cfg(any(feature = "core-cli", feature = "analysis"))]
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
        let overrides = ExtractionOverrides {
            chunk_overlap: Some(200),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let chunking = config.chunking.unwrap();
        assert_eq!(chunking.overlap, 200);
        assert_eq!(chunking.max_characters, 800);
    }

    #[cfg(all(
        any(feature = "core-cli", feature = "analysis"),
        not(feature = "chunking-tokenizers")
    ))]
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

    /// Lock around the `XBERG_LLM_API_KEY` env var to keep the resolution
    /// tests deterministic in the multi-threaded test runner. Tests that touch
    /// the environment must hold this guard for their full duration.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[allow(unsafe_code)]
    fn with_env_var<R>(key: &str, value: Option<&str>, f: impl FnOnce() -> R) -> R {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var(key).ok();
        unsafe {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let result = f();
        unsafe {
            match previous {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        result
    }

    #[test]
    fn cli_flag_takes_precedence_over_env_var() {
        with_env_var("XBERG_LLM_API_KEY", Some("env-value"), || {
            let resolved = resolve_llm_api_key(Some("cli-value"));
            assert_eq!(resolved.as_deref(), Some("cli-value"));
        });
    }

    #[test]
    fn env_var_used_when_cli_flag_absent() {
        with_env_var("XBERG_LLM_API_KEY", Some("env-value"), || {
            let resolved = resolve_llm_api_key(None);
            assert_eq!(resolved.as_deref(), Some("env-value"));
        });
    }

    #[test]
    fn returns_none_when_neither_source_is_set() {
        with_env_var("XBERG_LLM_API_KEY", None, || {
            let resolved = resolve_llm_api_key(None);
            assert!(resolved.is_none());
        });
    }

    #[test]
    fn empty_cli_flag_does_not_count() {
        with_env_var("XBERG_LLM_API_KEY", Some("env-value"), || {
            let resolved = resolve_llm_api_key(Some("   "));
            assert_eq!(resolved.as_deref(), Some("env-value"));
        });
    }

    #[test]
    fn apply_llm_api_key_fills_translation_slot() {
        use xberg::core::config::{LlmConfig, TranslationConfig};
        let mut config = ExtractionConfig {
            translation: Some(TranslationConfig {
                target_lang: "de".to_string(),
                source_lang: None,
                preserve_markup: false,
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        apply_llm_api_key(&mut config, "resolved");
        let t = config.translation.unwrap();
        assert_eq!(t.llm.api_key.as_deref(), Some("resolved"));
    }

    #[test]
    fn apply_llm_api_key_preserves_existing_key() {
        use xberg::core::config::{LlmConfig, TranslationConfig};
        let mut config = ExtractionConfig {
            translation: Some(TranslationConfig {
                target_lang: "de".to_string(),
                source_lang: None,
                preserve_markup: false,
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    api_key: Some("explicit".to_string()),
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        apply_llm_api_key(&mut config, "resolved");
        let t = config.translation.unwrap();
        assert_eq!(
            t.llm.api_key.as_deref(),
            Some("explicit"),
            "explicit config keys take precedence over the resolved value"
        );
    }

    #[test]
    fn apply_llm_api_key_fills_page_classification_slot() {
        use xberg::core::config::{LlmConfig, PageClassificationConfig};
        let mut config = ExtractionConfig {
            page_classification: Some(PageClassificationConfig {
                prompt_template: None,
                labels: vec!["a".to_string()],
                multi_label: false,
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        apply_llm_api_key(&mut config, "resolved");
        let pc = config.page_classification.unwrap();
        assert_eq!(pc.llm.api_key.as_deref(), Some("resolved"));
    }

    #[test]
    fn test_no_overrides_leaves_config_unchanged() {
        let original = ExtractionConfig::default();
        let mut config = original.clone();
        let overrides = default_overrides();
        overrides.apply(&mut config);

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

    #[cfg(feature = "ocr-surface")]
    #[test]
    fn test_ocr_backend_options_vlm_flow() {
        let mut config = ExtractionConfig::default();
        let backend_options_json = r#"{"task":"chart","layout_mode":"whole_page"}"#;
        let overrides = ExtractionOverrides {
            vlm_model: Some("openai/gpt-4o".to_string()),
            ocr_backend_options: Some(backend_options_json.to_string()),
            ..default_overrides()
        };

        assert!(overrides.validate().is_ok());

        overrides.apply(&mut config);

        let ocr = config.ocr.expect("OCR should be configured");
        assert_eq!(ocr.backend, "vlm");

        let opts = ocr.backend_options.expect("backend_options should be Some");
        assert!(opts.is_object());
        assert_eq!(opts.get("task").and_then(|v| v.as_str()), Some("chart"));
        assert_eq!(opts.get("layout_mode").and_then(|v| v.as_str()), Some("whole_page"));

        assert!(ocr.vlm_config.is_some());
        let vlm = ocr.vlm_config.unwrap();
        assert_eq!(vlm.model, "openai/gpt-4o");
    }

    // -- --pdf-backend (#700) ------------------------------------------------------

    /// Before this change, `apply_pdf`'s `has_pdf_flag` disjunction never checked
    /// `pdf_backend`, so a bare `--pdf-backend native` with no other PDF flag left
    /// `config.pdf_options` at `None` -- the flag was applied to nothing. This does not
    /// need `xberg::PdfBackend` to compile, so it exercises today's actual bug directly:
    /// this assertion fails against unfixed code (`pdf_options` stays `None`).
    #[cfg(feature = "pdf-surface")]
    #[test]
    fn test_pdf_backend_flag_alone_populates_pdf_options() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            pdf_backend: Some("native".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        assert!(
            config.pdf_options.is_some(),
            "a bare --pdf-backend flag must populate pdf_options even with no other PDF flag set"
        );
    }

    /// New surface: `xberg::PdfBackend` does not exist before this change, so this test
    /// cannot even compile against unfixed code -- it is new-surface-only, not a
    /// fails-today regression test.
    #[cfg(feature = "pdf-surface")]
    #[test]
    fn test_pdf_backend_pdfium_applied() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            pdf_backend: Some("pdfium".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let pdf = config.pdf_options.expect("pdf_options must be populated");
        assert_eq!(pdf.backend, xberg::PdfBackend::Pdfium);
    }

    /// New surface, same reason as above -- `xberg::PdfBackend` does not exist today.
    #[cfg(feature = "pdf-surface")]
    #[test]
    fn test_pdf_backend_default_applied_is_native() {
        let mut config = ExtractionConfig::default();
        let overrides = ExtractionOverrides {
            pdf_backend: Some("native".to_string()),
            ..default_overrides()
        };
        overrides.apply(&mut config);
        let pdf = config.pdf_options.expect("pdf_options must be populated");
        assert_eq!(pdf.backend, xberg::PdfBackend::Native);
    }

    #[cfg(feature = "pdf-surface")]
    #[test]
    fn test_pdf_backend_rejects_unknown_value() {
        let overrides = ExtractionOverrides {
            pdf_backend: Some("xyz".to_string()),
            ..default_overrides()
        };
        let error = overrides.validate().expect_err("unknown backend must fail");
        assert!(
            error.to_string().contains("native"),
            "error should mention 'native', got: {error}"
        );
    }

    /// Fails today: the old validator's message is "Invalid PDF backend '<x>'. Only
    /// 'native' is currently supported." for *any* value other than "native",
    /// including "pdfium" -- it does not name a rebuild feature, so
    /// `contains("pdf-pdfium-surface")` is false against unfixed code. It becomes true
    /// only once the validator gains the feature-gated actionable-error branch this
    /// change adds.
    #[cfg(all(feature = "pdf-surface", not(feature = "pdf-pdfium-surface")))]
    #[test]
    fn test_pdf_backend_pdfium_rejected_without_feature_names_rebuild_hint() {
        let overrides = ExtractionOverrides {
            pdf_backend: Some("pdfium".to_string()),
            ..default_overrides()
        };
        let error = overrides
            .validate()
            .expect_err("pdfium must be rejected without the feature");
        assert!(
            error.to_string().contains("pdf-pdfium-surface"),
            "error should name the feature to rebuild with, got: {error}"
        );
    }

    #[cfg(feature = "pdf-pdfium-surface")]
    #[test]
    fn test_pdf_backend_pdfium_accepted_with_feature() {
        let overrides = ExtractionOverrides {
            pdf_backend: Some("pdfium".to_string()),
            ..default_overrides()
        };
        assert!(overrides.validate().is_ok());
    }
}
