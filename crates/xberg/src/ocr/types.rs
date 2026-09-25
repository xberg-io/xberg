use serde::{Deserialize, Serialize};

pub use crate::types::ImagePreprocessingConfig;

/// Page Segmentation Mode for Tesseract OCR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PSMMode {
    /// Orientation and script detection only.
    OsdOnly = 0,
    /// Automatic page segmentation with OSD.
    AutoOsd = 1,
    /// Automatic page segmentation without OSD or OCR.
    AutoOnly = 2,
    /// Fully automatic page segmentation with no OSD (default).
    Auto = 3,
    /// Assume a single column of text of variable sizes.
    SingleColumn = 4,
    /// Assume a single uniform block of vertically aligned text.
    SingleBlockVertical = 5,
    /// Assume a single uniform block of text.
    SingleBlock = 6,
    /// Treat the image as a single text line.
    SingleLine = 7,
    /// Treat the image as a single word.
    SingleWord = 8,
    /// Treat the image as a single word in a circle.
    CircleWord = 9,
    /// Treat the image as a single character.
    SingleChar = 10,
}

#[cfg(test)]
impl PSMMode {
    pub(crate) fn from_u8(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(PSMMode::OsdOnly),
            1 => Ok(PSMMode::AutoOsd),
            2 => Ok(PSMMode::AutoOnly),
            3 => Ok(PSMMode::Auto),
            4 => Ok(PSMMode::SingleColumn),
            5 => Ok(PSMMode::SingleBlockVertical),
            6 => Ok(PSMMode::SingleBlock),
            7 => Ok(PSMMode::SingleLine),
            8 => Ok(PSMMode::SingleWord),
            9 => Ok(PSMMode::CircleWord),
            10 => Ok(PSMMode::SingleChar),
            _ => Err(format!("Invalid PSM mode value: {}", value)),
        }
    }

    pub(crate) fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Configuration for Tesseract OCR (internal, efficient types).
