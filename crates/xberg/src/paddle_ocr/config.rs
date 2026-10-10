//! Configuration for PaddleOCR backend via ONNX Runtime.
//!
//! This module provides comprehensive configuration for PaddleOCR text detection, angle
//! classification, and recognition. Supports multi-language OCR with customizable detection
//! and recognition thresholds.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(super) const MIN_RECOGNITION_BATCH_SIZE: u32 = 1;
pub(super) const DEFAULT_RECOGNITION_BATCH_SIZE: u32 = 6;
pub(super) const MAX_RECOGNITION_BATCH_SIZE: u32 = 64;
const DEFAULT_DETECTION_LIMIT_SIDE_LEN: u32 = 1024;

/// Configuration for PaddleOCR backend.
///
/// Configures PaddleOCR text detection and recognition with multi-language support.
/// Uses a builder pattern for convenient configuration.
///
/// # Examples
///
/// ```no_run
/// use xberg::PaddleOcrConfig;
///
/// // Create with default English configuration
/// let config = PaddleOcrConfig::new("en");
///
/// // Create with custom cache directory
/// let config = PaddleOcrConfig::new("ch")
///     .with_cache_dir("/path/to/cache".into());
///
/// // Table detection is off by default (unlike `TesseractConfig`); PaddleOCR has no
/// // per-word table-candidate signal, so unfiltered clustering over-fabricates tables
/// // on ordinary prose. Enable it explicitly once you've validated it for your corpus.
/// let config = PaddleOcrConfig::new("en")
///     .with_table_detection(true);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PaddleOcrConfig {
    /// Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra")
    pub language: String,

    /// Optional Hugging Face Hub cache root for model files.
    ///
    /// When unset, `XBERG_OCR_MODEL_CACHE_DIR` is used when non-empty, followed by the standard
    /// `HF_HUB_CACHE`, legacy `HUGGINGFACE_HUB_CACHE`, and `HF_HOME` conventions.
    pub cache_dir: Option<PathBuf>,

    /// Enable angle classification for rotated text (default: false).
    /// Can misfire on short text regions, rotating crops incorrectly before recognition.
    pub use_angle_cls: bool,

    /// Enable table structure detection (default: **false**, unlike `TesseractConfig`
    /// which defaults to `true` — see `crate::ocr::types::TesseractConfig`).
    ///
    /// PaddleOCR has no per-word table-candidate confidence carve-out the way Tesseract's
    /// TSV does (every recognised word is a clustering candidate), so unfiltered clustering
    /// over-fabricates tables on ordinary prose. This stays off until it clears a
    /// cell-scored accuracy measurement (see the pinning test at the bottom of this file).
    /// Consequence for callers: switching an existing config from the Tesseract backend to
    /// PaddleOCR with stock defaults silently produces **no** OCR tables — enable this
    /// explicitly if you need them.
    ///
    /// When enabled, this only clusters and reconstructs a grid from word boxes PaddleOCR
    /// has already produced (see `crate::paddle_ocr::backend::PaddleOcrBackend::process_image`)
    /// — it does not run any additional model inference, so the added per-page cost is the
    /// CPU-only clustering/reconstruction pass, not another ONNX call.
    pub enable_table_detection: bool,

    /// Database threshold for text detection (default: 0.3)
    /// Range: 0.0-1.0, higher values require more confident detections
    pub det_db_thresh: f32,

    /// Box threshold for text bounding box refinement (default: 0.5)
    /// Range: 0.0-1.0
    pub det_db_box_thresh: f32,

    /// Unclip ratio for expanding text bounding boxes (default: 1.6)
    /// Controls the expansion of detected text regions
    pub det_db_unclip_ratio: f32,

    /// Maximum side length for detection image (default: 1024)
    /// Larger images may be resized to this limit for faster inference
    pub det_limit_side_len: u32,

    /// Batch size for recognition inference (default: 6)
    /// Number of text regions to process simultaneously
    pub rec_batch_num: u32,

    /// Padding in pixels added around the image before detection (default: 10).
    /// Large values can include surrounding content like table gridlines.
    pub padding: u32,

    /// Minimum recognition confidence score for text lines (default: 0.5).
    /// Text regions with recognition confidence below this threshold are discarded.
    /// Matches PaddleOCR Python's `drop_score` parameter.
    /// Range: 0.0-1.0
    pub drop_score: f32,

    /// Model tier controlling detection/recognition model size and accuracy trade-off.
    ///
    /// For PP-OCRv5 (`model_version = "pp-ocrv5"`):
    /// - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference
    /// - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents
    ///
    /// For PP-OCRv6 (`model_version = "pp-ocrv6"`, the default):
    /// - `"small"`: ~9.9MB detection, full 18,708-char CJK+Latin+JA/KO recognition dictionary.
    ///   The default `"mobile"` resolves here, so this is what an unconfigured extraction uses.
    /// - `"medium"`: ~62MB detection, same dictionary. Higher accuracy, substantially slower on
    ///   CPU. A legacy `"server"` tier, or any unrecognised value, resolves here.
    /// - `"tiny"`: ~1.8MB detection, but a reduced 6,904-char (~zh/en) dictionary — it cannot
    ///   read the scripts the other two cover.
    ///
    /// Note PaddleOCR pages do not run concurrently: the ONNX session is held behind a mutex, so
    /// the thread budget goes to intra-op parallelism and wall time scales with page count times
    /// per-page inference. Tier choice therefore dominates throughput on multi-page documents.
    pub model_tier: String,

    /// Model generation: `"pp-ocrv6"` (default) or `"pp-ocrv5"`.
    ///
    /// PP-OCRv6 adds a unified CJK+Latin+JA/KO recognition model with `medium`/`small`/`tiny`
    /// tiers (see `model_tier`). Scripts outside the v6 unified coverage (Arabic, Cyrillic,
    /// Devanagari, Greek, Tamil, Telugu, Thai) transparently fall back to the PP-OCRv5
    /// per-script recognition models. Defaults to `"pp-ocrv6"`; the default `model_tier`
    /// (`"mobile"`) resolves to the v6 `"small"` tier. Select `"pp-ocrv5"` to pin the
    /// legacy per-script/unified fleet.
    pub model_version: String,

    /// Explicit inference engine choice.
    ///
    /// `None` (the default) resolves to the compiled default: `ort` when the
    /// `paddle-ocr-ort` feature is compiled in, otherwise `tract`. An explicit choice
    /// is validated against the compiled features when the OCR engine is constructed
    /// (see `crate::paddle_ocr::backend::effective_backend`); requesting an engine
    /// whose feature is not compiled in is a clear configuration error rather than a
    /// silent fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_backend: Option<PaddleInferenceBackend>,
}

