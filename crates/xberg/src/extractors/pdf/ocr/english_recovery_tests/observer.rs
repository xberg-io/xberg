use crate::core::config::OcrConfig;
use crate::plugins::{ConfidenceSemantics, OcrBackend, OcrBackendType, PageOrientationHandling, Plugin};
use crate::types::ExtractedDocument;
use std::sync::{Arc, Mutex};

pub(super) type Records = Arc<Mutex<Vec<serde_json::Value>>>;

struct Observer {
    backend: Arc<dyn OcrBackend>,
    records: Records,
}

impl Plugin for Observer {
    fn name(&self) -> &str {
        self.backend.name()
    }
    fn version(&self) -> String {
        self.backend.version()
    }
    fn initialize(&self) -> crate::Result<()> {
        self.backend.initialize()
    }
    fn shutdown(&self) -> crate::Result<()> {
        self.backend.shutdown()
    }
}

#[async_trait::async_trait]
impl OcrBackend for Observer {
    fn backend_type(&self) -> OcrBackendType {
        self.backend.backend_type()
    }
    fn supports_language(&self, language: &str) -> bool {
        self.backend.supports_language(language)
    }
    fn supports_language_for(&self, config: &OcrConfig, language: &str) -> bool {
        self.backend.supports_language_for(config, language)
    }
    fn supported_languages(&self) -> Vec<String> {
        self.backend.supported_languages()
    }
    fn supported_languages_for(&self, config: &OcrConfig) -> Vec<String> {
        self.backend.supported_languages_for(config)
    }
    fn supports_table_detection(&self) -> bool {
        self.backend.supports_table_detection()
    }
    fn probe(&self, config: &OcrConfig) -> crate::doctor::DoctorCheck {
        self.backend.probe(config)
    }
    fn confidence_semantics(&self) -> ConfidenceSemantics {
        self.backend.confidence_semantics()
    }
    fn page_orientation_handling(&self) -> PageOrientationHandling {
        self.backend.page_orientation_handling()
    }
    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.process_image_owned(Arc::new(bytes.to_vec()), config).await
    }
    async fn process_image_file(&self, path: &std::path::Path, config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.backend.process_image_file(path, config).await
    }
    async fn process_image_owned(&self, bytes: Arc<Vec<u8>>, config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let input_sha = super::fixture::sha(&bytes);
        let input_len = bytes.len();
        let result = self.backend.process_image_owned(bytes, config).await;
        self.records.lock().unwrap().push(serde_json::json!({
            "input_sha256":input_sha,"input_bytes":input_len,"config":config,
            "runtime_max_content_size":config.security_limits.as_ref().map(|limits| limits.max_content_size),
            "actual_text":result.as_ref().ok().map(|document| &document.content),
            "error":result.as_ref().err().map(ToString::to_string)
        }));
        result
    }
}

pub(super) fn install() -> Records {
    crate::plugins::ensure_ocr_backends_initialized();
    let backend = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .get("tesseract")
        .expect("real compiled-in Tesseract, not a mock");
    assert!(matches!(backend.backend_type(), OcrBackendType::Tesseract));
    assert_eq!(backend.version(), "5.5.4");
    crate::plugins::unregister_ocr_backend("tesseract").unwrap();
    let records = Arc::new(Mutex::new(Vec::new()));
    crate::plugins::register_ocr_backend(Arc::new(Observer {
        backend,
        records: records.clone(),
    }))
    .unwrap();
    records
}

pub(super) fn assert_policy(records: &Records, cap: usize, directory: &std::path::Path) {
    let calls = records.lock().unwrap();
    assert!(!calls.is_empty(), "no cache-only or native-text-only pass");
    assert!(
        calls
            .iter()
            .any(|call| { call["actual_text"].as_str().is_some_and(super::fixture::has_words) }),
        "real delegate must supply the observed English words"
    );
    for call in calls.iter() {
        assert!(call["input_bytes"].as_u64().unwrap() > 0);
        assert_eq!(call["input_sha256"].as_str().unwrap().len(), 64);
        let config = &call["config"];
        assert_eq!(config["language"], serde_json::json!(["eng"]));
        assert_eq!(config["tessdata_path"], serde_json::json!(directory));
        assert_eq!(call["runtime_max_content_size"], cap);
        assert_eq!(config["backend_options"]["use_cache"], false);
        assert!(config["backend_options"]["source_dpi"].as_f64().unwrap() > 0.0);
        let preprocessing = &config["tesseract_config"]["preprocessing"];
        assert_eq!(preprocessing["target_dpi"], 300);
        assert_eq!(preprocessing["binarization_method"], "none");
        assert_eq!(preprocessing["contrast_enhance"], true);
        for field in [
            "auto_rotate",
            "deskew",
            "denoise",
            "invert_colors",
            "normalize_shaded_rows",
        ] {
            assert_eq!(preprocessing[field], false);
        }
    }
}
