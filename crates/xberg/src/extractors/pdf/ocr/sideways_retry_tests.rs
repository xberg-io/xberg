use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::scoring::compute_quality_score;
use super::sideways_retry::read_whole_page_raster;
use crate::core::config::{OcrConfig, OcrQualityThresholds};
use crate::plugins::{ConfidenceSemantics, OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementConfig, OcrElementLevel};

const SPARSE_PSM: i32 = 11;
const NOISY_TEXT: &str = "ab cd ef gh ij kl mn op qr st uv wx";
const CLEAN_TEXT: &str =
    "The quarterly report lists every invoice number and the matching payment date for each supplier in the region.";

/// Answers by segmentation mode: the sparse-text read gets `sparse`, any other mode gets `auto`.
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
        if psm == Some(SPARSE_PSM) {
            return Ok(self.sparse.clone());
        }
        *self.last_auto_config.lock().unwrap() = Some(config.clone());
        match &self.auto {
            Ok(document) => Ok(document.clone()),
            Err(error) => Err(crate::XbergError::Other(error.to_string())),
        }
    }
}

impl Plugin for ModeBackend {
    fn name(&self) -> &str {
        "mode"
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

fn word(width: u32, height: u32) -> OcrElement {
    OcrElement {
        text: "word".to_string(),
        level: OcrElementLevel::Word,
        geometry: OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width,
            height,
        },
        ..Default::default()
    }
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
