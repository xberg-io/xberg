use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::scoring::compute_quality_score;
use super::sideways_retry::read_whole_page_raster;
use crate::core::config::{OcrConfig, OcrQualityThresholds};
use crate::plugins::{ConfidenceSemantics, OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementConfig, OcrElementLevel};

const SPARSE_PSM: i32 = 11;
const AUTO_PSM: i32 = 3;
/// Blocks in a `read_levels` read: more than the five words, so counting them would decide the verdict.
const BLOCKS: usize = 8;
const NOISY_TEXT: &str = "ab cd ef gh ij kl mn op qr st uv wx";
const CLEAN_TEXT: &str =
    "The quarterly report lists every invoice number and the matching payment date for each supplier in the region.";

/// Answers by segmentation mode: automatic layout gets `auto`, any other mode gets `sparse`.
struct ModeBackend {
    sparse: ExtractedDocument,
    auto: crate::Result<ExtractedDocument>,
    calls: AtomicUsize,
    last_auto_config: std::sync::Mutex<Option<OcrConfig>>,
}

#[async_trait::async_trait]
impl OcrBackend for ModeBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn confidence_semantics(&self) -> ConfidenceSemantics {
        ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let psm = config.tesseract_config.as_ref().and_then(|tesseract| tesseract.psm);
        if psm != Some(AUTO_PSM) {
            return Ok(self.sparse.clone());
        }
        *self.last_auto_config.lock().unwrap() = Some(config.clone());
        match &self.auto {
            Ok(document) => {
                // Like the real engine: the element selection of the request filters the elements.
                let mut document = document.clone();
                document.ocr_elements = match (config.element_config.as_ref(), document.ocr_elements.take()) {
                    (Some(selection), Some(elements)) => Some(selection.select_elements(&elements)),
                    _ => None,
                };
                Ok(document)
            }
            Err(error) => Err(crate::XbergError::Other(error.to_string())),
        }
    }
}

impl Plugin for ModeBackend {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

fn element(level: OcrElementLevel, width: u32, height: u32) -> OcrElement {
    OcrElement {
        text: "word".to_string(),
        level,
        geometry: OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width,
            height,
        },
        ..Default::default()
    }
}

fn word(width: u32, height: u32) -> OcrElement {
    element(OcrElementLevel::Word, width, height)
}

fn read(content: &str, confidence: f64, tall: bool) -> ExtractedDocument {
    let (width, height) = if tall { (10, 40) } else { (40, 10) };
    let mut document = ExtractedDocument {
        content: content.to_string(),
        ocr_elements: Some(vec![word(width, height); 5]),
        ..Default::default()
    };
    document
        .metadata
        .additional
        .insert("mean_text_conf".into(), serde_json::json!(confidence));
    document
}

/// A read whose words run `words_tall` and whose blocks and page box run `blocks_tall`.
fn read_levels(content: &str, confidence: f64, words_tall: bool, blocks_tall: bool) -> ExtractedDocument {
    let mut document = read(content, confidence, words_tall);
    let (width, height) = if blocks_tall { (100, 400) } else { (400, 100) };
    let elements = document.ocr_elements.as_mut().unwrap();
    elements.extend(vec![element(OcrElementLevel::Block, width, height); BLOCKS]);
    elements.push(element(OcrElementLevel::Page, width, height));
    document
}

fn backend(sparse: ExtractedDocument, auto: crate::Result<ExtractedDocument>) -> Arc<ModeBackend> {
    Arc::new(ModeBackend {
        sparse,
        auto,
        calls: AtomicUsize::new(0),
        last_auto_config: std::sync::Mutex::new(None),
    })
}

fn implicit_config() -> OcrConfig {
    OcrConfig {
        backend: "tesseract".to_string(),
        ..Default::default()
    }
}

fn sparse_config(original: &OcrConfig) -> OcrConfig {
    let mut effective = original.clone();
    effective.tesseract_config.get_or_insert_default().psm = Some(SPARSE_PSM);
    effective
}

