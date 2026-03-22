//! Comprehensive OCR backend plugin system tests.
//!
//! Tests custom OCR backend registration, execution, parameter passing,
//! error handling, and backend switching with real image extraction.

#![cfg(feature = "ocr")]

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin};
use kreuzberg::types::{ExtractionResult, Metadata};
use kreuzberg::{KreuzbergError, Result, extract_file_sync};
use serial_test::serial;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct BackendRegistryGuard;

impl Drop for BackendRegistryGuard {
    fn drop(&mut self) {
        let registry = get_ocr_backend_registry();
        if let Ok(mut reg) = registry.write() {
            let _ = reg.shutdown_all();
        }
    }
}


struct MockOcrBackend {
    name: String,
    return_text: String,
    call_count: AtomicUsize,
    last_language: Mutex<String>,
    initialized: AtomicBool,
}

impl Plugin for MockOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        self.initialized.store(false, Ordering::Release);
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for MockOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        *self.last_language.lock().expect("Operation failed") = config.language.clone();

        if image_bytes.is_empty() {
            return Err(KreuzbergError::validation("Empty image data".to_string()));
        }

        use std::borrow::Cow;
        Ok(ExtractionResult {
            content: format!("{} (lang: {})", self.return_text, config.language),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }

    fn supports_language(&self, lang: &str) -> bool {
        matches!(lang, "eng" | "deu" | "fra")
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["eng".to_string(), "deu".to_string(), "fra".to_string()]
    }
}

struct FailingOcrBackend {
    name: String,
}

impl Plugin for FailingOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for FailingOcrBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
        Err(KreuzbergError::ocr("OCR processing intentionally failed".to_string()))
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

struct ValidatingOcrBackend {
    name: String,
    min_size: usize,
}

