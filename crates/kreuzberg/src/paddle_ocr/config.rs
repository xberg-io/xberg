//! Configuration for PaddleOCR backend via ONNX Runtime.
//!
//! This module provides comprehensive configuration for PaddleOCR text detection, angle
//! classification, and recognition. Supports multi-language OCR with customizable detection
//! and recognition thresholds.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for PaddleOCR backend.
///
/// Configures PaddleOCR text detection and recognition with multi-language support.
/// Uses a builder pattern for convenient configuration.
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::ocr::paddle::PaddleOcrConfig;
///
/// // Create with default English configuration
/// let config = PaddleOcrConfig::new("en");
///
/// // Create with custom cache directory
/// let config = PaddleOcrConfig::new("ch")
///     .with_cache_dir("/path/to/cache".into());
///
/// // Enable table detection
/// let config = PaddleOcrConfig::new("en")
///     .with_table_detection(true);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddleOcrConfig {
    /// Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra")
    pub language: String,

    /// Optional custom cache directory for model files
    pub cache_dir: Option<PathBuf>,

    /// Enable angle classification for rotated text (default: true)
    pub use_angle_cls: bool,

    /// Enable table structure detection (default: false)
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

    /// Maximum side length for detection image (default: 960)
    /// Larger images may be resized to this limit for faster inference
    pub det_limit_side_len: u32,

    /// Batch size for recognition inference (default: 6)
    /// Number of text regions to process simultaneously
    pub rec_batch_num: u32,
}

impl PaddleOcrConfig {
    /// Creates a new PaddleOCR configuration with specified language.
    ///
    /// # Arguments
    ///
    /// * `language` - Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ocr::paddle::PaddleOcrConfig;
    ///
    /// let config = PaddleOcrConfig::new("en");
    /// ```
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            cache_dir: None,
            use_angle_cls: true,
            enable_table_detection: false,
            det_db_thresh: 0.3,
            det_db_box_thresh: 0.5,
            det_db_unclip_ratio: 1.6,
            det_limit_side_len: 960,
            rec_batch_num: 6,
        }
    }

    /// Sets a custom cache directory for model files.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to cache directory
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ocr::paddle::PaddleOcrConfig;
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
    /// use kreuzberg::ocr::paddle::PaddleOcrConfig;
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
        self.det_db_thresh = threshold;
        self
    }

    /// Sets the box threshold for text bounding box refinement.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Box threshold (0.0-1.0)
    pub fn with_det_db_box_thresh(mut self, threshold: f32) -> Self {
        self.det_db_box_thresh = threshold;
        self
    }

    /// Sets the unclip ratio for expanding text bounding boxes.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Unclip ratio (typically 1.5-2.0)
    pub fn with_det_db_unclip_ratio(mut self, ratio: f32) -> Self {
        self.det_db_unclip_ratio = ratio;
        self
    }

    /// Sets the maximum side length for detection images.
    ///
    /// # Arguments
    ///
    /// * `length` - Maximum side length in pixels
    pub fn with_det_limit_side_len(mut self, length: u32) -> Self {
        self.det_limit_side_len = length;
        self
    }

    /// Sets the batch size for recognition inference.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - Number of text regions to process simultaneously
    pub fn with_rec_batch_num(mut self, batch_size: u32) -> Self {
        self.rec_batch_num = batch_size;
        self
    }

    /// Resolves the cache directory, checking in order:
    /// 1. Configured `cache_dir` if set
    /// 2. `KREUZBERG_PADDLE_CACHE_DIR` environment variable
    /// 3. Default: `~/.cache/kreuzberg/paddle-ocr/`
    ///
    /// # Returns
    ///
    /// The resolved cache directory path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ocr::paddle::PaddleOcrConfig;
    ///
    /// let config = PaddleOcrConfig::new("en");
    /// let cache_dir = config.resolve_cache_dir();
    /// println!("Cache directory: {:?}", cache_dir);
    /// ```
    pub fn resolve_cache_dir(&self) -> PathBuf {
        // First check if cache_dir is explicitly set
        if let Some(path) = &self.cache_dir {
            return path.clone();
        }

        // Check environment variable
        if let Ok(env_path) = std::env::var("KREUZBERG_PADDLE_CACHE_DIR") {
            return PathBuf::from(env_path);
        }

        // Default to ~/.cache/kreuzberg/paddle-ocr/
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kreuzberg")
            .join("paddle-ocr")
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// use kreuzberg::ocr::paddle::PaddleLanguage;
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
    ///
    /// # Examples
    ///
    /// ```
    /// use kreuzberg::ocr::paddle::PaddleLanguage;
    ///
    /// assert_eq!(PaddleLanguage::from_code("en"), Some(PaddleLanguage::English));
    /// assert_eq!(PaddleLanguage::from_code("ch"), Some(PaddleLanguage::Chinese));
    /// assert_eq!(PaddleLanguage::from_code("unknown"), None);
    /// ```
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::English),
            "ch" => Some(Self::Chinese),
            "jpn" => Some(Self::Japanese),
            "kor" => Some(Self::Korean),
            "deu" => Some(Self::German),
            "fra" => Some(Self::French),
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
        assert!(config.use_angle_cls);
        assert!(!config.enable_table_detection);
    }

    #[test]
    fn test_default_config() {
        let config = PaddleOcrConfig::default();
        assert_eq!(config.language, "en");
        assert_eq!(config.det_db_thresh, 0.3);
        assert_eq!(config.det_db_box_thresh, 0.5);
        assert_eq!(config.det_db_unclip_ratio, 1.6);
        assert_eq!(config.det_limit_side_len, 960);
        assert_eq!(config.rec_batch_num, 6);
    }

    #[test]
    fn test_builder_pattern() {
        let config = PaddleOcrConfig::new("ch")
            .with_angle_cls(false)
            .with_table_detection(true)
            .with_det_db_thresh(0.4)
            .with_rec_batch_num(12);

        assert_eq!(config.language, "ch");
        assert!(!config.use_angle_cls);
        assert!(config.enable_table_detection);
        assert_eq!(config.det_db_thresh, 0.4);
        assert_eq!(config.rec_batch_num, 12);
    }

    #[test]
    fn test_with_cache_dir() {
        let cache_path = PathBuf::from("/tmp/cache");
        let config = PaddleOcrConfig::new("en").with_cache_dir(cache_path.clone());
        assert_eq!(config.cache_dir, Some(cache_path));
    }

    #[test]
    fn test_resolve_cache_dir_explicit() {
        let cache_path = PathBuf::from("/tmp/explicit");
        let config = PaddleOcrConfig::new("en").with_cache_dir(cache_path.clone());
        assert_eq!(config.resolve_cache_dir(), cache_path);
    }

    #[test]
    fn test_resolve_cache_dir_default() {
        let config = PaddleOcrConfig::new("en");
        let cache_dir = config.resolve_cache_dir();
        // Should contain "kreuzberg" and "paddle-ocr" in the path
        assert!(cache_dir.to_string_lossy().contains("kreuzberg"));
        assert!(cache_dir.to_string_lossy().contains("paddle-ocr"));
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
    fn test_serialization() {
        let config = PaddleOcrConfig::new("ch")
            .with_table_detection(true)
            .with_angle_cls(false);

        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: PaddleOcrConfig = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.language, config.language);
        assert_eq!(deserialized.enable_table_detection, config.enable_table_detection);
        assert_eq!(deserialized.use_angle_cls, config.use_angle_cls);
    }
}
