//! The second read on the page route of a build with no async runtime, which reads pages in turn.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::plugins::{ConfidenceSemantics, OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};

const AUTO_PSM: i32 = 3;
const NOISY_TEXT: &str = "ab cd ef gh ij kl mn op qr st uv wx";
const CLEAN_TEXT: &str =
    "The quarterly report lists every invoice number and the matching payment date for each supplier in the region.";

/// Answers automatic layout with a clean sideways read, and any other mode with noise.
struct ModeBackend {
    calls: AtomicUsize,
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
        Ok(if psm == Some(AUTO_PSM) {
            read(CLEAN_TEXT, 90.0, (10, 40))
        } else {
            read(NOISY_TEXT, 40.0, (40, 10))
        })
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

fn read(content: &str, confidence: f64, (width, height): (u32, u32)) -> ExtractedDocument {
    let word = OcrElement {
        text: "word".to_string(),
        level: OcrElementLevel::Word,
        geometry: OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width,
            height,
        },
        ..Default::default()
    };
    let mut document = ExtractedDocument {
        content: content.to_string(),
        ocr_elements: Some(vec![word; 5]),
        ocr_internal_document: Some(crate::types::internal::InternalDocument::new("ocr")),
        ..Default::default()
    };
    document
        .metadata
        .additional
        .insert("mean_text_conf".into(), serde_json::json!(confidence));
    document
}

struct RestoreBuiltins;

impl Drop for RestoreBuiltins {
    fn drop(&mut self) {
        let _ = crate::plugins::clear_ocr_backends();
        crate::plugins::ensure_ocr_backends_initialized();
    }
}

#[test]
#[serial_test::serial]
fn the_sequential_pdf_route_retries_a_noisy_scanned_page() {
    let backend = Arc::new(ModeBackend {
        calls: AtomicUsize::new(0),
    });
    crate::plugins::ensure_ocr_backends_initialized();
    crate::plugins::clear_ocr_backends().unwrap();
    let _restore = RestoreBuiltins;
    crate::plugins::register_ocr_backend(backend.clone()).unwrap();
    let pdf = crate::pdf::render::build_full_page_raster_pdf((612.0, 792.0), (600, 800), 1.0, 0);
    let boundaries = vec![crate::types::PageBoundary {
        byte_start: 0,
        byte_end: 0,
        page_number: 1,
    }];
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };

    let run = futures::executor::block_on(super::extract_mixed_ocr_native_with_layout_pages(
        "",
        &boundaries,
        (&[1], &[]),
        &pdf,
        &config,
        None,
    ))
    .expect("the page route completes");

    assert!(
        run.output.0.contains(CLEAN_TEXT),
        "the page keeps the automatic-layout read: {}",
        run.output.0
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
}
