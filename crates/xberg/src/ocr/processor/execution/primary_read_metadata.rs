use super::{MIN_ORIENTATION_CONFIDENCE, insert_dictionary_line_counts, insert_retained_word_confidence_metadata};
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::RetainedWordConfidenceStats;
use crate::ocr::types::TesseractConfig;
use crate::types::ImagePreprocessingMetadata;
use std::collections::HashMap;

pub(super) type WordConfidenceStats = (i32, i32, usize, usize);

pub(super) struct PrimaryRead<'a> {
    pub(super) config: &'a TesseractConfig,
    pub(super) dimensions: (u32, u32),
    pub(super) image_preprocessing: Option<ImagePreprocessingMetadata>,
    pub(super) auto_rotate_unavailable: bool,
    pub(super) dictionary_line_counts: (usize, usize),
    pub(super) retained_hocr_confidence_stats: Option<&'a RetainedWordConfidenceStats>,
    pub(super) mean_text_conf: i32,
    pub(super) word_confidence_stats: Option<WordConfidenceStats>,
    pub(super) detected_orientation: Option<(i32, f32)>,
}

pub(super) fn build(read: PrimaryRead<'_>) -> Result<HashMap<String, serde_json::Value>, OcrError> {
    let (ocr_image_width, ocr_image_height) = read.dimensions;
    let mut metadata = HashMap::new();
    if let Some(image_preprocessing) = read.image_preprocessing {
        let value = serde_json::to_value(image_preprocessing).map_err(|error| {
            OcrError::ProcessingFailed(format!("Failed to serialize image preprocessing metadata: {error}"))
        })?;
        metadata.insert(
            crate::ocr_metadata_keys::OCR_IMAGE_PREPROCESSING_METADATA_KEY.to_string(),
            value,
        );
    }
    metadata.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.to_string(),
        serde_json::Value::Number(ocr_image_width.into()),
    );
    metadata.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.to_string(),
        serde_json::Value::Number(ocr_image_height.into()),
    );
    metadata.insert(
        "language".to_string(),
        serde_json::Value::String(read.config.language.clone()),
    );
    metadata.insert(
        "psm".to_string(),
        serde_json::Value::String(read.config.psm.to_string()),
    );
    metadata.insert("table_count".to_string(), serde_json::Value::Number(0.into()));
    metadata.insert("tables_detected".to_string(), serde_json::Value::Number(0.into()));
    if read.config.output_format == "markdown" {
        metadata.insert(
            "source_format".to_string(),
            serde_json::Value::String("hocr".to_string()),
        );
    }
    if read.auto_rotate_unavailable {
        metadata.insert("auto_rotate_unavailable".to_string(), serde_json::Value::Bool(true));
    }
    insert_dictionary_line_counts(
        &mut metadata,
        read.dictionary_line_counts.0,
        read.dictionary_line_counts.1,
    );
    insert_initial_confidence(
        &mut metadata,
        &read.config.output_format,
        read.retained_hocr_confidence_stats,
        read.mean_text_conf,
        read.word_confidence_stats,
    );
    insert_orientation(&mut metadata, read.detected_orientation);
    Ok(metadata)
}

fn insert_initial_confidence(
    metadata: &mut HashMap<String, serde_json::Value>,
    output_format: &str,
    retained_hocr_confidence_stats: Option<&RetainedWordConfidenceStats>,
    mean_text_conf: i32,
    word_confidence_stats: Option<WordConfidenceStats>,
) {
    if let Some(stats) = retained_hocr_confidence_stats {
        insert_retained_word_confidence_metadata(metadata, stats);
    } else if output_format != "text" {
        if mean_text_conf >= 0 {
            metadata.insert(
                "mean_text_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(mean_text_conf)),
            );
        }

        if let Some((median_conf, p10_conf, word_count, low_conf_count)) = word_confidence_stats {
            metadata.insert(
                "median_word_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(median_conf)),
            );
            metadata.insert(
                "p10_word_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(p10_conf)),
            );
            metadata.insert(
                "word_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(word_count)),
            );
            metadata.insert(
                "low_conf_word_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(low_conf_count)),
            );
        }
    }
}