const PADDLE_OCR_CONFIG_FIELD_NAMES: &[&str] = &[
    "language",
    "cache_dir",
    "use_angle_cls",
    "enable_table_detection",
    "det_db_thresh",
    "det_db_box_thresh",
    "det_db_unclip_ratio",
    "det_limit_side_len",
    "rec_batch_num",
    "padding",
    "drop_score",
    "model_tier",
    "model_version",
    "inference_backend",
];

const LEGACY_CAMEL_CASE_FIELD_ALIASES: &[(&str, &str)] = &[
    ("cacheDir", "cache_dir"),
    ("useAngleCls", "use_angle_cls"),
    ("enableTableDetection", "enable_table_detection"),
    ("detDbThresh", "det_db_thresh"),
    ("detDbBoxThresh", "det_db_box_thresh"),
    ("detDbUnclipRatio", "det_db_unclip_ratio"),
    ("detLimitSideLen", "det_limit_side_len"),
    ("recBatchNum", "rec_batch_num"),
    ("dropScore", "drop_score"),
    ("modelTier", "model_tier"),
    ("modelVersion", "model_version"),
    ("inferenceBackend", "inference_backend"),
];

/// Which concrete ONNX inference engine PaddleOCR model loading uses.
///
/// Mirrors `sceptre::Backend` for the PaddleOCR backend: `Ort` is the native,
/// full-featured path (acceleration/execution-provider hook, ONNX-embedded
/// dictionary metadata); `Tract` is the pure-Rust, CPU-only path used on targets
/// where `ort` cannot link (Android x86_64 emulator, WASM once wired).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaddleInferenceBackend {
    /// Native ONNX Runtime (requires the `paddle-ocr-ort` feature).
    Ort,
    /// Pure-Rust ONNX via `tract` (requires the `paddle-ocr-tract` feature).
    Tract,
}

