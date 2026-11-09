use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use xberg::plugins::{OcrBackend, OcrBackendType, Plugin};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig};

const READ: &str = "The document backend preserves the complete source context and returns ordinary readable words without splitting the document into independent page requests.";

struct DocumentBackend {
    name: &'static str,
    documents: AtomicUsize,
    images: AtomicUsize,
}

impl Plugin for DocumentBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".into()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

fn recognized_document() -> ExtractedDocument {
    let mut result = ExtractedDocument::default();
    result.content = READ.into();
    result.mime_type = "text/plain".into();
    result
        .metadata
        .additional
        .insert("mean_text_conf".into(), serde_json::json!(95));
    result
        .metadata
        .additional
        .insert("word_count".into(), serde_json::json!(READ.split_whitespace().count()));
    result
}

#[async_trait::async_trait]
impl OcrBackend for DocumentBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn supports_document_processing(&self) -> bool {
        true
    }
    fn confidence_semantics(&self) -> xberg::plugins::ConfidenceSemantics {
        xberg::plugins::ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }
    async fn process_image(&self, _bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.images.fetch_add(1, Ordering::Relaxed);
        Ok(recognized_document())
    }
    async fn process_document(&self, path: &std::path::Path, _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        assert!(path.is_file(), "the backend receives the original PDF path");
        self.documents.fetch_add(1, Ordering::Relaxed);
        Ok(recognized_document())
    }
}

struct RegisteredBackend(Arc<DocumentBackend>);

impl Drop for RegisteredBackend {
    fn drop(&mut self) {
        xberg::plugins::unregister_ocr_backend(self.0.name).expect("remove the test-owned backend");
    }
}

fn scope_config(name: &'static str, explicit_pipeline: bool) -> ExtractionConfig {
    let pipeline = explicit_pipeline.then(|| {
        serde_json::from_value(serde_json::json!({"stages": [{"backend": name, "priority": 1}]}))
            .expect("a valid explicit OCR pipeline")
    });
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            enabled: true,
            backend: if explicit_pipeline { "tesseract" } else { name }.into(),
            pipeline,
            ..Default::default()
        }),
        pdf_options: Some(xberg::PdfConfig {
            top_margin_fraction: Some(0.0),
            bottom_margin_fraction: Some(0.0),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn assert_document_scope(name: &'static str, explicit_pipeline: bool) {
    if super::helpers::skip_if_missing(super::SOURCE) {
        return;
    }
    let backend = Arc::new(DocumentBackend {
        name,
        documents: AtomicUsize::new(0),
        images: AtomicUsize::new(0),
    });
    xberg::plugins::register_ocr_backend(backend.clone()).unwrap();
    let registered = RegisteredBackend(backend);
    let file = tempfile::Builder::new().suffix(".pdf").tempfile().unwrap();
    std::fs::write(file.path(), super::excerpt(&[super::SOURCE_PAGE])).unwrap();
    let config = scope_config(name, explicit_pipeline);
    let document = super::helpers::extract_uri_document(file.path(), Some("application/pdf"), &config)
        .await
        .expect("extract the pinned public page through the selected backend");
    assert!(!document.content.trim().is_empty());
    let documents = registered.0.documents.load(Ordering::Relaxed);
    let images = registered.0.images.load(Ordering::Relaxed);
    assert!(
        documents > 0 || images > 0,
        "automatic OCR must invoke the observed backend"
    );
    assert_eq!(
        documents, 1,
        "one original-document request; detached image requests: {images}"
    );
    assert_eq!(images, 0, "no detached image request");
}

#[tokio::test]
async fn custom_document_backend_keeps_the_original_pdf_request() {
    assert_document_scope("all-pages-document-scope-2768", false).await;
}

#[tokio::test]
async fn explicit_pipeline_keeps_the_original_pdf_context() {
    assert_document_scope("all-pages-pipeline-scope-2768", true).await;
}
