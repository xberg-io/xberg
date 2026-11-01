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
    /// Cancelled by the backend while it answers the automatic-layout read, as a caller would mid-retry.
    cancel_during_retry: std::sync::Mutex<Option<crate::cancellation::CancellationToken>>,
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
        if let Some(token) = self.cancel_during_retry.lock().unwrap().as_ref() {
            token.cancel();
        }
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

/// A markdown read: its words, and the internal document the engine builds for markdown output.
fn read(content: &str, confidence: f64, tall: bool) -> ExtractedDocument {
    let (width, height) = if tall { (10, 40) } else { (40, 10) };
    let mut document = ExtractedDocument {
        content: content.to_string(),
        ocr_elements: Some(vec![word(width, height); 5]),
        ocr_internal_document: Some(crate::types::internal::InternalDocument::new("ocr")),
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
        cancel_during_retry: std::sync::Mutex::new(None),
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
        None,
    )
    .await
    .expect("the read completes")
}

async fn run_with_token(
    backend: &Arc<ModeBackend>,
    token: &crate::cancellation::CancellationToken,
) -> crate::Result<ExtractedDocument> {
    let original = implicit_config();
    let dynamic: Arc<dyn OcrBackend> = backend.clone();
    read_whole_page_raster(
        &dynamic,
        Arc::new(vec![0u8; 4]),
        &original,
        &sparse_config(&original),
        true,
        &OcrQualityThresholds::default(),
        Some(token),
        None,
    )
    .await
}

fn cancelling(backend: &Arc<ModeBackend>) -> crate::cancellation::CancellationToken {
    let token = crate::cancellation::CancellationToken::new();
    *backend.cancel_during_retry.lock().unwrap() = Some(token.clone());
    token
}

#[tokio::test]
async fn a_cancel_during_a_retry_that_would_be_selected_is_reported() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let token = cancelling(&backend);
    let result = run_with_token(&backend, &token).await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)), "{result:?}");
}

#[tokio::test]
async fn a_cancel_during_a_retry_that_would_be_rejected_is_reported() {
    let backend = backend(read(CLEAN_TEXT, 40.0, false), Ok(read(NOISY_TEXT, 90.0, true)));
    let token = cancelling(&backend);
    let result = run_with_token(&backend, &token).await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)), "{result:?}");
}

#[tokio::test]
async fn a_cancel_during_a_retry_that_fails_is_reported() {
    let backend = backend(
        read(NOISY_TEXT, 40.0, false),
        Err(crate::XbergError::Other("engine failed".to_string())),
    );
    let token = cancelling(&backend);
    let result = run_with_token(&backend, &token).await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)), "{result:?}");
}

#[tokio::test]
async fn an_uncancelled_token_lets_the_retry_be_selected() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let token = crate::cancellation::CancellationToken::new();
    let selected = run_with_token(&backend, &token).await.expect("the read completes");
    assert_eq!(selected.content, CLEAN_TEXT);
    assert!(!token.is_cancelled());
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

/// A sideways, higher-scoring retry that still carries the noise verdict is not a better read.
#[tokio::test]
async fn a_retry_that_is_still_noise_keeps_the_sparse_read() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 30.0, true)));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, NOISY_TEXT);
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

/// A first-read paragraph at `left`, `top`, as the hOCR parser builds it, block id included.
fn paragraph(text: &str, confidence: f64, left: u32, top: u32) -> crate::types::internal::InternalElement {
    let mut paragraph = crate::types::internal::InternalElement::text(
        crate::types::internal::ElementKind::OcrText {
            level: OcrElementLevel::Block,
        },
        text,
        0,
    )
    .with_page(1);
    paragraph.ocr_geometry = Some(OcrBoundingGeometry::Rectangle {
        left,
        top,
        width: 60,
        height: 28,
    });
    paragraph.ocr_confidence = Some(crate::types::OcrConfidence {
        detection: None,
        recognition: confidence,
    });
    paragraph.attributes = Some(
        [(
            crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE.to_string(),
            "block_1_1".to_string(),
        )]
        .into_iter()
        .collect(),
    );
    paragraph
}

