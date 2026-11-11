use super::super::pipeline::{ProvenReadPolicy, XObjectRecoveryContext};
use super::{XObjectRecoveryOutcome, recover_image_xobjects};
use crate::core::config::{OcrConfig, OcrQualityThresholds};
use crate::extractors::security::{SecurityBudget, SecurityLimits};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, FormatMetadata, ImagePreprocessingMetadata, OcrMetadata};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const PROSE: &str = "The farmers wait for the fields to dry";

struct ScriptedBackend {
    reads: Vec<ExtractedDocument>,
    next: AtomicUsize,
}

impl Plugin for ScriptedBackend {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "test".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for ScriptedBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        Ok(self.reads[self.next.fetch_add(1, Ordering::SeqCst)].clone())
    }
}

fn read(text: &str, marker: &str, clamped: bool) -> ExtractedDocument {
    let mut doc = ExtractedDocument {
        content: text.to_owned(),
        ..Default::default()
    };
    doc.metadata.image_preprocessing = Some(ImagePreprocessingMetadata {
        original_dimensions: (300, 300).into(),
        original_dpi: (300.0, 300.0).into(),
        target_dpi: 300,
        scale_factor: if clamped { 0.24 } else { 1.0 },
        auto_adjusted: clamped,
        final_dpi: if clamped { 72 } else { 300 },
        new_dimensions: None,
        resample_method: marker.to_owned(),
        dimension_clamped: clamped,
        calculated_dpi: None,
        skipped_resize: !clamped,
        resize_error: None,
    });
    doc.metadata.format = Some(FormatMetadata::Ocr(OcrMetadata {
        language: "eng".to_owned(),
        psm: 6,
        output_format: "text".to_owned(),
        table_count: 0,
        table_rows: None,
        table_cols: None,
    }));
    doc
}

async fn recover(reads: Vec<ExtractedDocument>, discard: bool) -> XObjectRecoveryOutcome {
    let thresholds = OcrQualityThresholds {
        discard_suspected_ocr_noise: discard,
        ..Default::default()
    };
    recover_with_limits(reads, &thresholds, &SecurityLimits::default())
        .await
        .expect("scripted image recovery")
}

async fn recover_with_limits(
    reads: Vec<ExtractedDocument>,
    thresholds: &OcrQualityThresholds,
    limits: &SecurityLimits,
) -> crate::Result<XObjectRecoveryOutcome> {
    let images = (0..reads.len())
        .map(|index| {
            let mut image = crate::pdf::native::images::PageFallbackImage::placeholder_jpeg(index);
            image.source_dpi = Some(300.0);
            image
        })
        .collect::<Vec<_>>();
    let backend: Arc<dyn OcrBackend> = Arc::new(ScriptedBackend {
        reads,
        next: AtomicUsize::new(0),
    });
    let mut budget = SecurityBudget::from_limits(limits);
    let mut context = XObjectRecoveryContext::with_budget(&mut budget, None);
    context.proven_read = Some(ProvenReadPolicy {
        thresholds,
        warn_when_kept: false,
    });
    recover_image_xobjects(&backend, &images, 0, 1, &OcrConfig::default(), context).await
}

#[tokio::test]
async fn rejected_first_read_does_not_supply_retained_page_preprocessing() {
    let noise = "ab ".repeat(20);
    let outcome = recover(vec![read(&noise, "discarded", true), read(PROSE, "kept", false)], true).await;
    assert_eq!(outcome.text, PROSE);
    assert_eq!(outcome.image_preprocessing.unwrap().resample_method, "kept");
    assert_eq!(outcome.tesseract_psm, None);
}

#[tokio::test]
async fn a_rejected_single_read_sets_neither_preprocessing_nor_tesseract_mode() {
    let outcome = recover(vec![read(&"ab ".repeat(20), "discarded", true)], true).await;
    assert_eq!(outcome.text, "");
    assert!(outcome.image_preprocessing.is_none());
    assert_eq!(outcome.tesseract_psm, None);
}

#[tokio::test]
async fn a_retained_single_read_keeps_its_preprocessing_and_tesseract_mode() {
    let outcome = recover(vec![read(PROSE, "kept", false)], true).await;
    assert_eq!(outcome.text, PROSE);
    assert_eq!(outcome.image_preprocessing.unwrap().resample_method, "kept");
    assert_eq!(outcome.tesseract_psm, Some(6));
}

#[tokio::test]
async fn pooled_rejections_do_not_supply_page_preprocessing() {
    let noise = "ab ".repeat(12);
    let outcome = recover(
        vec![read(&noise, "discarded", true), read(&noise, "discarded", true)],
        true,
    )
    .await;
    assert_eq!(outcome.text, "");
    assert!(outcome.image_preprocessing.is_none());
    assert_eq!(outcome.tesseract_psm, None);
}

