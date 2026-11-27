//! The extraction's Tesseract model set reaches the OCR backend through the real entry points.

use crate::core::config::{ExtractionConfig, OcrConfig, TessdataModel};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use std::sync::{Arc, Mutex};

const PDF: &[u8] = include_bytes!("../../../tests/fixtures/ocr/rotated_financial_table.pdf");

#[derive(Default)]
struct CapturingBackend {
    seen: Mutex<Vec<Option<TessdataModel>>>,
}

impl Plugin for CapturingBackend {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".into()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for CapturingBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.seen.lock().unwrap().push(config.tessdata_model);
        Ok(ExtractedDocument {
            content: "Captured OCR text for the model set check.".into(),
            ..Default::default()
        })
    }
}

fn config(ocr: Option<OcrConfig>) -> ExtractionConfig {
    ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        tessdata_model: Some(TessdataModel::Best),
        ocr,
        ..Default::default()
    }
}

async fn seen_models(config: &ExtractionConfig, from_file: bool) -> Vec<Option<TessdataModel>> {
    let backend = Arc::new(CapturingBackend::default());
    crate::plugins::registry::with_test_ocr_backend(backend.clone(), async {
        if from_file {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("scan.pdf");
            std::fs::write(&path, PDF).unwrap();
            super::extract_file(&path, Some("application/pdf"), config)
                .await
                .unwrap();
        } else {
            super::extract_bytes(PDF, "application/pdf", config).await.unwrap();
        }
    })
    .await;
    backend.seen.lock().unwrap().clone()
}

#[tokio::test]
async fn the_top_level_set_reaches_the_backend_with_and_without_an_ocr_section() {
    for ocr in [None, Some(OcrConfig::default())] {
        for from_file in [false, true] {
            let seen = seen_models(&config(ocr.clone()), from_file).await;
            assert!(!seen.is_empty(), "the page must be read by the capturing backend");
            assert!(
                seen.iter().all(|model| *model == Some(TessdataModel::Best)),
                "section {:?}, from_file {from_file}: {seen:?}",
                ocr.is_some()
            );
        }
    }
}

#[tokio::test]
async fn an_ocr_section_set_wins_over_the_top_level_set() {
    let own = OcrConfig {
        tessdata_model: Some(TessdataModel::Fast),
        ..Default::default()
    };
    let seen = seen_models(&config(Some(own)), false).await;
    assert!(!seen.is_empty());
    assert!(seen.iter().all(|model| *model == Some(TessdataModel::Fast)), "{seen:?}");
}