fn with_paragraphs(
    mut document: ExtractedDocument,
    paragraphs: Vec<crate::types::internal::InternalElement>,
) -> ExtractedDocument {
    let mut internal = crate::types::internal::InternalDocument::new("ocr");
    for paragraph in paragraphs {
        internal.push_element(paragraph);
    }
    document.ocr_internal_document = Some(internal);
    document
}

#[tokio::test]
async fn a_page_number_only_the_sparse_read_holds_stays_in_the_page() {
    let sparse = with_paragraphs(
        read(NOISY_TEXT, 40.0, false),
        vec![paragraph(NOISY_TEXT, 0.40, 0, 0), paragraph("417", 0.97, 900, 1500)],
    );
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n417"));
    let internal = selected
        .ocr_internal_document
        .expect("the kept read keeps its document");
    let texts: Vec<_> = internal.elements.iter().map(|element| element.text.as_str()).collect();
    assert_eq!(texts, [CLEAN_TEXT, "417"]);
    assert!(
        internal.elements[1]
            .attributes
            .as_ref()
            .is_none_or(|attributes| !attributes.contains_key(crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE)),
        "the carried paragraph must not join a block of the kept read"
    );
}

/// Runs a noisy sparse read holding `paragraphs` against a clean sideways retry whose words sit at
/// 0, 0, 10 wide and 40 tall, and returns the kept read.
async fn carry(paragraphs: Vec<crate::types::internal::InternalElement>) -> ExtractedDocument {
    let sparse = with_paragraphs(read(NOISY_TEXT, 40.0, false), paragraphs);
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    run(&backend, &original, &sparse_config(&original), true).await
}

fn texts(document: ExtractedDocument) -> Vec<String> {
    document
        .ocr_internal_document
        .expect("the kept read keeps its document")
        .elements
        .iter()
        .map(|element| element.text.clone())
        .collect()
}

#[tokio::test]
async fn a_two_digit_page_number_only_the_sparse_read_holds_stays_in_the_page() {
    let selected = carry(vec![
        paragraph(NOISY_TEXT, 0.40, 0, 0),
        paragraph("29", 0.97, 900, 1500),
    ])
    .await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n29"));
    assert_eq!(texts(selected), [CLEAN_TEXT, "29"]);
}