///
/// This is the internal representation used by the OCR processor.
/// Public API uses i32 for PyO3 compatibility, converted to u8 here for efficiency.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesseractConfig {
    /// Tesseract language code (e.g. `"eng"`, `"deu"`, `"eng+fra"`).
    pub language: String,
    /// Page Segmentation Mode as a raw `u8` (see [`PSMMode`]).
    pub psm: u8,
    /// OCR output format: `"text"`, `"markdown"`, `"hocr"`, or `"tsv"`.
    pub output_format: String,
    /// OCR Engine Mode (0 = Legacy, 1 = LSTM, 2 = Both, 3 = Default).
    pub oem: u8,
    /// Minimum word confidence threshold (0.0–100.0); words below are dropped.
    ///
    /// Applied per WORD, not per page (see `parse_tsv_to_elements` and
    /// `extract_elements_via_iterator` in `ocr::processor::execution`): an individual
    /// low-confidence word is dropped from the output while the rest of the page's words
    /// are kept. A floor set too high can still empty an entire page one word at a time,
    /// which is observably identical to a page-level drop, so this is not a "safe by
    /// construction" knob. See [`Self::default`] for why this stays at `0.0` and what
    /// measurement is owed before raising it.
    pub min_confidence: f64,
    /// Optional image preprocessing applied before recognition.
    pub preprocessing: Option<ImagePreprocessingConfig>,
    /// Whether to attempt table detection from hOCR/TSV output.
    pub enable_table_detection: bool,
    /// Minimum confidence for cells included in reconstructed tables.
    pub table_min_confidence: f64,
    /// Pixel threshold for grouping words into columns.
    pub table_column_threshold: u32,
    /// Fraction of column height a word must span to count as a row separator.
    pub table_row_threshold_ratio: f64,
    /// Whether to use the on-disk OCR result cache.
    pub use_cache: bool,
    /// Tesseract `classify_use_pre_adapted_templates` variable.
    pub classify_use_pre_adapted_templates: bool,
    /// Tesseract `language_model_ngram_on` variable.
    ///
    /// Enables Tesseract's character n-gram language model, which penalizes output that
    /// does not look like a word of the target language even when the classifier itself
    /// was confident about individual glyphs. This is the dominant failure mode this crate
    /// has measured for scanned line art (survey plats, engineering drawings, signature
    /// flourishes): the engine reads confident-looking non-words such as `LAAALDLI` that
    /// downstream heuristics only partially catch (see `is_ocr_recognition_noise` in
    /// `extractors::pdf::ocr`). Left off, Tesseract does not apply this penalty at all.
    /// Kept on by default rather than off, unlike upstream's own default, because prose
    /// pages are unaffected while noise pages are meaningfully suppressed.
    ///
    /// **This default must stay in sync with
    /// [`crate::types::TesseractConfig::language_model_ngram_on`]'s default** (the
    /// public-facing counterpart, in `types/formats.rs`). The two structs have independent
    /// `Default` impls, and several call sites in `extractors::image` construct the public
    /// struct's default and convert it (via the `From` impl below) *before* this default is
    /// ever consulted — so this field's value only reaches standalone image OCR when both
    /// defaults agree.
    pub language_model_ngram_on: bool,
    /// Tesseract `tessedit_dont_blkrej_good_wds` variable.
    pub tessedit_dont_blkrej_good_wds: bool,
    /// Tesseract `tessedit_dont_rowrej_good_wds` variable.
    pub tessedit_dont_rowrej_good_wds: bool,
    /// Tesseract `tessedit_enable_dict_correction` variable.
    pub tessedit_enable_dict_correction: bool,
    /// Restrict recognized characters to this set (empty = unrestricted).
    pub tessedit_char_whitelist: String,
    /// Exclude these characters from recognition (empty = none excluded).
    pub tessedit_char_blacklist: String,
    /// Tesseract `tessedit_use_primary_params_model` variable.
    pub tessedit_use_primary_params_model: bool,
    /// Tesseract `textord_space_size_is_variable` variable.
    pub textord_space_size_is_variable: bool,
    /// Tesseract `thresholding_method` variable, by name; see [`ThresholdingMethod`].
    pub thresholding_method: String,

    /// Enable automatic page rotation based on orientation detection.
    ///
    /// When enabled (and the `auto-rotate` build feature is compiled in), page
    /// orientation (0/90/180/270 degrees) is detected with an ONNX PP-LCNet
    /// document-orientation classifier — NOT Tesseract's own
    /// `DetectOrientationScript()`/OSD, which this crate does not call. The
    /// classifier reports only a rotation angle and confidence; it has no
    /// script-detection capability, so no script name is produced. If the page
    /// is rotated with high confidence, the image is corrected before
    /// recognition.
    pub auto_rotate: bool,

    /// Highest-priority override for the tessdata directory.
    ///
    /// When set, [`resolve_tessdata_path`](crate::ocr::processor) uses this path
    /// before consulting `TESSDATA_PREFIX`, cache, or system locations.
    pub tessdata_path: Option<std::path::PathBuf>,

    /// True resolution, in DPI, of the image bytes accompanying this config, when the caller
    /// knows it.
    ///
    /// This is *not* a user-facing knob and has no counterpart on the public
    /// [`crate::types::TesseractConfig`]: it is a per-call fact about one image, set by the PDF
    /// OCR route from [`crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION`] because that route
    /// rendered the page and can derive it exactly. Every other caller leaves it `None`.
    ///
    /// `None` means "unknown", and the DPI-normalization step then falls back to assuming 72 DPI
    /// as it always has. That assumption is what this field exists to displace: a page rendered
    /// at 150 DPI but declared as 72 gets scaled by `target_dpi / 72` instead of
    /// `target_dpi / 150`, which on a Letter page means upscaling into the 4096px dimension clamp
    /// for no added information and then telling Tesseract a `scan_res` that is roughly half the
    /// raster's real resolution.
    #[serde(default)]
    pub source_dpi: Option<f64>,

    /// The true, 1-indexed page number of the source document that `image_bytes` came
    /// from, when the caller knows it.
    ///
    /// Like `source_dpi`, this is *not* a user-facing knob and has no counterpart on the
    /// public [`crate::types::TesseractConfig`]: `perform_ocr` runs Tesseract on exactly one
    /// loaded image per call, so Tesseract's own per-call page index (hOCR's `ppageno`, the
    /// TSV `page_num` column, and the iterator extraction's page argument) is always `0`/`1`
    /// regardless of which page of the source document this image actually is. Every parsed
    /// element, table, and `OcrElement` is stamped with this field's value instead of that
    /// per-call index, so a caller processing page 2 of a multi-page document must set this
    /// to `2` for the resulting elements to report the right page. Callers that don't know
    /// (or don't need) the true page number leave it at the default, `1`.
    #[serde(default = "default_page_number")]
    pub page_number: u32,

    /// Security limits for the image decode this OCR call performs.
    ///
    /// Carried from `OcrConfig::security_limits`, which the caller's `ExtractionConfig`
    /// populates before dispatch. Before GH#1651 this type had no such field, so
    /// `config_to_tesseract` had nothing to copy into and every Tesseract decode ran under
    /// `SecurityLimits::default()` on every route -- a caller who raised the limit to admit
    /// a large scan was still refused at 100 MiB.
    ///
    /// `#[serde(skip)]` because it is injected at runtime, never read from a config file,
    /// and deliberately absent from `hash_config` (`ocr::processor::config`): limits gate
    /// whether a decode is attempted and never change the text Tesseract produces, so
    /// folding them into the cache key would split cache entries that are identical in
    /// content. `None` means `SecurityLimits::default()`, never "disable the check". ~keep
    #[serde(skip)]
    pub security_limits: Option<crate::extractors::security::SecurityLimits>,
}