impl PaddleOcrConfig {
    /// Deserialize the deprecated raw JSON field while preserving its unknown-key tolerance and
    /// the camelCase keys accepted by the former hand-written Node binding. ~keep
    pub(crate) fn from_legacy_value(value: &serde_json::Value) -> serde_json::Result<Self> {
        let mut value = value.clone();
        if let serde_json::Value::Object(fields) = &mut value {
            for &(alias, canonical) in LEGACY_CAMEL_CASE_FIELD_ALIASES {
                if fields.contains_key(canonical) {
                    fields.remove(alias);
                } else if let Some(value) = fields.remove(alias) {
                    fields.insert(canonical.to_string(), value);
                }
            }
            fields.retain(|name, _| PADDLE_OCR_CONFIG_FIELD_NAMES.contains(&name.as_str()));
        }
        serde_json::from_value(value)
    }

    /// Creates a new PaddleOCR configuration with specified language.
    ///
    /// # Arguments
    ///
    /// * `language` - Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xberg::PaddleOcrConfig;
    ///
    /// let config = PaddleOcrConfig::new("en");
    /// ```
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            cache_dir: None,
            use_angle_cls: false,
            enable_table_detection: false,
            det_db_thresh: 0.3,
            det_db_box_thresh: 0.5,
            det_db_unclip_ratio: 1.6,
            det_limit_side_len: DEFAULT_DETECTION_LIMIT_SIDE_LEN,
            rec_batch_num: DEFAULT_RECOGNITION_BATCH_SIZE,
            padding: 10,
            drop_score: 0.5,
            model_tier: "mobile".to_string(),
            model_version: "pp-ocrv6".to_string(),
            inference_backend: None,
        }
    }

    /// Resolves the Hugging Face Hub cache directory, using an explicit
    /// `cache_dir` when supplied, `XBERG_OCR_MODEL_CACHE_DIR` when non-empty, and the standard
    /// Hugging Face environment conventions otherwise.
    ///
    /// # Returns
    ///
    /// The resolved cache directory path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xberg::PaddleOcrConfig;
    ///
    /// let config = PaddleOcrConfig::new("en");
    /// let cache_dir = config.resolve_cache_dir();
    /// println!("Cache directory: {:?}", cache_dir);
    /// ```
    #[cfg_attr(alef, alef(skip))]
    pub fn resolve_cache_dir(&self) -> PathBuf {
        if let Some(path) = &self.cache_dir {
            return path.clone();
        }

        if let Ok(Some(path)) = crate::cache_dir::ocr_model_cache_override(None) {
            return path;
        }

        Self::default_cache_dir()
    }

    #[cfg(paddle_ocr)]
    pub(crate) fn checked_cache_dir(&self) -> crate::Result<PathBuf> {
        Ok(crate::cache_dir::ocr_model_cache_override(self.cache_dir.as_deref())?
            .unwrap_or_else(Self::default_cache_dir))
    }

    fn default_cache_dir() -> PathBuf {
        // `PaddleOcrConfig` compiles in every build because `OcrConfig` holds it, but `hf_hub`
        // is linked only with the PaddleOCR engine and never on wasm32, so builds without the
        // engine fall back to the shared cache-dir resolver. ~keep
        #[cfg(all(paddle_ocr, not(target_arch = "wasm32")))]
        {
            hf_hub::resolve_cache_dir()
        }
        #[cfg(not(all(paddle_ocr, not(target_arch = "wasm32"))))]
        {
            crate::cache_dir::resolve_cache_dir("paddle-ocr")
        }
    }
}