#[tokio::test]
async fn a_word_the_sparse_read_holds_at_low_confidence_stays_in_the_page() {
    let selected = carry(vec![paragraph("Appendix", 0.75, 900, 1500)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nAppendix"));
    assert_eq!(texts(selected), [CLEAN_TEXT, "Appendix"]);
}

#[tokio::test]
async fn text_under_the_kept_words_is_not_carried() {
    let selected = carry(vec![
        paragraph("Hxqzvk", 0.95, 0, 0),
        paragraph("quarterly", 0.95, 9, 39),
    ])
    .await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert_eq!(texts(selected), [CLEAN_TEXT]);
}

#[tokio::test]
async fn a_paragraph_that_only_touches_a_kept_word_box_is_carried() {
    let selected = carry(vec![paragraph("Margin", 0.95, 10, 0), paragraph("Footer", 0.95, 0, 40)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nMargin\n\nFooter"));
    assert_eq!(texts(selected), [CLEAN_TEXT, "Margin", "Footer"]);
}

/// Text, hOCR and TSV reads build no internal document, so the carry cannot see the first read's
/// paragraphs: the first read stays and none of its text is lost.
#[tokio::test]
async fn a_read_without_paragraphs_keeps_the_sparse_read() {
    for format in ["text", "hocr", "tsv"] {
        let mut sparse = read("29 ab cd ef gh ij kl mn op qr st uv wx", 40.0, false);
        sparse.ocr_internal_document = None;
        let mut auto = read(CLEAN_TEXT, 90.0, true);
        auto.ocr_internal_document = None;
        let backend = backend(sparse.clone(), Ok(auto));
        let mut original = implicit_config();
        original.tesseract_config.get_or_insert_default().output_format = format.to_string();
        let selected = run(&backend, &original, &sparse_config(&original), true).await;
        assert_eq!(selected.content, sparse.content, "{format}: the first read stays");
        assert_eq!(backend.calls.load(Ordering::SeqCst), 1, "{format}: no second read runs");
    }
}

#[tokio::test]
async fn a_retry_without_paragraphs_keeps_the_sparse_read() {
    let sparse = with_paragraphs(read(NOISY_TEXT, 40.0, false), vec![paragraph("29", 0.97, 900, 1500)]);
    let mut auto = read(CLEAN_TEXT, 90.0, true);
    auto.ocr_internal_document = None;
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, NOISY_TEXT);
}

#[tokio::test]
async fn the_retry_keeps_the_caller_preprocessing() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let mut original = implicit_config();
    let caller = crate::types::ImagePreprocessingConfig {
        target_dpi: 600,
        ..Default::default()
    };
    original.tesseract_config.get_or_insert_default().preprocessing = Some(caller);
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    let retry = backend.last_auto_config.lock().unwrap().clone().expect("the retry ran");
    let preprocessing = retry.tesseract_config.and_then(|tesseract| tesseract.preprocessing);
    assert_eq!(
        preprocessing.map(|preprocessing| (preprocessing.target_dpi, preprocessing.deskew)),
        Some((600, true)),
        "the retry reads the page in the caller's pixel frame"
    );
}

#[tokio::test]
async fn without_caller_preprocessing_the_retry_uses_the_sparse_fallback_preprocessing() {
    let backend = backend(read(NOISY_TEXT, 40.0, false), Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    run(&backend, &original, &sparse_config(&original), true).await;
    let retry = backend.last_auto_config.lock().unwrap().clone().expect("the retry ran");
    let preprocessing = retry.tesseract_config.and_then(|tesseract| tesseract.preprocessing);
    assert_eq!(
        preprocessing.map(|preprocessing| (preprocessing.target_dpi, preprocessing.deskew)),
        Some((300, false))
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
        native_page_layers: None,
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

/// The ninth word: a dictionary word of the first read, at low confidence, under a kept word box.
#[tokio::test]
async fn a_dictionary_word_under_a_kept_box_joins_the_kept_read() {
    let selected = carry(vec![paragraph("xqzvk window", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nwindow"));
    assert_eq!(texts(selected), [CLEAN_TEXT, "window"]);
}

#[tokio::test]
async fn a_number_under_a_kept_box_joins_the_kept_read() {
    let selected = carry(vec![paragraph("xqzvk A-417", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nA-417"));
}

/// Runs a noisy sparse read that holds `paragraphs` and one table with `cells` against the clean retry.
async fn carry_with_table(
    paragraphs: Vec<crate::types::internal::InternalElement>,
    cells: &[&[&str]],
) -> ExtractedDocument {
    let mut sparse = with_paragraphs(read(NOISY_TEXT, 40.0, false), paragraphs);
    let mut table = crate::types::Table::default();
    for row in cells {
        table.cells.push(row.iter().map(|cell| cell.to_string()).collect());
    }
    sparse.tables = vec![table];
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    run(&backend, &original, &sparse_config(&original), true).await
}

const PRINTED_TABLE: &str = "| ab | A-417 |\n| --- | --- |";

/// A noisy sparse read whose text prints its table, as Tesseract writes a table into the markdown text.
fn sparse_with_printed_table(paragraphs: Vec<crate::types::internal::InternalElement>) -> ExtractedDocument {
    let content = format!("{NOISY_TEXT}\n\n{PRINTED_TABLE}");
    let mut sparse = with_paragraphs(read(&content, 40.0, false), paragraphs);
    let mut table = crate::types::Table::default();
    table.cells.push(vec!["ab".to_string(), "A-417".to_string()]);
    table.markdown = PRINTED_TABLE.to_string();
    sparse.tables = vec![table];
    sparse
}

async fn carry_printed_table(paragraphs: Vec<crate::types::internal::InternalElement>) -> ExtractedDocument {
    keep_clean_read_over(sparse_with_printed_table(paragraphs)).await
}

async fn keep_clean_read_over(sparse: ExtractedDocument) -> ExtractedDocument {
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    run(&backend, &original, &sparse_config(&original), true).await
}

/// The page text comes from the text of the read, so a table that text prints joins the kept text.
#[tokio::test]
async fn a_table_the_first_read_prints_stays_in_the_kept_text() {
    let selected = carry_printed_table(Vec::new()).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n{PRINTED_TABLE}"));
    assert_eq!(selected.tables.len(), 1);
    assert_eq!(selected.tables[0].markdown, PRINTED_TABLE);
}

/// The first read holds the number twice, in a paragraph and in the table, so the kept text does too.
#[tokio::test]
async fn a_number_of_a_paragraph_and_a_printed_table_stays_twice() {
    let selected = carry_printed_table(vec![paragraph("xqzvk A-417", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nA-417\n\n{PRINTED_TABLE}"));
}

/// The PDF page route takes the page text from the text of the kept read.
#[cfg(feature = "pdf")]
#[tokio::test]
#[serial_test::serial]
async fn the_mixed_pdf_route_prints_a_table_of_the_first_read_in_the_page() {
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let sparse = sparse_with_printed_table(vec![paragraph(NOISY_TEXT, 0.40, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
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

    assert!(result.0.contains(CLEAN_TEXT), "the second read is kept: {}", result.0);
    assert!(result.0.contains(PRINTED_TABLE), "the table prints: {}", result.0);
    assert!(
        result.1[&1].contains(PRINTED_TABLE),
        "the page holds it: {}",
        result.1[&1]
    );
    assert_eq!(result.2[&1].tables.len(), 1, "the table of the first read stays");
}

/// Only a table the first read prints joins the text. The table stays in the tables of the kept read.
#[tokio::test]
async fn a_table_with_markdown_that_the_first_text_lacks_stays_out_of_the_kept_text() {
    let mut sparse = sparse_with_printed_table(Vec::new());
    sparse.content = NOISY_TEXT.to_string();
    let selected = keep_clean_read_over(sparse).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert_eq!(selected.tables.len(), 1);
}

/// A table with no markdown adds no text, and no blank block, to the kept text.
#[tokio::test]
async fn a_table_with_no_markdown_stays_out_of_the_kept_text() {
    let selected = carry_with_table(Vec::new(), &[&["ab", "A-417"]]).await;
    assert_eq!(selected.content, CLEAN_TEXT);
    assert_eq!(selected.tables.len(), 1);
    assert_eq!(selected.tables[0].cells, [["ab", "A-417"]]);
}

/// The kept read gives the table to the caller, beside the words of the kept read.
#[tokio::test]
#[serial_test::serial]
async fn a_table_of_the_first_read_reaches_the_caller() {
    let sparse = sparse_with_printed_table(vec![paragraph(NOISY_TEXT, 0.40, 0, 0)]);
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let _restore = install(&backend);
    let images = [image::DynamicImage::new_rgb8(600, 800)];
    let hints = super::PageOcrHints {
        source_dpi: None,
        known_full_page_scan: true,
        single_block_pages: None,
        native_page_layers: None,
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

    assert!(result.0.contains(CLEAN_TEXT), "the second read is kept: {}", result.0);
    assert_eq!(result.2.len(), 1, "the table of the first read stays");
    assert_eq!(result.2[0].cells, [["ab", "A-417"]]);
    assert!(
        result.6[0].contains(PRINTED_TABLE),
        "the page text prints it: {}",
        result.6[0]
    );
}

/// A table of each read stays.
#[tokio::test]
async fn the_tables_of_both_reads_stay() {
    let mut sparse = with_paragraphs(read(NOISY_TEXT, 40.0, false), Vec::new());
    let mut first_table = crate::types::Table::default();
    first_table.cells.push(vec!["first".to_string()]);
    sparse.tables = vec![first_table];
    let mut auto = with_paragraphs(read(CLEAN_TEXT, 90.0, true), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let mut second_table = crate::types::Table::default();
    second_table.cells.push(vec!["second".to_string()]);
    auto.tables = vec![second_table];
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    let cells: Vec<_> = selected.tables.iter().map(|table| table.cells[0][0].as_str()).collect();
    assert_eq!(cells, ["second", "first"]);
}

/// With a rejected second read the first read stays whole, with its one table.
#[tokio::test]
async fn a_rejected_second_read_keeps_one_table() {
    let mut sparse = with_paragraphs(read(NOISY_TEXT, 40.0, false), Vec::new());
    sparse.tables = vec![crate::types::Table::default()];
    let auto = with_paragraphs(read(CLEAN_TEXT, 90.0, false), vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, NOISY_TEXT);
    assert_eq!(selected.tables.len(), 1);
}

/// A date splits into digit groups under three characters, and it stays one word.
#[tokio::test]
async fn a_date_of_short_digit_groups_under_a_kept_box_joins_whole() {
    let selected = carry(vec![paragraph("xqzvk 12/05/24", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n12/05/24"));
}

#[tokio::test]
async fn a_time_under_a_kept_box_joins_whole() {
    let selected = carry(vec![paragraph("xqzvk 10:30.", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n10:30"));
}

/// A number in the digits of another script is a number: the dictionary does not judge it.
#[tokio::test]
async fn a_number_in_arabic_digits_under_a_kept_box_joins() {
    let selected = carry(vec![paragraph("xqzvk \u{0664}\u{0661}\u{0667}", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n\u{0664}\u{0661}\u{0667}"));
}

#[tokio::test]
async fn a_number_the_first_read_holds_twice_joins_twice() {
    let selected = carry(vec![paragraph("xqzvk A-417 xqzvk A-417", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nA-417 A-417"));
}

/// The kept read holds the number once, so one of the three of the first read is held.
#[tokio::test]
async fn a_repeated_number_joins_only_what_the_kept_read_lacks() {
    let sparse = with_paragraphs(
        read(NOISY_TEXT, 40.0, false),
        vec![paragraph("A-417 A-417 A-417", 0.39, 0, 0)],
    );
    let kept = format!("{CLEAN_TEXT} A-417");
    let auto = with_paragraphs(read(&kept, 90.0, true), vec![paragraph(&kept, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, format!("{kept}\n\nA-417 A-417"));
}

/// A word counts once, however often the first read holds it.
#[tokio::test]
async fn a_repeated_word_joins_once() {
    let selected = carry(vec![paragraph("xqzvk window window", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nwindow"));
}

/// A date the kept read holds is held, whatever marks stand around it.
#[tokio::test]
async fn a_date_the_kept_read_holds_does_not_join() {
    let sparse = with_paragraphs(read(NOISY_TEXT, 40.0, false), vec![paragraph("(12/05/24)", 0.39, 0, 0)]);
    let kept = format!("{CLEAN_TEXT} 12/05/24.");
    let auto = with_paragraphs(read(&kept, 90.0, true), vec![paragraph(&kept, 0.90, 0, 0)]);
    let backend = backend(sparse, Ok(auto));
    let original = implicit_config();
    let selected = run(&backend, &original, &sparse_config(&original), true).await;
    assert_eq!(selected.content, kept);
}

/// A token with no digit still splits on a slash: each side is a word for the dictionary.
#[tokio::test]
async fn a_word_pair_splits_at_its_slash() {
    let selected = carry(vec![paragraph("window/thunder", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\nwindow thunder"));
}

/// Case and punctuation do not make a word that the kept read holds a new word, and a word of two
/// characters stays out.
#[tokio::test]
async fn a_word_the_kept_read_holds_or_a_short_word_does_not_join() {
    let selected = carry(vec![paragraph("INVOICE, (region). of", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, CLEAN_TEXT);
}

/// A number joins at any length.
#[tokio::test]
async fn a_number_of_one_or_two_characters_under_a_kept_box_joins() {
    let selected = carry(vec![paragraph("xqzvk 42 7", 0.39, 0, 0)]).await;
    assert_eq!(selected.content, format!("{CLEAN_TEXT}\n\n42 7"));
}

/// A second stage that a test scripts: it returns `read` for every call.
struct LaterStage {
    read: ExtractedDocument,
    calls: AtomicUsize,
}

#[async_trait::async_trait]
impl OcrBackend for LaterStage {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn confidence_semantics(&self) -> ConfidenceSemantics {
        ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.read.clone())
    }
}

impl Plugin for LaterStage {
    fn name(&self) -> &str {
        "paddleocr"
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

/// Runs a two-stage pipeline over one full-page raster: `first` reads noise, `later` reads `read`.
/// Returns the text, the calls of the first backend and the calls of the later stage.
async fn two_stage_run(auto: crate::Result<ExtractedDocument>, read: ExtractedDocument) -> (String, usize, usize) {
    // ~keep The bar sits over the stage score of the noise, as on a real page that needs the later stage.
    let thresholds = OcrQualityThresholds {
        pipeline_min_quality: 0.9,
        ..Default::default()
    };
    two_stage_run_with(auto, read, thresholds).await
}

async fn two_stage_run_with(
    auto: crate::Result<ExtractedDocument>,
    read: ExtractedDocument,
    thresholds: OcrQualityThresholds,
) -> (String, usize, usize) {
    let first = backend(
        with_paragraphs(read_noise(), vec![paragraph(NOISY_TEXT, 0.40, 0, 0)]),
        auto,
    );
    let _restore = install(&first);
    let later = Arc::new(LaterStage {
        read,
        calls: AtomicUsize::new(0),
    });
    crate::plugins::register_ocr_backend(later.clone()).unwrap();
    let stage = |backend: &str, priority: u32| crate::core::config::OcrPipelineStage {
        backend: backend.to_string(),
        priority,
        language: None,
        tesseract_config: None,
        paddle_ocr_config: None,
        paddle_ocr_settings: None,
        vlm_config: None,
        backend_options: None,
    };
    let pipeline = crate::core::config::OcrPipelineConfig {
        stages: vec![stage("tesseract", 100), stage("paddleocr", 50)],
        quality_thresholds: thresholds,
    };
    let images = [image::DynamicImage::new_rgb8(600, 800)];
    let hints = super::PageOcrHints {
        source_dpi: None,
        known_full_page_scan: true,
        single_block_pages: None,
        native_page_layers: None,
    };
    let result = super::run_ocr_pipeline(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &ocr_extraction_config(),
        &pipeline,
        None,
        Some(hints),
    )
    .await
    .expect("the pipeline completes");
    (
        result.0,
        first.calls.load(Ordering::SeqCst),
        later.calls.load(Ordering::SeqCst),
    )
}

fn read_noise() -> ExtractedDocument {
    read(NOISY_TEXT, 40.0, false)
}

const LATER_TEXT: &str =
    "The later stage reads every line of this page and returns each sentence of the region report in order.";

#[tokio::test]
#[serial_test::serial]
async fn a_later_stage_that_reads_the_page_clean_keeps_it_with_no_second_read() {
    let (text, first_calls, later_calls) =
        two_stage_run(Ok(read(CLEAN_TEXT, 90.0, true)), read(LATER_TEXT, 95.0, false)).await;
    assert_eq!(text.trim(), LATER_TEXT);
    assert_eq!((first_calls, later_calls), (1, 1), "no second read runs");
}

/// The second read runs after the last stage, and the kept read goes through the stage once more
/// without a further read of the page.
#[tokio::test]
#[serial_test::serial]
async fn the_second_read_runs_after_a_later_stage_that_still_reads_noise() {
    let (text, first_calls, later_calls) = two_stage_run(
        Ok(with_paragraphs(
            read(CLEAN_TEXT, 90.0, true),
            vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)],
        )),
        read("mn op qr st uv wx ab cd ef gh ij kl", 30.0, false),
    )
    .await;
    assert!(text.contains(CLEAN_TEXT), "the second read is kept: {text}");
    assert_eq!(
        (first_calls, later_calls),
        (2, 1),
        "the later stage ran, and no read ran twice"
    );
}

/// The word floor compares with the read the pipeline returned, here the read of the later stage.
/// Against the noise of the first stage, the same second read passes.
#[tokio::test]
#[serial_test::serial]
async fn the_gates_compare_with_the_returned_read() {
    let returned = format!("{CLEAN_TEXT} {LATER_TEXT}");
    let (text, first_calls, later_calls) = two_stage_run(
        Ok(with_paragraphs(
            read(CLEAN_TEXT, 90.0, true),
            vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)],
        )),
        read(&returned, 40.0, false),
    )
    .await;
    assert_eq!(text.trim(), returned);
    assert_eq!((first_calls, later_calls), (2, 1));
}

/// A retry that fails leaves the returned read of the pipeline.
#[tokio::test]
#[serial_test::serial]
async fn a_failed_second_read_after_the_stages_keeps_the_returned_read() {
    let (text, first_calls, _) = two_stage_run(
        Err(crate::XbergError::Other("no".to_string())),
        read("ab cd ef gh", 30.0, false),
    )
    .await;
    assert!(text.contains("ab cd"), "a stage read stays: {text}");
    assert_eq!(first_calls, 2);
}

/// With the discard setting the page keeps the value of the base: no second read.
#[tokio::test]
async fn the_discard_setting_runs_no_second_read() {
    let backend = backend(read_noise(), Ok(read(CLEAN_TEXT, 90.0, true)));
    let original = implicit_config();
    let dynamic: Arc<dyn OcrBackend> = backend.clone();
    let thresholds = OcrQualityThresholds {
        discard_suspected_ocr_noise: true,
        ..Default::default()
    };
    let selected = read_whole_page_raster(
        &dynamic,
        Arc::new(vec![0u8; 4]),
        &original,
        &sparse_config(&original),
        true,
        &thresholds,
        None,
        None,
    )
    .await
    .expect("the read completes");
    assert_eq!(selected.content, NOISY_TEXT);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
}

/// A stage that the pipeline accepts ends the run, so its verdict page takes the second read there.
#[tokio::test]
#[serial_test::serial]
async fn an_accepted_stage_with_the_verdict_takes_the_second_read() {
    let (text, first_calls, later_calls) = two_stage_run_with(
        Ok(with_paragraphs(
            read(CLEAN_TEXT, 90.0, true),
            vec![paragraph(CLEAN_TEXT, 0.90, 0, 0)],
        )),
        read(LATER_TEXT, 95.0, false),
        OcrQualityThresholds {
            pipeline_min_quality: 0.0,
            ..Default::default()
        },
    )
    .await;
    assert!(text.contains(CLEAN_TEXT), "the second read is kept: {text}");
    assert_eq!((first_calls, later_calls), (2, 0), "no later stage runs");
}

#[test]
fn a_word_without_a_verdict_joins_and_a_rejected_word_does_not() {
    let missed = vec![
        (0, "kept", false),
        (0, "A-12", true),
        (1, "gone", false),
        (2, "open", false),
    ];
    let joined = super::sideways_retry::join_verdicts(missed, vec![Some(true), Some(false), None]);
    assert_eq!(joined, [(0, "kept"), (0, "A-12"), (2, "open")]);
}