/// Default for [`TesseractConfig::page_number`]: page 1, matching Tesseract's own
/// single-image `ppageno`/TSV convention when no caller-supplied page number is known.
fn default_page_number() -> u32 {
    1
}

/// Word-level confidence floor (0.0-100.0) below which Tesseract drops a recognized word.
/// `0.0` accepts every word Tesseract reports, regardless of confidence.
///
/// Calibration owed before this can safely move above `0.0`: unlike
/// `max_ocr_output_fragmented_word_ratio` / `min_ocr_mean_confidence` in
/// `OcrQualityThresholds` (measured as PAGE-level statistics over a recorded municipal
/// ordinance), this floor is applied per WORD, so the required measurement is different
/// in kind, not just in corpus: for each word Tesseract emits, cross-tabulate its raw
/// `conf` value (TSV column 11 / the iterator's per-word confidence) against whether the
/// word is genuinely correct — e.g. using ground truth, or the dictionary-validity signal
/// in `dictionary_invalid_word_ratio` (`ocr::processor::execution`) as a proxy — and find
/// the confidence value below which words are predominantly wrong. Do not raise this
/// constant without that measurement: guessing a page-level number (like 70.0) and
/// applying it per word risks silently deleting individual correct words throughout
/// otherwise-good pages, which is a different and less visible failure than dropping a
/// whole bad page.
const MIN_CONFIDENCE_FLOOR_DEFAULT: f64 = 0.0;

/// Engine-facing PSM used when the public `types::formats::TesseractConfig::psm` is
/// `None` (no explicit caller choice) and no pipeline-level default (whole-image,
/// vertical-language, layout-region, sparse-retry) applied one either — see #1573.
/// Keep in sync with the platform split documented on `types::formats::TesseractConfig::psm`.
#[cfg(target_arch = "wasm32")]
const DEFAULT_ENGINE_PSM: u8 = 6;
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_ENGINE_PSM: u8 = 3;

impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: DEFAULT_ENGINE_PSM,
            output_format: "markdown".to_string(),
            oem: 3,
            min_confidence: MIN_CONFIDENCE_FLOOR_DEFAULT,
            preprocessing: None,
            enable_table_detection: true,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_char_blacklist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: "otsu".to_string(),
            auto_rotate: false,
            tessdata_path: None,
            source_dpi: None,
            page_number: default_page_number(),
            security_limits: None,
        }
    }
}

/// Tesseract's `thresholding_method` parameter (xberg-io/xberg#1784).
///
/// The engine reads the variable as an integer: 0 Otsu, 1 LeptonicaOtsu, 2 Sauvola. A value
/// that does not parse leaves it at 0, and `SetVariable` still reports success for any known
/// variable name, so the value has to be checked here, before the engine sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThresholdingMethod {
    Otsu,
    LeptonicaOtsu,
    Sauvola,
}

impl ThresholdingMethod {
    /// The accepted names, in the order the engine numbers them.
    pub(crate) const NAMES: [&'static str; 3] = ["otsu", "leptonica_otsu", "sauvola"];

    /// The method for a config value, or `None` for a name the engine has no number for.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "otsu" => Some(Self::Otsu),
            "leptonica_otsu" => Some(Self::LeptonicaOtsu),
            "sauvola" => Some(Self::Sauvola),
            _ => None,
        }
    }

    /// The integer Tesseract parses for this method, as the string `SetVariable` takes.
    pub(crate) fn tesseract_value(self) -> &'static str {
        match self {
            Self::Otsu => "0",
            Self::LeptonicaOtsu => "1",
            Self::Sauvola => "2",
        }
    }
}