#[tokio::test]
async fn small_legible_read_survives_garbage_in_the_same_pool() {
    for clamped in [true, false] {
        for discard in [true, false] {
            let noise = "ab ".repeat(12);
            let outcome = recover(
                vec![
                    read(&noise, "first", clamped),
                    read(PROSE, "legible", clamped),
                    read(&noise, "last", clamped),
                ],
                discard,
            )
            .await;
            let expected = if clamped || discard {
                PROSE.to_owned()
            } else {
                format!("{noise}\n\n{PROSE}\n\n{noise}")
            };
            assert_eq!(outcome.text, expected, "clamped={clamped}, discard={discard}");
            let expected_marker = if clamped || discard { "legible" } else { "first" };
            assert_eq!(outcome.image_preprocessing.unwrap().resample_method, expected_marker);
            if !clamped && discard {
                assert_eq!(outcome.warnings.len(), 1);
                assert_eq!(
                    outcome.warnings[0].message,
                    "Page 1 produced suspected OCR recognition noise; discarded 2 of 3 small embedded-image reads."
                );
            } else {
                assert!(outcome.warnings.is_empty());
            }
        }
    }
}

#[test]
fn counts_two_character_fragments_regardless_of_utf8_width() {
    use super::super::scoring::small_read_has_fragmented_words;

    let thresholds = OcrQualityThresholds {
        max_ocr_output_fragmented_word_ratio: 0.5,
        ..Default::default()
    };
    assert!(small_read_has_fragmented_words("ab cd ef gh", &thresholds));
    assert!(small_read_has_fragmented_words("éa üö ña åb", &thresholds));
    assert!(!small_read_has_fragmented_words("été über mañana åter", &thresholds));
}

#[tokio::test]
async fn tiny_garbage_reads_are_still_rejected_when_their_pool_has_enough_words() {
    let reads = (0..4).map(|_| read("ab cd ef gh ij kl", "noise", true)).collect();
    let outcome = recover(reads, true).await;
    assert_eq!(outcome.text, "");
    assert!(outcome.image_preprocessing.is_none());
}

#[tokio::test]
async fn a_legible_small_read_keeps_its_table_and_formula_when_its_pool_is_noisy() {
    let mut legible = read(PROSE, "legible", true);
    legible.tables.push(crate::types::Table {
        markdown: "| Wheat | Harvest |".to_owned(),
        cells: vec![vec!["Wheat".to_owned(), "Harvest".to_owned()]],
        ..Default::default()
    });
    legible.formulas.push(crate::types::Formula {
        latex: "x+y".to_owned(),
        bbox: None,
        page: None,
    });
    let noise = "ab ".repeat(12);
    let outcome = recover(
        vec![read(&noise, "noise", true), legible, read(&noise, "noise", true)],
        true,
    )
    .await;
    assert_eq!(outcome.tables.len(), 1);
    assert_eq!(outcome.tables[0].cells, [vec!["Wheat", "Harvest"]]);
    assert_eq!(outcome.tables[0].page_number, 1);
    assert_eq!(outcome.formulas.len(), 1);
    assert_eq!(outcome.formulas[0].latex, "x+y");
    assert_eq!(outcome.formulas[0].page, Some(1));
}

#[tokio::test]
async fn only_reads_with_text_or_structured_content_supply_page_metadata() {
    let outcome = recover(vec![read(" \t", "blank", true), read(PROSE, "kept", false)], true).await;
    assert_eq!(outcome.text, PROSE);
    assert_eq!(outcome.image_preprocessing.unwrap().resample_method, "kept");

    let blank = recover(vec![read(" \t", "blank", true)], true).await;
    assert!(blank.image_preprocessing.is_none());
    assert_eq!(blank.tesseract_psm, None);

    let mut formula_only = read("", "formula", false);
    formula_only.formulas.push(crate::types::Formula {
        latex: "x+y".to_owned(),
        bbox: None,
        page: None,
    });
    let structured = recover(vec![formula_only], true).await;
    assert_eq!(structured.formulas.len(), 1);
    assert_eq!(structured.image_preprocessing.unwrap().resample_method, "formula");
    assert_eq!(structured.tesseract_psm, Some(6));
}

#[tokio::test]
async fn zero_fragmentation_threshold_still_rejects_a_numeric_only_pool() {
    let numbers = "123 ".repeat(10);
    let thresholds = OcrQualityThresholds {
        max_ocr_output_fragmented_word_ratio: 0.0,
        ..Default::default()
    };
    let outcome = recover_with_limits(
        vec![read(&numbers, "first", true), read(&numbers, "second", true)],
        &thresholds,
        &SecurityLimits::default(),
    )
    .await
    .expect("scripted numeric reads");
    assert_eq!(outcome.text, "");
    assert!(outcome.image_preprocessing.is_none());
}

#[tokio::test]
async fn unpublished_preprocessing_does_not_consume_the_output_budget() {
    let text = "First\n\nSecond";
    let marker = "first-resampling";
    let limits = SecurityLimits {
        max_content_size: 2 * text.len() + marker.len(),
        ..Default::default()
    };
    let outcome = recover_with_limits(
        vec![read("First", marker, false), read("Second", "second-resampling", false)],
        &OcrQualityThresholds::default(),
        &limits,
    )
    .await
    .expect("output includes only the first read's preprocessing");
    assert_eq!(outcome.text, text);
    assert_eq!(outcome.text_outside_tables, text);
    assert_eq!(outcome.image_preprocessing.unwrap().resample_method, marker);
}