fn insert_orientation(metadata: &mut HashMap<String, serde_json::Value>, detected_orientation: Option<(i32, f32)>) {
    // No `script_name`/`script_confidence` metadata: the ONNX PP-LCNet
    // orientation classifier used here has no script-detection capability,
    // unlike Tesseract's own `DetectOrientationScript()`/OSD (which this crate
    // does not call). Emitting a placeholder script name would misrepresent
    // real detection as having happened (#184). ~keep
    if let Some((orient_deg, orient_conf)) = detected_orientation {
        metadata.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY.to_string(),
            serde_json::Value::Number(serde_json::Number::from(orient_deg)),
        );
        metadata.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_CONFIDENCE_METADATA_KEY.to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(orient_conf as f64).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
            metadata.insert(
                crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY.to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr_metadata_keys::{
        OCR_AUTO_ROTATED_METADATA_KEY, OCR_IMAGE_PREPROCESSING_METADATA_KEY, OCR_ORIENTATION_CONFIDENCE_METADATA_KEY,
        OCR_ORIENTATION_DEGREES_METADATA_KEY, OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
    };
    use serde_json::json;

    fn primary_read(config: &TesseractConfig) -> PrimaryRead<'_> {
        PrimaryRead {
            config,
            dimensions: (400, 200),
            image_preprocessing: None,
            auto_rotate_unavailable: false,
            dictionary_line_counts: (0, 0),
            retained_hocr_confidence_stats: None,
            mean_text_conf: 80,
            word_confidence_stats: Some((71, 24, 6, 1)),
            detected_orientation: None,
        }
    }

    #[test]
    fn text_defers_engine_confidence_and_keeps_preprocessing_provenance() {
        let config = TesseractConfig {
            output_format: "text".into(),
            language: "deu".into(),
            psm: 11,
            ..Default::default()
        };
        let mut read = primary_read(&config);
        read.image_preprocessing = Some(ImagePreprocessingMetadata {
            original_dimensions: (200_usize, 100).into(),
            original_dpi: (150.0, 150.0).into(),
            target_dpi: 300,
            scale_factor: 2.0,
            auto_adjusted: false,
            final_dpi: 300,
            new_dimensions: Some((400_usize, 200).into()),
            resample_method: "CATMULLROM".into(),
            dimension_clamped: false,
            calculated_dpi: None,
            skipped_resize: false,
            resize_error: None,
        });
        let expected_preprocessing = serde_json::to_value(read.image_preprocessing.as_ref().unwrap()).unwrap();
        let values = build(read).unwrap();
        assert_eq!(values[OCR_IMAGE_PREPROCESSING_METADATA_KEY], expected_preprocessing);
        assert_eq!(values[OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY], json!(400));
        assert_eq!(values[OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY], json!(200));
        assert_eq!(values["language"], json!("deu"));
        assert_eq!(values["psm"], json!("11"));
        assert_eq!(values["table_count"], json!(0));
        assert_eq!(values["tables_detected"], json!(0));
        assert!(!values.contains_key("mean_text_conf"));
        assert!(!values.contains_key("word_count"));
        assert!(!values.contains_key("source_format"));
        assert!(!values.contains_key("auto_rotate_unavailable"));
    }

    #[test]
    fn markdown_reports_only_retained_hocr_confidence_and_provenance() {
        let config = TesseractConfig {
            output_format: "markdown".into(),
            ..Default::default()
        };
        let mut stats = RetainedWordConfidenceStats::default();
        stats.record(21.0);
        stats.record(83.0);
        let mut read = primary_read(&config);
        read.retained_hocr_confidence_stats = Some(&stats);
        read.dictionary_line_counts = (3, 1);
        read.detected_orientation = Some((180, 0.75));
        let values = build(read).unwrap();
        assert_eq!(values["source_format"], json!("hocr"));
        assert_eq!(values["mean_text_conf"], json!(52));
        assert_eq!(values["median_word_conf"], json!(52));
        assert_eq!(values["p10_word_conf"], json!(21));
        assert_eq!(values["word_count"], json!(2));
        assert_eq!(values["low_conf_word_count"], json!(1));
        assert_eq!(values["dictionary_filtered_line_count"], json!(3));
        assert_eq!(values["dictionary_kept_numeral_line_count"], json!(1));
        assert_eq!(values[OCR_ORIENTATION_DEGREES_METADATA_KEY], json!(180));
        assert_eq!(values[OCR_ORIENTATION_CONFIDENCE_METADATA_KEY], json!(0.75));
        assert_eq!(values[OCR_AUTO_ROTATED_METADATA_KEY], json!(true));
    }

    #[test]
    fn direct_markup_uses_engine_confidence_and_only_reports_applied_rotation() {
        for output_format in ["hocr", "tsv"] {
            let config = TesseractConfig {
                output_format: output_format.into(),
                ..Default::default()
            };
            let mut read = primary_read(&config);
            read.auto_rotate_unavailable = true;
            read.detected_orientation = Some((180, MIN_ORIENTATION_CONFIDENCE));
            let values = build(read).unwrap();
            assert_eq!(values["mean_text_conf"], json!(80));
            assert_eq!(values["median_word_conf"], json!(71));
            assert_eq!(values["p10_word_conf"], json!(24));
            assert_eq!(values["word_count"], json!(6));
            assert_eq!(values["low_conf_word_count"], json!(1));
            assert_eq!(values["auto_rotate_unavailable"], json!(true));
            assert_eq!(values[OCR_ORIENTATION_DEGREES_METADATA_KEY], json!(180));
            assert!(!values.contains_key(OCR_AUTO_ROTATED_METADATA_KEY));
            assert!(!values.contains_key("source_format"));
        }
    }

    #[test]
    fn confident_upright_orientation_is_not_reported_as_auto_rotated() {
        let config = TesseractConfig {
            output_format: "hocr".into(),
            ..Default::default()
        };
        let mut read = primary_read(&config);
        read.detected_orientation = Some((0, 0.9));
        let values = build(read).unwrap();
        assert_eq!(values[OCR_ORIENTATION_DEGREES_METADATA_KEY], json!(0));
        assert_eq!(
            values[OCR_ORIENTATION_CONFIDENCE_METADATA_KEY],
            json!(f64::from(0.9_f32))
        );
        assert!(!values.contains_key(OCR_AUTO_ROTATED_METADATA_KEY));
    }

    #[test]
    fn negative_engine_mean_confidence_is_omitted_but_word_stats_remain() {
        let config = TesseractConfig {
            output_format: "hocr".into(),
            ..Default::default()
        };
        let mut read = primary_read(&config);
        read.mean_text_conf = -1;
        let values = build(read).unwrap();
        assert!(!values.contains_key("mean_text_conf"));
        assert_eq!(values["median_word_conf"], json!(71));
        assert_eq!(values["word_count"], json!(6));
    }
}
