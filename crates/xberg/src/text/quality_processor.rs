//! Quality processing post-processor.
//!
//! This module provides a PostProcessor plugin that performs quality assessment on
//! extraction results.
//!
//! # Performance
//!
//! This processor optimizes metadata handling by:
//! - Checking if important metadata fields exist before allocating
//! - Converting to HashMap only when beneficial metadata is present
//! - Skipping allocation entirely for documents without metadata
//!
//! This avoids unnecessary string cloning for sparse metadata scenarios.

use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
use crate::{ExtractedDocument, ExtractionConfig, Result};
use async_trait::async_trait;
#[cfg(test)]
use std::borrow::Cow;

/// Post-processor that calculates a quality score.
///
/// This processor:
/// - Runs in the Early processing stage
/// - Calculates quality score when `config.enable_quality_processing` is true
/// - Stores the text cleanliness/readability score in `ExtractedDocument::quality_score`
///
/// The score describes retained text, not extraction completeness or recall.
/// Callers must inspect `ExtractedDocument::processing_warnings` separately for
/// known omissions and degraded processing.
///
/// When OCR ran and reported a per-page recognition confidence
/// (`PageContent::ocr_confidence`), the score is capped at the word-count-weighted mean
/// of those confidences (issue #1669): the text-shape heuristic in
/// `text::quality::calculate_quality_score` reads only the retained text itself, so a page
/// the OCR engine recognized at 81% confidence can still look shape-clean and score 1.0.
/// That confidence is a fact about the trustworthiness of the SAME retained text
/// `quality_score` already claims to describe, not a completeness signal, so folding it in
/// here does not reopen the completeness/quality boundary
/// `quality_score_does_not_hide_completeness_warning` establishes below.
///
/// # Example
///
/// ```rust,no_run
/// use xberg::plugins::{Plugin, PostProcessor};
/// use xberg::text::QualityProcessor;
///
/// let processor = QualityProcessor;
/// assert_eq!(processor.name(), "quality-processing");
/// ```
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy)]
pub struct QualityProcessor;

impl Plugin for QualityProcessor {
    fn name(&self) -> &str {
        "quality-processing"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for QualityProcessor {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let mut quality_score = if should_use_metadata(&result.metadata) {
            crate::text::quality::calculate_quality_score(&result.content, Some(&result.metadata.additional))
        } else {
            crate::text::quality::calculate_quality_score(&result.content, None)
        };

        if let Some(ocr_confidence) = aggregate_ocr_confidence(result.pages.as_deref()) {
            quality_score = quality_score.min(ocr_confidence);
        }

        result.quality_score = Some(quality_score);

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }

    fn should_process(&self, _result: &ExtractedDocument, config: &ExtractionConfig) -> bool {
        config.enable_quality_processing
    }

    fn estimated_duration_ms(&self, result: &ExtractedDocument) -> u64 {
        let text_length = result.content.len();
        (text_length / 102400).max(1) as u64
    }

    fn priority(&self) -> i32 {
        30
    }
}

/// Check if metadata contains any important fields without allocation.
///
/// # Performance
///
/// O(1) check avoiding HashMap allocation when metadata is sparse.
/// Only allocates HashMap when important metadata fields are present.
fn should_use_metadata(metadata: &crate::types::Metadata) -> bool {
    const IMPORTANT_FIELDS: &[&str] = &["title", "author", "subject", "description", "keywords"];
    IMPORTANT_FIELDS
        .iter()
        .any(|field| metadata.additional.contains_key(*field))
}

/// Below this many recognized words across all OCR'd pages, the mean confidence is based on
/// too little evidence to trust as a ceiling on the whole document's score (issue #1669).
/// It follows the caution in `PageOcrConfidence::word_count`'s own doc comment: a high score
/// next to a small word count is not representative.
///
/// ~keep: this floor is calibrated for this cap alone and is deliberately independent. It is
/// not derived from `OcrConfig::min_words_for_ocr_output_check`, whose serde default happens
/// to be 20 today, because that field is an operator-tunable knob for an unrelated check on
/// fragmented OCR output. Deriving this floor from it would let a deployer who tunes that
/// check silently move the ceiling on every document's quality score.
const MIN_OCR_WORDS_FOR_CONFIDENCE_FLOOR: u64 = 20;

/// Word-count-weighted mean OCR recognition confidence across pages that report one.
///
/// `None` when no page carries a score (native extraction, a backend with no calibrated
/// legibility scale, or fewer than [`MIN_OCR_WORDS_FOR_CONFIDENCE_FLOOR`] recognized words
/// total). Pages OCR'd by different backends are pooled by word count; `PageOcrConfidence`
/// documents that raw scores must never be compared ACROSS backends, but a single weighted
/// mean used only as a ceiling does not compare them against each other, only against the
/// text-shape score.
fn aggregate_ocr_confidence(pages: Option<&[crate::types::PageContent]>) -> Option<f64> {
    let (weighted_sum, total_words) = pages?
        .iter()
        .filter_map(|page| page.ocr_confidence.as_ref())
        .filter_map(|confidence| confidence.score.map(|score| (score, u64::from(confidence.word_count))))
        .fold((0.0_f64, 0_u64), |(sum, words), (score, word_count)| {
            (sum + score * word_count as f64, words + word_count)
        });

    (total_words >= MIN_OCR_WORDS_FOR_CONFIDENCE_FLOOR).then(|| weighted_sum / total_words as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PageContent, PageOcrConfidence};

    fn ocrd_page(page_number: u32, score: Option<f64>, word_count: u32) -> PageContent {
        PageContent {
            page_number,
            content: String::new(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: Some(PageOcrConfidence {
                score,
                word_count,
                backend: "tesseract".to_string(),
            }),
        }
    }

    /// xberg#1669: the exact shape of the issue. Retained text reads as clean prose (no
    /// double-spaces, ellipses, or dash artifacts), so the text-shape heuristic alone scores
    /// it 1.0, but the OCR engine that produced it reported 0.81 mean confidence with plenty
    /// of words behind that number. `quality_score` must reflect the confidence it already
    /// has, not just the shape of the text.
    #[tokio::test]
    async fn quality_score_is_capped_by_low_ocr_confidence_on_shape_clean_text() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };
        let mut result = ExtractedDocument {
            content: "The retained paragraph reads as clean, readable prose with complete \
                      sentences and conventional punctuation throughout the page."
                .to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            pages: Some(vec![ocrd_page(1, Some(0.81), 248)]),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.quality_score, Some(0.81));
    }