impl Plugin for ValidatingOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for ValidatingOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
        if image_bytes.len() < self.min_size {
            return Err(KreuzbergError::validation(format!(
                "Image too small: {} < {} bytes",
                image_bytes.len(),
                self.min_size
            )));
        }

        use std::borrow::Cow;
        Ok(ExtractionResult {
            content: format!("Processed {} bytes", image_bytes.len()),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

struct MetadataOcrBackend {
    name: String,
}

impl Plugin for MetadataOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for MetadataOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let mut metadata = Metadata::default();
        metadata.additional.insert(
            std::borrow::Cow::Borrowed("ocr_backend"),
            serde_json::json!(self.name()),
        );
        metadata.additional.insert(
            std::borrow::Cow::Borrowed("image_size"),
            serde_json::json!(image_bytes.len()),
        );
        metadata.additional.insert(
            std::borrow::Cow::Borrowed("ocr_language"),
            serde_json::json!(config.language),
        );

        use std::borrow::Cow;
        Ok(ExtractionResult {
            content: "OCR processed text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            ..Default::default()
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

struct DocumentProcessingOcrBackend {
    name: String,
    image_call_count: AtomicUsize,
    document_call_count: AtomicUsize,
    supports_doc_override: bool,
}

impl Plugin for DocumentProcessingOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for DocumentProcessingOcrBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
        self.image_call_count.fetch_add(1, Ordering::SeqCst);
        
        use std::borrow::Cow;
        Ok(ExtractionResult {
            content: "Processed via image extraction".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }
    
    fn supports_document_processing(&self) -> bool {
        self.supports_doc_override
    }
    
    async fn process_document(&self, _document_path: &std::path::Path, _config: &OcrConfig) -> Result<ExtractionResult> {
        self.document_call_count.fetch_add(1, Ordering::SeqCst);
        
        use std::borrow::Cow;
        Ok(ExtractionResult {
            content: "Processed natively as document".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

#[serial]
#[test]
fn test_register_custom_ocr_backend() {
    let _guard = BackendRegistryGuard;
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "test-ocr".to_string(),
        return_text: "Mocked OCR Result".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        let result = reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>);
        assert!(result.is_ok(), "Failed to register OCR backend: {:?}", result.err());
    }

    assert!(
        backend.initialized.load(Ordering::Acquire),
        "OCR backend was not initialized"
    );

    let list = {
        let reg = registry.read().expect("Operation failed");
        reg.list()
    };

    assert!(list.contains(&"test-ocr".to_string()));

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_used_for_image_extraction() {
    let _guard = BackendRegistryGuard;
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "extraction-test-ocr".to_string(),
        return_text: "CUSTOM OCR TEXT".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "extraction-test-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let extraction_result = result.expect("Operation failed");
    assert!(
        extraction_result.content.contains("CUSTOM OCR TEXT"),
        "Custom OCR backend was not used. Content: {}",
        extraction_result.content
    );

    assert_eq!(
        backend.call_count.load(Ordering::SeqCst),
        1,
        "OCR backend was not called exactly once"
    );

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_receives_correct_parameters() {
    let _guard = BackendRegistryGuard;
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "param-test-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "param-test-ocr".to_string(),
        language: "deu".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_ok());

    let last_lang = backend.last_language.lock().expect("Operation failed");
    assert_eq!(*last_lang, "deu", "Language parameter not passed correctly");

    let extraction_result = result.expect("Operation failed");
    assert!(extraction_result.content.contains("(lang: deu)"));

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_returns_correct_format() {
    let _guard = BackendRegistryGuard;
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MetadataOcrBackend {
        name: "format-test-ocr".to_string(),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(backend as Arc<dyn OcrBackend>).expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "format-test-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.expect("Operation failed");

    assert!(!extraction_result.content.is_empty());
    assert_eq!(extraction_result.mime_type, "image/png");
    assert!(extraction_result.metadata.additional.contains_key("ocr_backend"));
    assert!(extraction_result.metadata.additional.contains_key("image_size"));
    assert!(extraction_result.metadata.additional.contains_key("ocr_language"));

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_error_handling() {
    let _guard = BackendRegistryGuard;
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(FailingOcrBackend {
        name: "failing-ocr".to_string(),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(backend as Arc<dyn OcrBackend>).expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "failing-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_err(), "Expected OCR to fail");

    match result.expect_err("Operation failed") {
        KreuzbergError::Ocr { message, .. } => {
            assert!(message.contains("intentionally failed"));
        }
        other => panic!("Expected Ocr error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_validation_error() {
    let _guard = BackendRegistryGuard;
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(ValidatingOcrBackend {
        name: "validating-ocr".to_string(),
        min_size: 1_000_000,
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(backend as Arc<dyn OcrBackend>).expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "validating-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_err(), "Expected validation to fail");

    match result.expect_err("Operation failed") {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("Image too small"));
        }
        other => panic!("Expected Validation error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_switching_between_ocr_backends() {
    let _guard = BackendRegistryGuard;
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend1 = Arc::new(MockOcrBackend {
        name: "backend-1".to_string(),
        return_text: "BACKEND ONE OUTPUT".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    let backend2 = Arc::new(MockOcrBackend {
        name: "backend-2".to_string(),
        return_text: "BACKEND TWO OUTPUT".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend1) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
        reg.register(Arc::clone(&backend2) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    let ocr_config1 = OcrConfig {
        backend: "backend-1".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config1 = ExtractionConfig {
        ocr: Some(ocr_config1),
        force_ocr: false,
        ..Default::default()
    };

    let result1 = extract_file_sync(test_image, None, &config1);
    assert!(result1.is_ok());
    assert!(
        result1
            .expect("Operation failed")
            .content
            .contains("BACKEND ONE OUTPUT")
    );
    assert_eq!(backend1.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(backend2.call_count.load(Ordering::SeqCst), 0);

    let ocr_config2 = OcrConfig {
        backend: "backend-2".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config2 = ExtractionConfig {
        ocr: Some(ocr_config2),
        force_ocr: false,
        ..Default::default()
    };

    let result2 = extract_file_sync(test_image, None, &config2);
    assert!(result2.is_ok());
    assert!(
        result2
            .expect("Operation failed")
            .content
            .contains("BACKEND TWO OUTPUT")
    );
    assert_eq!(backend1.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(backend2.call_count.load(Ordering::SeqCst), 1);

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_language_support() {
    let _guard = BackendRegistryGuard;
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "lang-test-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("deu"));
    assert!(backend.supports_language("fra"));
    assert!(!backend.supports_language("jpn"));

    let supported = backend.supported_languages();
    assert_eq!(supported.len(), 3);
    assert!(supported.contains(&"eng".to_string()));
    assert!(supported.contains(&"deu".to_string()));
    assert!(supported.contains(&"fra".to_string()));

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_type() {
    let _guard = BackendRegistryGuard;
    let backend = MockOcrBackend {
        name: "type-test".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    };

    assert_eq!(backend.backend_type(), OcrBackendType::Custom);
}

#[serial]
#[test]
fn test_ocr_backend_invalid_name() {
    let _guard = BackendRegistryGuard;
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "invalid name".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        let result = reg.register(backend);

        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("Operation failed"),
            KreuzbergError::Validation { .. }
        ));
    }

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_initialization_lifecycle() {
    let _guard = BackendRegistryGuard;
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "lifecycle-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    assert!(
        !backend.initialized.load(Ordering::Acquire),
        "Backend should not be initialized yet"
    );

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    assert!(
        backend.initialized.load(Ordering::Acquire),
        "Backend should be initialized after registration"
    );

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    assert!(
        !backend.initialized.load(Ordering::Acquire),
        "Backend should be shutdown"
    );
}

#[serial]
#[test]
fn test_unregister_ocr_backend() {
    let _guard = BackendRegistryGuard;
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    let backend = Arc::new(MockOcrBackend {
        name: "unregister-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.remove("unregister-ocr").expect("Operation failed");
    }

    let list = {
        let reg = registry.read().expect("Operation failed");
        reg.list()
    };

    assert!(!list.contains(&"unregister-ocr".to_string()));
    assert!(
        !backend.initialized.load(Ordering::Acquire),
        "Backend should be shutdown after unregistration"
    );
}

#[serial]
#[test]
fn test_ocr_backend_document_processing_fallback() {
    let _guard = BackendRegistryGuard;
    let test_document = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_documents/pdf/ocr_test.pdf");
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    // Backend that DOES NOT support document processing natively
    let backend = Arc::new(DocumentProcessingOcrBackend {
        name: "fallback-ocr".to_string(),
        image_call_count: AtomicUsize::new(0),
        document_call_count: AtomicUsize::new(0),
        supports_doc_override: false,
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "fallback-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    // Use async environment if required or standard sync method
    let result = extract_file_sync(test_document, None, &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let extraction_result = result.expect("Operation failed");
    assert!(
        extraction_result.content.contains("Processed via image extraction"),
        "Custom OCR fallback was not used. Content: {}",
        extraction_result.content
    );

    // It should have called process_image multiple times (one for each PDF page)
    assert!(
        backend.image_call_count.load(Ordering::SeqCst) > 0,
        "OCR fallback to image extraction was not called"
    );
    assert_eq!(
        backend.document_call_count.load(Ordering::SeqCst),
        0,
        "Native process_document was called unexpectedly"
    );

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_document_processing_override() {
    let _guard = BackendRegistryGuard;
    let test_document = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_documents/pdf/ocr_test.pdf");
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }

    // Backend that DOES support document processing
    let backend = Arc::new(DocumentProcessingOcrBackend {
        name: "override-ocr".to_string(),
        image_call_count: AtomicUsize::new(0),
        document_call_count: AtomicUsize::new(0),
        supports_doc_override: true,
    });

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "override-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_document, None, &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let extraction_result = result.expect("Operation failed");
    assert!(
        extraction_result.content.contains("Processed natively as document"),
        "Custom OCR document override was not used. Content: {}",
        extraction_result.content
    );

    // It should have exactly one call to process_document natively
    assert_eq!(
        backend.image_call_count.load(Ordering::SeqCst),
        0,
        "process_image was called unexpectedly"
    );
    assert_eq!(
        backend.document_call_count.load(Ordering::SeqCst),
        1,
        "process_document was not called exactly once"
    );

    {
        let mut reg = registry.write().expect("Operation failed");
        reg.shutdown_all().expect("Operation failed");
    }
}

#[serial]
#[test]
fn test_ocr_backend_document_processing_missing_path_fallback() {
    let _guard = BackendRegistryGuard;
    let test_document = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_documents/pdf/ocr_test.pdf");
    
    let bytes = std::fs::read(test_document).expect("Failed to read test document");
    
    let backend = std::sync::Arc::new(DocumentProcessingOcrBackend {
        name: "missing-path-ocr".to_string(),
        image_call_count: std::sync::atomic::AtomicUsize::new(0),
        document_call_count: std::sync::atomic::AtomicUsize::new(0),
        supports_doc_override: true,
    });

    {
        let registry = get_ocr_backend_registry();
        let mut reg = registry.write().expect("Operation failed");
        reg.register(std::sync::Arc::clone(&backend) as std::sync::Arc<dyn OcrBackend>)
            .expect("Operation failed");
    }

    let ocr_config = OcrConfig {
        backend: "missing-path-ocr".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = kreuzberg::extract_bytes_sync(&bytes, "application/pdf", &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let extraction_result = result.expect("Operation failed");
    assert!(
        extraction_result.content.contains("Processed via image extraction"),
        "Custom OCR fallback was not used. Content: {}",
        extraction_result.content
    );

    assert!(
        backend.image_call_count.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "OCR fallback to image extraction was not called"
    );
    assert_eq!(
        backend.document_call_count.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "Native process_document was called unexpectedly on memory bytes"
    );
}