impl PaddleOcrConfig {
    /// Sets a custom Hugging Face Hub cache root for model files.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to cache directory
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xberg::PaddleOcrConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = PaddleOcrConfig::new("en")
    ///     .with_cache_dir(PathBuf::from("/tmp/paddle-cache"));
    /// ```
    pub fn with_cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = Some(path);
        self
    }

    /// Enables or disables table structure detection.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable table detection
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xberg::PaddleOcrConfig;
    ///
    /// let config = PaddleOcrConfig::new("en")
    ///     .with_table_detection(true);
    /// ```
    pub fn with_table_detection(mut self, enable: bool) -> Self {
        self.enable_table_detection = enable;
        self
    }

    /// Enables or disables angle classification for rotated text.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable angle classification
    pub fn with_angle_cls(mut self, enable: bool) -> Self {
        self.use_angle_cls = enable;
        self
    }

    /// Sets the database threshold for text detection.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Detection threshold (0.0-1.0)
    pub fn with_det_db_thresh(mut self, threshold: f32) -> Self {
        self.det_db_thresh = threshold.clamp(0.0, 1.0);
        self
    }

    /// Sets the box threshold for text bounding box refinement.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Box threshold (0.0-1.0)
    pub fn with_det_db_box_thresh(mut self, threshold: f32) -> Self {
        self.det_db_box_thresh = threshold.clamp(0.0, 1.0);
        self
    }

    /// Sets the unclip ratio for expanding text bounding boxes.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Unclip ratio (typically 1.5-2.0)
    pub fn with_det_db_unclip_ratio(mut self, ratio: f32) -> Self {
        self.det_db_unclip_ratio = ratio.clamp(1.0, 3.0);
        self
    }

    /// Sets the maximum side length for detection images.
    ///
    /// # Arguments
    ///
    /// * `length` - Maximum side length in pixels
    pub fn with_det_limit_side_len(mut self, length: u32) -> Self {
        self.det_limit_side_len = length.clamp(64, 4096);
        self
    }

    /// Sets the batch size for recognition inference.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - Number of text regions to process simultaneously
    pub fn with_rec_batch_num(mut self, batch_size: u32) -> Self {
        self.rec_batch_num = batch_size.clamp(MIN_RECOGNITION_BATCH_SIZE, MAX_RECOGNITION_BATCH_SIZE);
        self
    }

    /// Sets the minimum recognition confidence threshold.
    ///
    /// # Arguments
    ///
    /// * `score` - Minimum confidence (0.0-1.0), text below this is dropped
    pub fn with_drop_score(mut self, score: f32) -> Self {
        self.drop_score = score.clamp(0.0, 1.0);
        self
    }

    /// Sets padding in pixels added around images before detection.
    ///
    /// # Arguments
    ///
    /// * `padding` - Padding in pixels (0-100)
    pub fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding.clamp(0, 100);
        self
    }

    /// Sets the model tier controlling detection/recognition model size.
    ///
    /// # Arguments
    ///
    /// * `tier` - `"mobile"` (default, lightweight, faster) or `"server"` (high accuracy, GPU/complex documents)
    pub fn with_model_tier(mut self, tier: impl Into<String>) -> Self {
        self.model_tier = tier.into();
        self
    }

    /// Sets the model generation.
    ///
    /// # Arguments
    ///
    /// * `version` - `"pp-ocrv6"` (default) or `"pp-ocrv5"`. Under `"pp-ocrv6"`, `model_tier`
    ///   selects among `"medium"`/`"small"`/`"tiny"`.
    pub fn with_model_version(mut self, version: impl Into<String>) -> Self {
        self.model_version = version.into();
        self
    }
}

impl Default for PaddleOcrConfig {
    /// Creates a default configuration with English language support.
    fn default() -> Self {
        Self::new("en")
    }
}

/// Supported languages in PaddleOCR.
///
/// Maps user-friendly language codes to paddle-ocr-rs language identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaddleLanguage {
    /// English
    English,
    /// Simplified Chinese
    Chinese,
    /// Japanese
    Japanese,
    /// Korean
    Korean,
    /// German
    German,
    /// French
    French,
    /// Latin script (covers most European languages)
    Latin,
    /// Cyrillic (Russian and related)
    Cyrillic,
    /// Traditional Chinese
    TraditionalChinese,
    /// Thai
    Thai,
    /// Greek
    Greek,
    /// East Slavic (Russian, Ukrainian, Belarusian)
    EastSlavic,
    /// Arabic (Arabic, Persian, Urdu)
    Arabic,
    /// Devanagari (Hindi, Marathi, Sanskrit, Nepali)
    Devanagari,
    /// Tamil
    Tamil,
    /// Telugu
    Telugu,
}