impl TesseractConfig {
    #[cfg(feature = "ocr")]
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !matches!(self.output_format.as_str(), "text" | "markdown" | "hocr" | "tsv") {
            return Err(format!(
                "Invalid output_format: '{}'. Must be one of: text, markdown, hocr, tsv",
                self.output_format
            ));
        }
        if ThresholdingMethod::parse(&self.thresholding_method).is_none() {
            return Err(format!(
                "Invalid thresholding_method: '{}'. Must be one of: {}",
                self.thresholding_method,
                ThresholdingMethod::NAMES.join(", ")
            ));
        }
        if let Some(preprocessing) = &self.preprocessing {
            crate::core::config_validation::validate_image_preprocessing_config(preprocessing).map_err(|error| {
                if let crate::XbergError::Validation { message, .. } = error {
                    message
                } else {
                    error.to_string()
                }
            })?;
        }
        Ok(())
    }
}

/// Convert from public API TesseractConfig to internal OCR TesseractConfig.
///
/// This conversion handles type differences (i32 → u8/u32) and clones
/// necessary fields. The public API uses i32 for PyO3 compatibility,
/// while the internal representation uses more efficient types.
impl From<&crate::types::TesseractConfig> for TesseractConfig {
    fn from(config: &crate::types::TesseractConfig) -> Self {
        Self {
            psm: config.psm.map(|psm| psm as u8).unwrap_or(DEFAULT_ENGINE_PSM),
            language: config.language.join("+"),
            output_format: config.output_format.clone(),
            oem: config.oem as u8,
            min_confidence: config.min_confidence,
            preprocessing: config.preprocessing.clone(),
            enable_table_detection: config.enable_table_detection,
            table_min_confidence: config.table_min_confidence,
            table_column_threshold: config.table_column_threshold as u32,
            table_row_threshold_ratio: config.table_row_threshold_ratio,
            use_cache: config.use_cache,
            classify_use_pre_adapted_templates: config.classify_use_pre_adapted_templates,
            language_model_ngram_on: config.language_model_ngram_on,
            tessedit_dont_blkrej_good_wds: config.tessedit_dont_blkrej_good_wds,
            tessedit_dont_rowrej_good_wds: config.tessedit_dont_rowrej_good_wds,
            tessedit_enable_dict_correction: config.tessedit_enable_dict_correction,
            tessedit_char_whitelist: config.tessedit_char_whitelist.clone(),
            tessedit_char_blacklist: config.tessedit_char_blacklist.clone(),
            tessedit_use_primary_params_model: config.tessedit_use_primary_params_model,
            textord_space_size_is_variable: config.textord_space_size_is_variable,
            thresholding_method: config.thresholding_method.clone(),
            auto_rotate: config.preprocessing.as_ref().map(|p| p.auto_rotate).unwrap_or(false),
            tessdata_path: None,
            // The public config is a user-supplied document-wide setting and cannot know the
            // resolution of any one image; only the per-call `backend_options` hint can.
            source_dpi: None,
            // Same rationale as `source_dpi`: the public config has no notion of which page
            // of a document this one call is for. Unlike `source_dpi`, no caller currently
            // threads a per-call value in through `backend_options`, so this always resolves
            // to the default; direct callers of the internal `TesseractConfig`/`perform_ocr`
            // API can still set it explicitly.
            page_number: default_page_number(),
            security_limits: None,
        }
    }
}

/// OCR extraction result returned by the internal OCR processor.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted text content (plain text, Markdown, or hOCR depending on `output_format`).
    pub content: String,
    /// MIME type of the source image.
    pub mime_type: String,
    /// Additional metadata key-value pairs (e.g. page count, engine version).
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Tables reconstructed from the OCR output.
    pub tables: Vec<Table>,
}

/// A table reconstructed from OCR output (hOCR or TSV word bounding boxes).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Cell text as a 2D grid (rows × columns).
    pub cells: Vec<Vec<String>>,
    /// Markdown-formatted table string.
    pub markdown: String,
    /// Zero-based page index this table was found on.
    pub page_number: i32,
}