    /// A single-word OCR'd fragment must not tank the score for the rest of a long, clean,
    /// confidently-recognized document: not enough evidence to trust (#1669).
    #[tokio::test]
    async fn quality_score_ignores_a_tiny_low_confidence_fragment() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };
        let mut result = ExtractedDocument {
            content: "The retained paragraph reads as clean, readable prose with complete \
                      sentences and conventional punctuation throughout the page."
                .to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            pages: Some(vec![ocrd_page(1, Some(0.1), 1)]),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.quality_score, Some(1.0));
    }

    /// A backend with no calibrated legibility scale reports `score: None`; it must not be
    /// treated as a zero confidence.
    #[tokio::test]
    async fn quality_score_is_unaffected_by_an_uncalibrated_ocr_backend() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };
        let mut result = ExtractedDocument {
            content: "The retained paragraph reads as clean, readable prose with complete \
                      sentences and conventional punctuation throughout the page."
                .to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            pages: Some(vec![ocrd_page(1, None, 500)]),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.quality_score, Some(1.0));
    }

    #[test]
    fn aggregate_ocr_confidence_weights_by_word_count() {
        let pages = vec![ocrd_page(1, Some(0.9), 100), ocrd_page(2, Some(0.5), 300)];

        let aggregate = aggregate_ocr_confidence(Some(&pages)).expect("must aggregate");

        // (0.9*100 + 0.5*300) / 400 = 0.6
        assert!((aggregate - 0.6).abs() < 1e-9, "got {aggregate}");
    }

    #[test]
    fn aggregate_ocr_confidence_is_none_without_pages() {
        assert_eq!(aggregate_ocr_confidence(None), None);
    }

    #[tokio::test]
    async fn test_quality_processor() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };

        let mut result = ExtractedDocument {
            content: "This is a well-written paragraph with proper structure. It contains multiple sentences. The quality should be good.".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.quality_score.is_some());
        let score = result.quality_score.unwrap();
        assert!((0.0..=1.0).contains(&score));
    }

    #[tokio::test]
    async fn quality_score_does_not_hide_completeness_warning() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };
        let warning = crate::types::ProcessingWarning {
            source: Cow::Borrowed("ocr"),
            message: Cow::Borrowed("Page 4 was rejected and its text was discarded."),
        };
        let mut result = ExtractedDocument {
            content:
                "The retained paragraph is clean, readable prose with complete sentences and conventional punctuation."
                    .to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            processing_warnings: vec![warning.clone()],
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.quality_score, Some(1.0));
        assert_eq!(result.processing_warnings.len(), 1);
        assert_eq!(result.processing_warnings[0].source, warning.source);
        assert_eq!(result.processing_warnings[0].message, warning.message);
    }

    #[tokio::test]
    async fn test_quality_processor_disabled() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: false,
            ..Default::default()
        };

        let mut result = ExtractedDocument {
            content: "Some text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        processor.process(&mut result, &config).await.unwrap();
    }

    #[test]
    fn test_quality_processor_plugin_interface() {
        let processor = QualityProcessor;
        assert_eq!(processor.name(), "quality-processing");
        assert!(!processor.version().is_empty());
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[test]
    fn test_quality_processor_stage() {
        let processor = QualityProcessor;
        assert_eq!(processor.processing_stage(), ProcessingStage::Early);
    }

    #[test]
    fn test_quality_processor_should_process() {
        let processor = QualityProcessor;

        let result = ExtractedDocument {
            content: "Sample text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let config_with_quality = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };
        assert!(processor.should_process(&result, &config_with_quality));

        let config_without_quality = ExtractionConfig {
            enable_quality_processing: false,
            ..Default::default()
        };
        assert!(!processor.should_process(&result, &config_without_quality));
    }

    #[test]
    fn test_quality_processor_estimated_duration() {
        let processor = QualityProcessor;

        let short_result = ExtractedDocument {
            content: "Short".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let long_result = ExtractedDocument {
            content: "a".repeat(1000000),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let short_duration = processor.estimated_duration_ms(&short_result);
        let long_duration = processor.estimated_duration_ms(&long_result);

        assert!(long_duration > short_duration);
    }
}