impl PaddleLanguage {
    /// Converts to the language code string.
    ///
    /// # Returns
    ///
    /// Language code as used by paddle-ocr-rs
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg::PaddleLanguage;
    ///
    /// assert_eq!(PaddleLanguage::English.code(), "en");
    /// assert_eq!(PaddleLanguage::Chinese.code(), "ch");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Chinese => "ch",
            Self::Japanese => "jpn",
            Self::Korean => "kor",
            Self::German => "deu",
            Self::French => "fra",
            Self::Latin => "latin",
            Self::Cyrillic => "cyrillic",
            Self::TraditionalChinese => "chinese_cht",
            Self::Thai => "thai",
            Self::Greek => "greek",
            Self::EastSlavic => "eslav",
            Self::Arabic => "arabic",
            Self::Devanagari => "devanagari",
            Self::Tamil => "tamil",
            Self::Telugu => "telugu",
        }
    }

    /// Parses a language code string to `PaddleLanguage`.
    ///
    /// # Arguments
    ///
    /// * `code` - Language code string
    ///
    /// # Returns
    ///
    /// `Some(PaddleLanguage)` if the code is recognized, `None` otherwise
    #[cfg(test)]
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::English),
            "ch" => Some(Self::Chinese),
            "jpn" => Some(Self::Japanese),
            "kor" => Some(Self::Korean),
            "deu" => Some(Self::German),
            "fra" => Some(Self::French),
            "latin" => Some(Self::Latin),
            "cyrillic" => Some(Self::Cyrillic),
            "chinese_cht" => Some(Self::TraditionalChinese),
            "thai" => Some(Self::Thai),
            "greek" => Some(Self::Greek),
            "eslav" => Some(Self::EastSlavic),
            "arabic" => Some(Self::Arabic),
            "devanagari" => Some(Self::Devanagari),
            "tamil" => Some(Self::Tamil),
            "telugu" => Some(Self::Telugu),
            _ => None,
        }
    }
}