/// Result for a single item in a batch OCR operation.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult {
    /// Path to the source file that was processed.
    pub file_path: String,
    /// Whether OCR succeeded for this file.
    pub success: bool,
    /// Extraction result, present when `success` is `true`.
    pub result: Option<crate::types::OcrExtractionResult>,
    /// Error message, present when `success` is `false`.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psm_mode_from_u8_valid() {
        let modes = [
            (0, PSMMode::OsdOnly),
            (1, PSMMode::AutoOsd),
            (2, PSMMode::AutoOnly),
            (3, PSMMode::Auto),
            (4, PSMMode::SingleColumn),
            (5, PSMMode::SingleBlockVertical),
            (6, PSMMode::SingleBlock),
            (7, PSMMode::SingleLine),
            (8, PSMMode::SingleWord),
            (9, PSMMode::CircleWord),
            (10, PSMMode::SingleChar),
        ];

        for (value, expected) in modes {
            let mode = PSMMode::from_u8(value).unwrap();
            assert_eq!(mode, expected);
        }
    }

    #[test]
    fn test_psm_mode_from_u8_invalid() {
        let invalid_values = [11, 12, 255, 100];

        for value in invalid_values {
            let result = PSMMode::from_u8(value);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Invalid PSM mode"));
        }
    }

    #[test]
    fn test_psm_mode_as_u8() {
        assert_eq!(PSMMode::Auto.as_u8(), 3);
        assert_eq!(PSMMode::SingleLine.as_u8(), 7);
        assert_eq!(PSMMode::SingleChar.as_u8(), 10);
    }

    #[test]
    fn test_tesseract_config_default() {
        let config = TesseractConfig::default();

        assert_eq!(config.language, "eng");
        assert_eq!(config.output_format, "markdown");
        assert!(config.enable_table_detection);
        assert_eq!(config.table_min_confidence, 0.0);
        assert_eq!(config.table_column_threshold, 50);
        assert_eq!(config.table_row_threshold_ratio, 0.5);
        assert!(config.use_cache);
        assert!(
            config.language_model_ngram_on,
            "the n-gram language model penalizes non-dictionary output (recognition noise) \
             and must be on by default, not off"
        );
        assert_eq!(
            config.min_confidence, MIN_CONFIDENCE_FLOOR_DEFAULT,
            "min_confidence stays at 0.0 until it is calibrated per-word, not per-page \
             (see MIN_CONFIDENCE_FLOOR_DEFAULT)"
        );

        #[cfg(target_arch = "wasm32")]
        assert_eq!(config.psm, 6, "WASM default must be PSM_SINGLE_BLOCK (6)");
        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(config.psm, 3, "native default must be PSM_AUTO (3)");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_tesseract_config_validate_valid() {
        let valid_formats = ["text", "markdown", "hocr", "tsv"];

        for format in valid_formats {
            let config = TesseractConfig {
                output_format: format.to_string(),
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }

        for method in ["none", "off"] {
            let config = TesseractConfig {
                preprocessing: Some(ImagePreprocessingConfig {
                    deskew: false,
                    binarization_method: method.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            };
            assert!(config.validate().is_ok(), "{method} must disable binarization");
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_reject_tesseract_config_when_deskew_has_no_binarization() {
        let config = TesseractConfig {
            preprocessing: Some(ImagePreprocessingConfig {
                deskew: true,
                binarization_method: "off".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            config.validate().unwrap_err(),
            "deskew must be false when binarization_method is none or off"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_tesseract_config_validate_invalid() {
        let config = TesseractConfig {
            output_format: "invalid".to_string(),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid output_format"));
    }

    #[test]
    fn test_extraction_result_creation() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let result = ExtractionResult {
            content: "Test content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: metadata.clone(),
            tables: vec![table],
        };

        assert_eq!(result.content, "Test content");
        assert_eq!(result.mime_type, "text/plain");
        assert_eq!(result.metadata.get("key").unwrap(), &serde_json::json!("value"));
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_table_creation() {
        let cells = vec![
            vec!["Header1".to_string(), "Header2".to_string()],
            vec!["Value1".to_string(), "Value2".to_string()],
        ];

        let markdown = "| Header1 | Header2 |\n| ------- | ------- |\n| Value1  | Value2  |".to_string();

        let table = Table {
            cells: cells.clone(),
            markdown: markdown.clone(),
            page_number: 1,
        };

        assert_eq!(table.cells.len(), 2);
        assert_eq!(table.cells[0].len(), 2);
        assert_eq!(table.markdown, markdown);
        assert_eq!(table.page_number, 1);
    }

    #[test]
    fn test_batch_item_result_success() {
        let result = crate::types::OcrExtractionResult {
            content: "content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: std::collections::HashMap::new(),
            tables: vec![],
            ocr_elements: None,
            internal_document: None,
        };

        let batch_result = BatchItemResult {
            file_path: "/path/to/file.png".to_string(),
            success: true,
            result: Some(result),
            error: None,
        };

        assert_eq!(batch_result.file_path, "/path/to/file.png");
        assert!(batch_result.success);
        assert!(batch_result.result.is_some());
        assert!(batch_result.error.is_none());
    }

    #[test]
    fn test_batch_item_result_failure() {
        let batch_result = BatchItemResult {
            file_path: "/path/to/file.png".to_string(),
            success: false,
            result: None,
            error: Some("File not found".to_string()),
        };

        assert_eq!(batch_result.file_path, "/path/to/file.png");
        assert!(!batch_result.success);
        assert!(batch_result.result.is_none());
        assert_eq!(batch_result.error.as_ref().unwrap(), "File not found");
    }

    #[test]
    fn test_tesseract_config_from_public_api() {
        let public_config = crate::types::TesseractConfig {
            language: vec!["deu".to_string()],
            psm: Some(6),
            output_format: "text".to_string(),
            oem: 1,
            min_confidence: 70.0,
            preprocessing: Some(ImagePreprocessingConfig::default()),
            enable_table_detection: false,
            table_min_confidence: 50.0,
            table_column_threshold: 100,
            table_row_threshold_ratio: 0.8,
            use_cache: false,
            classify_use_pre_adapted_templates: false,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: false,
            tessedit_dont_rowrej_good_wds: false,
            tessedit_enable_dict_correction: false,
            tessedit_char_whitelist: "0123456789".to_string(),
            tessedit_char_blacklist: "!@#$".to_string(),
            tessedit_use_primary_params_model: false,
            textord_space_size_is_variable: false,
            thresholding_method: "sauvola".to_string(),
        };

        let internal_config: TesseractConfig = (&public_config).into();

        assert_eq!(internal_config.language, "deu");
        assert_eq!(internal_config.psm, 6);
        assert_eq!(internal_config.output_format, "text");
        assert_eq!(internal_config.oem, 1);
        assert_eq!(internal_config.min_confidence, 70.0);
        assert!(internal_config.preprocessing.is_some());
        assert!(!internal_config.enable_table_detection);
        assert_eq!(internal_config.table_min_confidence, 50.0);
        assert_eq!(internal_config.table_column_threshold, 100);
        assert_eq!(internal_config.table_row_threshold_ratio, 0.8);
        assert!(!internal_config.use_cache);
        assert!(!internal_config.classify_use_pre_adapted_templates);
        assert!(internal_config.language_model_ngram_on);
        assert!(!internal_config.tessedit_dont_blkrej_good_wds);
        assert!(!internal_config.tessedit_dont_rowrej_good_wds);
        assert!(!internal_config.tessedit_enable_dict_correction);
        assert_eq!(internal_config.tessedit_char_whitelist, "0123456789");
        assert_eq!(internal_config.tessedit_char_blacklist, "!@#$");
        assert!(!internal_config.tessedit_use_primary_params_model);
        assert!(!internal_config.textord_space_size_is_variable);
        assert_eq!(internal_config.thresholding_method, "sauvola");
    }

    /// #1784: every name the engine numbers parses to its number; anything else, including the
    /// old boolean spelling, is rejected by `validate` before the engine could read it as 0.
    #[test]
    fn thresholding_method_names_map_to_the_engine_numbers_and_others_are_rejected() {
        assert_eq!(
            ThresholdingMethod::parse("otsu").map(ThresholdingMethod::tesseract_value),
            Some("0")
        );
        assert_eq!(
            ThresholdingMethod::parse("Leptonica_Otsu").map(ThresholdingMethod::tesseract_value),
            Some("1")
        );
        assert_eq!(
            ThresholdingMethod::parse("sauvola").map(ThresholdingMethod::tesseract_value),
            Some("2")
        );
        assert_eq!(ThresholdingMethod::parse("true"), None);
        assert_eq!(ThresholdingMethod::parse("adaptive"), None);
        assert_eq!(ThresholdingMethod::parse("1"), None);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn validate_rejects_a_thresholding_method_the_engine_cannot_use() {
        let config = TesseractConfig {
            thresholding_method: "true".to_string(),
            ..Default::default()
        };
        let error = config.validate().unwrap_err();
        assert!(
            error.contains("thresholding_method") && error.contains("sauvola"),
            "{error}"
        );
        assert!(
            TesseractConfig {
                thresholding_method: "leptonica_otsu".to_string(),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
    }
}