async fn run(
    backend: &Arc<ModeBackend>,
    original: &OcrConfig,
    effective: &OcrConfig,
    whole_page_raster: bool,
) -> ExtractedDocument {
    let dynamic: Arc<dyn OcrBackend> = backend.clone();
    read_whole_page_raster(
        &dynamic,
        Arc::new(vec![0u8; 4]),
        original,
        effective,
        whole_page_raster,
        &OcrQualityThresholds::default(),
        None,
    )
    .await
    .expect("the read completes")
}

#[test]
fn fixture_texts_score_apart() {
    let thresholds = OcrQualityThresholds::default();
    assert!(compute_quality_score(CLEAN_TEXT, &thresholds) > compute_quality_score(NOISY_TEXT, &thresholds));
}

#[tokio::test]
async fn noisy_sideways_page_takes_the_automatic_layout_read() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn noisy_upright_page_stays_on_the_sparse_read() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, false)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, NOISY_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_page_without_the_noise_verdict_is_read_once() {
    let backend = backend(read(CLEAN_TEXT, 95.0, false), Ok(read(CLEAN_TEXT, 95.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_lower_scoring_retry_keeps_the_sparse_read() {
    let backend = backend(read(CLEAN_TEXT, 40.0, false), Ok(read(NOISY_TEXT, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}

/// A sideways, higher-scoring retry that reads fewer words than the first read has lost words.
#[tokio::test]
async fn a_retry_that_reads_fewer_words_keeps_the_sparse_read() {
    let thresholds = OcrQualityThresholds::default();
    let wordy = format!("{CLEAN_TEXT} {CLEAN_TEXT} {NOISY_TEXT} {NOISY_TEXT}");
    let short = "The quarterly report lists every invoice number.";
    assert!(compute_quality_score(short, &thresholds) > compute_quality_score(&wordy, &thresholds));
    assert!(super::sideways_retry::count_words(short) < super::sideways_retry::count_words(&wordy));
    let backend = backend(read(&wordy, 40.0, false), Ok(read(short, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, wordy);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_tied_score_keeps_the_sparse_read() {
    let mut sparse = read(CLEAN_TEXT, 40.0, false);
    sparse.content = format!("{CLEAN_TEXT}\n");
    let backend = backend(sparse.clone(), Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, sparse.content);
}

#[tokio::test]
async fn a_page_that_is_not_one_raster_is_read_once() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), false).await;
    assert_eq!(selected.content, NOISY_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_segmentation_mode_set_by_the_caller_is_never_overridden() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let mut original = implicit_config();
    original.tesseract_config.get_or_insert_default().psm = Some(SPARSE_PSM);
    let selected = run(&backend, &original, &original, true).await;
    assert_eq!(selected.content, NOISY_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failed_retry_keeps_the_sparse_read() {
    let backend = backend(
        read(NOISY_TEXT, 40.0, false),
        Err(crate::XbergError::Other("engine failed".to_string())),
    );
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, NOISY_TEXT);
}

#[tokio::test]
async fn word_boxes_are_dropped_when_the_caller_did_not_ask_for_them() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert!(selected.ocr_elements.is_none());
    let retry_config = backend.last_auto_config.lock().unwrap().clone().expect("a retry ran");
    assert!(
        retry_config
            .element_config
            .is_some_and(|elements| elements.include_elements)
    );
}

fn caller_selecting(min_level: OcrElementLevel) -> OcrConfig {
    let mut original = implicit_config();
    original.element_config = Some(OcrElementConfig {
        include_elements: true,
        min_level,
        ..Default::default()
    });
    original
}

#[tokio::test]
async fn block_only_output_does_not_make_tall_upright_paragraphs_sideways() {
    let backend = backend(
        read(NOISY_TEXT, 40.0, false),
        Ok(read_levels(CLEAN_TEXT, 90.0, false, true)),
    );
    let original = caller_selecting(OcrElementLevel::Block);
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, NOISY_TEXT);
}

#[tokio::test]
async fn page_only_output_still_recognises_a_sideways_page() {
    let backend = backend(
        read(NOISY_TEXT, 40.0, false),
        Ok(read_levels(CLEAN_TEXT, 90.0, true, false)),
    );
    let original = caller_selecting(OcrElementLevel::Page);
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
}

#[tokio::test]
async fn the_caller_element_selection_is_restored_on_the_kept_read() {
    let backend = backend(
        read(NOISY_TEXT, 40.0, false),
        Ok(read_levels(CLEAN_TEXT, 90.0, true, false)),
    );
    let original = caller_selecting(OcrElementLevel::Block);
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    let levels: Vec<_> = selected.ocr_elements.iter().flatten().map(|e| e.level).collect();
    assert_eq!(
        levels.len(),
        BLOCKS + 1,
        "the blocks and the page box, no words: {levels:?}"
    );
    assert!(levels.iter().all(|level| *level != OcrElementLevel::Word));
}

#[tokio::test]
async fn the_detector_gets_unfiltered_words_for_every_caller_selection() {
    for selection in [None, Some(OcrElementLevel::Word), Some(OcrElementLevel::Page)] {
        let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
        let original = selection.map_or_else(implicit_config, caller_selecting);
        run(&backend, &original, &sparse_config(&original), true).await;
        let request = backend.last_auto_config.lock().unwrap().clone().expect("a retry ran");
        let elements = request.element_config.expect("the request selects elements");
        assert!(elements.include_elements, "{selection:?}");
        assert_eq!(elements.min_level, OcrElementLevel::Word, "{selection:?}");
        assert_eq!(elements.min_confidence, 0.0, "{selection:?}");
    }
}

#[tokio::test]
async fn word_boxes_are_kept_when_the_caller_asked_for_them() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let mut original = implicit_config();
    original.element_config = Some(OcrElementConfig {
        include_elements: true,
        ..Default::default()
    });
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert!(selected.ocr_elements.is_some());
}

#[tokio::test]
async fn the_warnings_of_the_sparse_read_survive_the_switch() {
    let mut sparse = read(NOISY_TEXT, 40.0, false);
    sparse.processing_warnings.push(crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Borrowed("suspected recognition noise"),
    });
    let backend = backend(sparse, Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert!(
        selected
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("suspected recognition noise"))
    );
}

/// Restores the built-in backends when a test that replaced them ends.
struct RestoreBuiltins;

impl Drop for RestoreBuiltins {
    fn drop(&mut self) {
        let _ = crate::plugins::clear_ocr_backends();
        crate::plugins::ensure_ocr_backends_initialized();
    }
}

fn install(backend: &Arc<ModeBackend>) -> RestoreBuiltins {
    crate::plugins::ensure_ocr_backends_initialized();
    crate::plugins::clear_ocr_backends().unwrap();
    let restore = RestoreBuiltins;
    crate::plugins::register_ocr_backend(backend.clone()).unwrap();
    restore
}

fn ocr_extraction_config() -> crate::core::config::ExtractionConfig {
    crate::core::config::ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    }
}

/// The PDF page route reads a page that is one full-page raster through the retry.
#[cfg(feature = "pdf")]
#[tokio::test]
#[serial_test::serial]
async fn the_mixed_pdf_route_retries_a_noisy_scanned_page() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let _restore = install(&backend);
    let pdf = crate::pdf::render::build_full_page_raster_pdf((612.0, 792.0), (600, 800), 1.0, 0);
    let boundaries = vec![crate::types::PageBoundary {
        byte_start: 0,
        byte_end: 0,
        page_number: 1,
    }];

    let result = super::pipeline::extract_mixed_ocr_native("", &boundaries, &[1], &pdf, &ocr_extraction_config(), None)
        .await
        .expect("the page route completes");

    assert!(
        result.0.contains(CLEAN_TEXT),
        "the page keeps the automatic-layout read: {}",
        result.0
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}

/// The whole-document route reads a page it knows to be one full-page raster through the retry.
#[tokio::test]
#[serial_test::serial]
async fn the_whole_document_route_retries_a_noisy_scanned_page() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let _restore = install(&backend);
    let images = [image::DynamicImage::new_rgb8(600, 800)];
    let hints = super::PageOcrHints {
        source_dpi: None,
        known_full_page_scan: true,
        single_block_pages: None,
    };

    let result = super::extract_with_ocr_with_page_hints(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &ocr_extraction_config(),
        None,
        Some(hints),
    )
    .await
    .expect("the whole-document route completes");

    assert!(
        result.0.contains(CLEAN_TEXT),
        "the page keeps the automatic-layout read: {}",
        result.0
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}