impl std::fmt::Display for PaddleLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::English => write!(f, "English"),
            Self::Chinese => write!(f, "Chinese"),
            Self::Japanese => write!(f, "Japanese"),
            Self::Korean => write!(f, "Korean"),
            Self::German => write!(f, "German"),
            Self::French => write!(f, "French"),
            Self::Latin => write!(f, "Latin"),
            Self::Cyrillic => write!(f, "Cyrillic"),
            Self::TraditionalChinese => write!(f, "Traditional Chinese"),
            Self::Thai => write!(f, "Thai"),
            Self::Greek => write!(f, "Greek"),
            Self::EastSlavic => write!(f, "East Slavic"),
            Self::Arabic => write!(f, "Arabic"),
            Self::Devanagari => write!(f, "Devanagari"),
            Self::Tamil => write!(f, "Tamil"),
            Self::Telugu => write!(f, "Telugu"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let config = PaddleOcrConfig::new("en");
        assert_eq!(config.language, "en");
        assert!(!config.use_angle_cls);
        assert!(
            !config.enable_table_detection,
            "paddle cannot tell a table from prose, so this stays off until it can -- see backend tests"
        );
        assert_eq!(config.padding, 10);
        assert_eq!(config.model_tier, "mobile");
        assert_eq!(config.model_version, "pp-ocrv6");
    }

    #[test]
    fn test_default_config() {
        let config = PaddleOcrConfig::default();
        assert_eq!(config.language, "en");
        assert_eq!(config.det_db_thresh, 0.3);
        assert_eq!(config.det_db_box_thresh, 0.5);
        assert_eq!(config.det_db_unclip_ratio, 1.6);
        assert_eq!(config.det_limit_side_len, 1024);
        assert_eq!(config.rec_batch_num, 6);
        assert_eq!(config.padding, 10);
        assert_eq!(config.model_tier, "mobile");
    }

    #[test]
    fn test_builder_pattern() {
        let config = PaddleOcrConfig::new("ch")
            .with_angle_cls(true)
            .with_table_detection(true)
            .with_det_db_thresh(0.4)
            .with_rec_batch_num(12)
            .with_padding(25);

        assert_eq!(config.language, "ch");
        assert!(config.use_angle_cls);
        assert!(config.enable_table_detection);
        assert_eq!(config.det_db_thresh, 0.4);
        assert_eq!(config.rec_batch_num, 12);
        assert_eq!(config.padding, 25);
    }

    #[test]
    fn test_with_cache_dir() {
        let cache_path = PathBuf::from("/tmp/cache");
        let config = PaddleOcrConfig::new("en").with_cache_dir(cache_path.clone());
        assert_eq!(config.cache_dir, Some(cache_path));
    }

    #[test]
    fn test_deserialize_rejects_camel_case_key() {
        let err = serde_json::from_value::<PaddleOcrConfig>(serde_json::json!({"enableTableDetection": true}))
            .expect_err("a camelCase key must not be dropped silently");
        assert!(err.to_string().contains("enableTableDetection"), "{err}");
    }

    #[test]
    fn should_preserve_every_known_field_while_ignoring_legacy_extensions() {
        let expected = PaddleOcrConfig {
            language: "deu".to_string(),
            cache_dir: Some(PathBuf::from("/tmp/paddle-cache")),
            use_angle_cls: true,
            enable_table_detection: true,
            det_db_thresh: 0.4,
            det_db_box_thresh: 0.6,
            det_db_unclip_ratio: 1.8,
            det_limit_side_len: 1536,
            rec_batch_num: 12,
            padding: 24,
            drop_score: 0.7,
            model_tier: "server".to_string(),
            model_version: "pp-ocrv5".to_string(),
            inference_backend: Some(PaddleInferenceBackend::Tract),
        };
        let mut legacy = serde_json::to_value(&expected).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .insert("vendor_extension".to_string(), serde_json::json!({"enabled": true}));

        let resolved = PaddleOcrConfig::from_legacy_value(&legacy).unwrap();

        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_deserialize_snake_case_keys() {
        let config: PaddleOcrConfig = serde_json::from_value(serde_json::json!({
            "use_angle_cls": true,
            "enable_table_detection": true,
            "model_tier": "server",
        }))
        .unwrap();
        assert!(config.use_angle_cls);
        assert!(config.enable_table_detection);
        assert_eq!(config.model_tier, "server");
        assert_eq!(config.language, "en");
    }

    #[test]
    fn test_resolve_cache_dir_explicit() {
        let cache_path = PathBuf::from("/tmp/explicit");
        let config = PaddleOcrConfig::new("en").with_cache_dir(cache_path.clone());
        assert_eq!(config.resolve_cache_dir(), cache_path);
    }

    #[cfg(all(paddle_ocr, not(target_arch = "wasm32")))]
    #[test]
    fn test_resolve_cache_dir_default() {
        let config = PaddleOcrConfig::new("en");
        assert_eq!(config.resolve_cache_dir(), hf_hub::resolve_cache_dir());
    }

    #[test]
    fn test_paddle_language_code() {
        assert_eq!(PaddleLanguage::English.code(), "en");
        assert_eq!(PaddleLanguage::Chinese.code(), "ch");
        assert_eq!(PaddleLanguage::Japanese.code(), "jpn");
        assert_eq!(PaddleLanguage::Korean.code(), "kor");
        assert_eq!(PaddleLanguage::German.code(), "deu");
        assert_eq!(PaddleLanguage::French.code(), "fra");
    }

    #[test]
    fn test_paddle_language_from_code() {
        assert_eq!(PaddleLanguage::from_code("en"), Some(PaddleLanguage::English));
        assert_eq!(PaddleLanguage::from_code("ch"), Some(PaddleLanguage::Chinese));
        assert_eq!(PaddleLanguage::from_code("jpn"), Some(PaddleLanguage::Japanese));
        assert_eq!(PaddleLanguage::from_code("kor"), Some(PaddleLanguage::Korean));
        assert_eq!(PaddleLanguage::from_code("deu"), Some(PaddleLanguage::German));
        assert_eq!(PaddleLanguage::from_code("fra"), Some(PaddleLanguage::French));
        assert_eq!(PaddleLanguage::from_code("unknown"), None);
    }

    #[test]
    fn test_paddle_language_display() {
        assert_eq!(PaddleLanguage::English.to_string(), "English");
        assert_eq!(PaddleLanguage::Chinese.to_string(), "Chinese");
        assert_eq!(PaddleLanguage::Japanese.to_string(), "Japanese");
    }

    #[test]
    fn test_threshold_values() {
        let config = PaddleOcrConfig::new("en")
            .with_det_db_thresh(0.25)
            .with_det_db_box_thresh(0.6)
            .with_det_db_unclip_ratio(1.8);

        assert_eq!(config.det_db_thresh, 0.25);
        assert_eq!(config.det_db_box_thresh, 0.6);
        assert_eq!(config.det_db_unclip_ratio, 1.8);
    }

    #[test]
    fn test_side_length_and_batch() {
        let config = PaddleOcrConfig::new("en")
            .with_det_limit_side_len(1280)
            .with_rec_batch_num(8);

        assert_eq!(config.det_limit_side_len, 1280);
        assert_eq!(config.rec_batch_num, 8);
    }

    #[test]
    fn should_default_detection_side_length_when_omitted() {
        let config: PaddleOcrConfig = serde_json::from_str(r#"{"language":"en"}"#).unwrap();
        assert_eq!(config.det_limit_side_len, 1024);
    }

    #[test]
    fn should_preserve_explicit_detection_side_length_override() {
        let config: PaddleOcrConfig = serde_json::from_str(r#"{"language":"en","det_limit_side_len":960}"#).unwrap();
        assert_eq!(config.det_limit_side_len, 960);
    }

    #[test]
    fn test_serialization() {
        let config = PaddleOcrConfig::new("ch")
            .with_table_detection(true)
            .with_angle_cls(false);

        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: PaddleOcrConfig = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.language, config.language);
        assert_eq!(deserialized.enable_table_detection, config.enable_table_detection);
        assert_eq!(deserialized.use_angle_cls, config.use_angle_cls);
        assert_eq!(deserialized.model_tier, config.model_tier);
    }

    #[test]
    fn test_model_tier_builder() {
        let config = PaddleOcrConfig::new("en").with_model_tier("server");
        assert_eq!(config.model_tier, "server");
    }

    #[test]
    fn test_model_version_builder() {
        let config = PaddleOcrConfig::new("en")
            .with_model_version("pp-ocrv6")
            .with_model_tier("small");
        assert_eq!(config.model_version, "pp-ocrv6");
        assert_eq!(config.model_tier, "small");
    }

    #[test]
    fn test_model_version_serde_roundtrip() {
        let config = PaddleOcrConfig::new("ch").with_model_version("pp-ocrv6");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"model_version\":\"pp-ocrv6\""));

        let deserialized: PaddleOcrConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model_version, "pp-ocrv6");
    }

    #[test]
    fn test_model_version_defaults_when_omitted() {
        let json = r#"{"language":"en","model_tier":"mobile"}"#;
        let config: PaddleOcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.model_version, "pp-ocrv6");
    }

    #[test]
    fn test_model_version_pins_legacy_v5_when_requested() {
        let json = r#"{"language":"en","model_tier":"mobile","model_version":"pp-ocrv5"}"#;
        let config: PaddleOcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.model_version, "pp-ocrv5");
    }

    #[test]
    fn test_model_tier_serde_roundtrip() {
        let config = PaddleOcrConfig::new("ch").with_model_tier("server");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"model_tier\":\"server\""));

        let deserialized: PaddleOcrConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model_tier, "server");
    }

    #[test]
    fn test_model_tier_backward_compat() {
        let json = r#"{"language":"en","det_db_thresh":0.3}"#;
        let config: PaddleOcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.model_tier, "mobile");
    }
}
