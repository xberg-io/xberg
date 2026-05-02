//! Comprehensive plugin system integration tests.
//!
//! Tests plugin registration, discovery, error handling, concurrent access,
//! and cross-registry interactions for all 4 plugin types.

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::registry::{
    DocumentExtractorRegistry, OcrBackendRegistry, PostProcessorRegistry, ValidatorRegistry,
};
use kreuzberg::plugins::{DocumentExtractor, Plugin, PostProcessor, ProcessingStage, Validator};
use kreuzberg::types::ExtractionResult;
use kreuzberg::types::internal::{ElementKind, InternalDocument, InternalElement};
use kreuzberg::{KreuzbergError, Result};
use std::borrow::Cow;
use std::sync::Arc;

struct FailingExtractor {
    name: String,
    should_fail_init: bool,
    should_fail_extract: bool,
}

impl Plugin for FailingExtractor {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> Result<()> {
        if self.should_fail_init {
            Err(KreuzbergError::Plugin {
                message: "Initialization failed".to_string(),
                plugin_name: self.name.clone(),
            })
        } else {
            Ok(())
        }
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for FailingExtractor {
    async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<InternalDocument> {
        if self.should_fail_extract {
            Err(KreuzbergError::Parsing {
                message: "Extraction failed".to_string(),
                source: None,
            })
        } else {
            let mut doc = InternalDocument::new("text");
            doc.mime_type = Cow::Borrowed("text/plain");
            doc.push_element(InternalElement::text(ElementKind::Paragraph, "success", 0));
            Ok(doc)
        }
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/plain"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

struct MetadataModifyingProcessor {
    name: String,
    stage: ProcessingStage,
    priority: i32,
}

impl MetadataModifyingProcessor {
    fn new(name: impl Into<String>, stage: ProcessingStage) -> Self {
        Self {
            name: name.into(),
            stage,
            priority: 50,
        }
    }

    fn with_priority(name: impl Into<String>, stage: ProcessingStage, priority: i32) -> Self {
        Self {
            name: name.into(),
            stage,
            priority,
        }
    }
}

impl Plugin for MetadataModifyingProcessor {
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
impl PostProcessor for MetadataModifyingProcessor {
    async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
        result.content.push_str(&format!(" [{}]", self.name));
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

struct FailingProcessor {
    name: String,
}

impl Plugin for FailingProcessor {
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
impl PostProcessor for FailingProcessor {
    async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
        Err(KreuzbergError::Plugin {
            message: "Processing failed".to_string(),
            plugin_name: self.name.clone(),
        })
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }
}

struct StrictValidator {
    name: String,
    min_length: usize,
}

impl Plugin for StrictValidator {
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
impl Validator for StrictValidator {
    async fn validate(&self, result: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
        if result.content.len() < self.min_length {
            Err(KreuzbergError::validation(format!(
                "Content too short: {} < {}",
                result.content.len(),
                self.min_length
            )))
        } else {
            Ok(())
        }
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[test]
fn test_extractor_registration_failure() {
    let mut registry = DocumentExtractorRegistry::new();

    let failing_extractor = Arc::new(FailingExtractor {
        name: "failing-extractor".to_string(),
        should_fail_init: true,
        should_fail_extract: false,
    });

    let result = registry.register(failing_extractor);
    assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
}

#[tokio::test]
async fn test_extractor_extraction_failure() {
    let mut registry = DocumentExtractorRegistry::new();

    let failing_extractor = Arc::new(FailingExtractor {
        name: "failing-extractor".to_string(),
        should_fail_init: false,
        should_fail_extract: true,
    });

    registry.register(failing_extractor).expect("Operation failed");

    let extractor = registry.get("text/plain").expect("Value not found");
    let config = ExtractionConfig::default();
    let result = extractor.extract_bytes(b"test", "text/plain", &config).await;

    assert!(matches!(result, Err(KreuzbergError::Parsing { .. })));
}

#[test]
fn test_extractor_duplicate_registration() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor1 = Arc::new(FailingExtractor {
        name: "same-name".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    });

    let extractor2 = Arc::new(FailingExtractor {
        name: "same-name".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    });

    registry.register(extractor1).expect("Operation failed");
    registry.register(extractor2).expect("Operation failed");

    let names = registry.list();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"same-name".to_string()));
}

#[test]
fn test_extractor_concurrent_registration() {
    use std::sync::{Arc as StdArc, RwLock};
    use std::thread;

    let registry = StdArc::new(RwLock::new(DocumentExtractorRegistry::new()));
    let mut handles = vec![];

    for i in 0..10 {
        let registry_clone = StdArc::clone(&registry);
        let handle = thread::spawn(move || {
            let extractor = Arc::new(FailingExtractor {
                name: format!("extractor-{}", i),
                should_fail_init: false,
                should_fail_extract: false,
            });

            let mut reg = registry_clone
                .write()
                .expect("Failed to acquire write lock on registry in test");
            reg.register(extractor).expect("Operation failed");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Operation failed");
    }

    let reg = registry
        .read()
        .expect("Failed to acquire read lock on registry in test");
    assert_eq!(reg.list().len(), 10);
}

#[test]
fn test_extractor_priority_ordering_complex() {
    let mut registry = DocumentExtractorRegistry::new();

    struct PriorityExtractor {
        name: String,
        priority: i32,
    }

    impl Plugin for PriorityExtractor {
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
    impl DocumentExtractor for PriorityExtractor {
        async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<InternalDocument> {
            let mut doc = InternalDocument::new("text");
            doc.mime_type = Cow::Borrowed("text/plain");
            doc.push_element(InternalElement::text(ElementKind::Paragraph, "test", 0));
            Ok(doc)
        }
        fn supported_mime_types(&self) -> &[&str] {
            &["text/plain"]
        }
        fn priority(&self) -> i32 {
            self.priority
        }
    }

    for priority in [10, 50, 100, 25, 75] {
        let extractor = Arc::new(PriorityExtractor {
            name: format!("priority-{}", priority),
            priority,
        });
        registry.register(extractor).expect("Operation failed");
    }

    let selected = registry.get("text/plain").expect("Value not found");
    assert_eq!(selected.name(), "priority-100");
    assert_eq!(selected.priority(), 100);
}

#[test]
fn test_extractor_wildcard_vs_exact_priority() {
    let mut registry = DocumentExtractorRegistry::new();

    let _wildcard = Arc::new(FailingExtractor {
        name: "wildcard-high".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    });

    struct WildcardExtractor(FailingExtractor);
    impl Plugin for WildcardExtractor {
        fn name(&self) -> &str {
            self.0.name()
        }
        fn version(&self) -> String {
            self.0.version()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl DocumentExtractor for WildcardExtractor {
        async fn extract_bytes(&self, c: &[u8], m: &str, cfg: &ExtractionConfig) -> Result<InternalDocument> {
            self.0.extract_bytes(c, m, cfg).await
        }
        fn supported_mime_types(&self) -> &[&str] {
            &["text/*"]
        }
        fn priority(&self) -> i32 {
            100
        }
    }

    let wildcard_arc = Arc::new(WildcardExtractor(FailingExtractor {
        name: "wildcard-high".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    }));

    let exact = Arc::new(FailingExtractor {
        name: "exact-low".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    });

    registry.register(wildcard_arc).expect("Operation failed");
    registry.register(exact).expect("Operation failed");

    let selected = registry.get("text/plain").expect("Value not found");
    assert_eq!(selected.name(), "exact-low");
}

#[test]
fn test_extractor_empty_mime_type() {
    let registry = DocumentExtractorRegistry::new();
    let result = registry.get("");
    assert!(matches!(result, Err(KreuzbergError::UnsupportedFormat(_))));
}

#[test]
fn test_extractor_special_characters_mime() {
    let registry = DocumentExtractorRegistry::new();
    let result = registry.get("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
    assert!(matches!(result, Err(KreuzbergError::UnsupportedFormat(_))));
}

#[test]
fn test_extractor_remove_nonexistent() {
    let mut registry = DocumentExtractorRegistry::new();
    let result = registry.remove("nonexistent");
    assert!(result.is_ok());
}

#[test]
fn test_extractor_list_after_partial_removal() {
    let mut registry = DocumentExtractorRegistry::new();

    for i in 0..5 {
        let extractor = Arc::new(FailingExtractor {
            name: format!("extractor-{}", i),
            should_fail_init: false,
            should_fail_extract: false,
        });
        registry.register(extractor).expect("Operation failed");
    }

    registry.remove("extractor-2").expect("Operation failed");
    registry.remove("extractor-3").expect("Operation failed");

    let names = registry.list();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"extractor-0".to_string()));
    assert!(names.contains(&"extractor-1".to_string()));
    assert!(names.contains(&"extractor-4".to_string()));
}

#[tokio::test]
async fn test_processor_execution_order_within_stage() {
    let mut registry = PostProcessorRegistry::new();

    let high = Arc::new(MetadataModifyingProcessor::with_priority(
        "high",
        ProcessingStage::Early,
        100,
    ));

    let medium = Arc::new(MetadataModifyingProcessor::with_priority(
        "medium",
        ProcessingStage::Early,
        50,
    ));

    let low = Arc::new(MetadataModifyingProcessor::with_priority(
        "low",
        ProcessingStage::Early,
        10,
    ));

    registry.register(low).expect("Operation failed");
    registry.register(high).expect("Operation failed");
    registry.register(medium).expect("Operation failed");

    let processors = registry.get_for_stage(ProcessingStage::Early);
    assert_eq!(processors.len(), 3);

    let mut result = ExtractionResult {
        content: "start".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };

    let config = ExtractionConfig::default();
    for processor in processors {
        processor
            .process(&mut result, &config)
            .await
            .expect("Async operation failed");
    }

    assert_eq!(result.content, "start [high] [medium] [low]");
}

#[tokio::test]
async fn test_processor_error_propagation() {
    let mut registry = PostProcessorRegistry::new();

    let failing = Arc::new(FailingProcessor {
        name: "failing".to_string(),
    });

    registry.register(failing).expect("Operation failed");

    let processors = registry.get_for_stage(ProcessingStage::Early);
    assert_eq!(processors.len(), 1);

    let mut result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };

    let config = ExtractionConfig::default();
    let process_result = processors[0].process(&mut result, &config).await;

    assert!(matches!(process_result, Err(KreuzbergError::Plugin { .. })));
}

#[test]
fn test_processor_multiple_stages() {
    let mut registry = PostProcessorRegistry::new();

    let early = Arc::new(MetadataModifyingProcessor::new("early", ProcessingStage::Early));
    let middle = Arc::new(MetadataModifyingProcessor::new("middle", ProcessingStage::Middle));
    let late = Arc::new(MetadataModifyingProcessor::new("late", ProcessingStage::Late));

    registry.register(early).expect("Operation failed");
    registry.register(middle).expect("Operation failed");
    registry.register(late).expect("Operation failed");

    assert_eq!(registry.get_for_stage(ProcessingStage::Early).len(), 1);
    assert_eq!(registry.get_for_stage(ProcessingStage::Middle).len(), 1);
    assert_eq!(registry.get_for_stage(ProcessingStage::Late).len(), 1);
}

#[test]
fn test_processor_registration_failure() {
    struct FailingInitProcessor;

    impl Plugin for FailingInitProcessor {
        fn name(&self) -> &str {
            "failing-init"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Err(KreuzbergError::Plugin {
                message: "Init failed".to_string(),
                plugin_name: "failing-init".to_string(),
            })
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for FailingInitProcessor {
        async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }
        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    let mut registry = PostProcessorRegistry::new();
    let processor = Arc::new(FailingInitProcessor);

    let result = registry.register(processor);
    assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
}

#[test]
fn test_processor_same_priority_same_stage() {
    let mut registry = PostProcessorRegistry::new();

    let proc1 = Arc::new(MetadataModifyingProcessor::new("processor1", ProcessingStage::Early));
    let proc2 = Arc::new(MetadataModifyingProcessor::new("processor2", ProcessingStage::Early));

    registry.register(proc1).expect("Operation failed");
    registry.register(proc2).expect("Operation failed");

    let processors = registry.get_for_stage(ProcessingStage::Early);
    assert_eq!(processors.len(), 2);
}

#[test]
fn test_processor_remove_from_specific_stage() {
    let mut registry = PostProcessorRegistry::new();

    let early = Arc::new(MetadataModifyingProcessor::new("processor", ProcessingStage::Early));

    registry.register(early).expect("Operation failed");
    assert_eq!(registry.get_for_stage(ProcessingStage::Early).len(), 1);

    registry.remove("processor").expect("Operation failed");
    assert_eq!(registry.get_for_stage(ProcessingStage::Early).len(), 0);
}

#[test]
fn test_processor_list_across_stages() {
    let mut registry = PostProcessorRegistry::new();

    for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
        let processor = Arc::new(MetadataModifyingProcessor::new(format!("{:?}-processor", stage), stage));
        registry.register(processor).expect("Operation failed");
    }

    let names = registry.list();
    assert_eq!(names.len(), 3);
}

#[test]
fn test_processor_shutdown_clears_all_stages() {
    let mut registry = PostProcessorRegistry::new();

    for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
        let processor = Arc::new(MetadataModifyingProcessor::new(format!("{:?}-processor", stage), stage));
        registry.register(processor).expect("Operation failed");
    }

    registry.shutdown_all().expect("Operation failed");

    assert_eq!(registry.get_for_stage(ProcessingStage::Early).len(), 0);
    assert_eq!(registry.get_for_stage(ProcessingStage::Middle).len(), 0);
    assert_eq!(registry.get_for_stage(ProcessingStage::Late).len(), 0);
}

#[tokio::test]
async fn test_validator_content_validation() {
    let mut registry = ValidatorRegistry::new();

    let strict = Arc::new(StrictValidator {
        name: "strict".to_string(),
        min_length: 10,
    });

    registry.register(strict).expect("Operation failed");

    let validators = registry.get_all();
    assert_eq!(validators.len(), 1);

    let config = ExtractionConfig::default();

    let short_result = ExtractionResult {
        content: "short".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };

    let validation = validators[0].validate(&short_result, &config).await;
    assert!(matches!(validation, Err(KreuzbergError::Validation { .. })));

    let long_result = ExtractionResult {
        content: "this is long enough content".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };

    let validation = validators[0].validate(&long_result, &config).await;
    assert!(validation.is_ok());
}

#[test]
fn test_validator_priority_ordering() {
    let mut registry = ValidatorRegistry::new();

    let _high = Arc::new(StrictValidator {
        name: "high-priority".to_string(),
        min_length: 5,
    });

    struct MediumPriorityValidator;
    impl Plugin for MediumPriorityValidator {
        fn name(&self) -> &str {
            "medium-priority"
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
    impl Validator for MediumPriorityValidator {
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }
        fn priority(&self) -> i32 {
            50
        }
    }

    struct LowPriorityValidator;
    impl Plugin for LowPriorityValidator {
        fn name(&self) -> &str {
            "low-priority"
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
    impl Validator for LowPriorityValidator {
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }
        fn priority(&self) -> i32 {
            10
        }
    }

    struct HighPriorityValidator;
    impl Plugin for HighPriorityValidator {
        fn name(&self) -> &str {
            "high-priority"
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
    impl Validator for HighPriorityValidator {
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }
        fn priority(&self) -> i32 {
            100
        }
    }

    let medium = Arc::new(MediumPriorityValidator);
    let low = Arc::new(LowPriorityValidator);
    let high_priority = Arc::new(HighPriorityValidator);

    registry.register(medium).expect("Operation failed");
    registry.register(low).expect("Operation failed");
    registry.register(high_priority).expect("Operation failed");

    let validators = registry.get_all();
    assert_eq!(validators.len(), 3);
    assert_eq!(validators[0].name(), "high-priority");
    assert_eq!(validators[1].name(), "medium-priority");
    assert_eq!(validators[2].name(), "low-priority");
}

#[test]
fn test_validator_registration_failure() {
    struct FailingInitValidator;

    impl Plugin for FailingInitValidator {
        fn name(&self) -> &str {
            "failing"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Err(KreuzbergError::Plugin {
                message: "Init failed".to_string(),
                plugin_name: "failing".to_string(),
            })
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for FailingInitValidator {
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }
        fn priority(&self) -> i32 {
            50
        }
    }

    let mut registry = ValidatorRegistry::new();
    let validator = Arc::new(FailingInitValidator);

    let result = registry.register(validator);
    assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
}

#[test]
fn test_validator_empty_registry() {
    let registry = ValidatorRegistry::new();
    let validators = registry.get_all();
    assert_eq!(validators.len(), 0);
}

#[test]
fn test_validator_remove_and_reregister() {
    let mut registry = ValidatorRegistry::new();

    let validator: Arc<dyn Validator> = Arc::new(StrictValidator {
        name: "validator".to_string(),
        min_length: 5,
    });

    registry.register(Arc::clone(&validator)).expect("Operation failed");
    assert_eq!(registry.get_all().len(), 1);

    registry.remove("validator").expect("Operation failed");
    assert_eq!(registry.get_all().len(), 0);

    registry.register(validator).expect("Operation failed");
    assert_eq!(registry.get_all().len(), 1);
}

#[test]
fn test_multiple_registries_independence() {
    let ocr_registry = OcrBackendRegistry::new_empty();
    let mut extractor_registry = DocumentExtractorRegistry::new();
    let mut processor_registry = PostProcessorRegistry::new();
    let mut validator_registry = ValidatorRegistry::new();

    let extractor = Arc::new(FailingExtractor {
        name: "test-extractor".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    });

    let processor = Arc::new(MetadataModifyingProcessor::new(
        "test-processor",
        ProcessingStage::Early,
    ));

    let validator = Arc::new(StrictValidator {
        name: "test-validator".to_string(),
        min_length: 5,
    });

    extractor_registry.register(extractor).expect("Operation failed");
    processor_registry.register(processor).expect("Operation failed");
    validator_registry.register(validator).expect("Operation failed");

    assert_eq!(ocr_registry.list().len(), 0);
    assert_eq!(extractor_registry.list().len(), 1);
    assert_eq!(processor_registry.list().len(), 1);
    assert_eq!(validator_registry.get_all().len(), 1);
}

#[test]
fn test_shutdown_all_registries() {
    let mut ocr_registry = OcrBackendRegistry::new_empty();
    let mut extractor_registry = DocumentExtractorRegistry::new();
    let mut processor_registry = PostProcessorRegistry::new();
    let mut validator_registry = ValidatorRegistry::new();

    let extractor = Arc::new(FailingExtractor {
        name: "test-extractor".to_string(),
        should_fail_init: false,
        should_fail_extract: false,
    });

    let processor = Arc::new(MetadataModifyingProcessor::new(
        "test-processor",
        ProcessingStage::Early,
    ));

    let validator = Arc::new(StrictValidator {
        name: "test-validator".to_string(),
        min_length: 5,
    });

    extractor_registry.register(extractor).expect("Operation failed");
    processor_registry.register(processor).expect("Operation failed");
    validator_registry.register(validator).expect("Operation failed");

    ocr_registry.shutdown_all().expect("Operation failed");
    extractor_registry.shutdown_all().expect("Operation failed");
    processor_registry.shutdown_all().expect("Operation failed");
    validator_registry.shutdown_all().expect("Operation failed");

    assert_eq!(ocr_registry.list().len(), 0);
    assert_eq!(extractor_registry.list().len(), 0);
    assert_eq!(processor_registry.list().len(), 0);
    assert_eq!(validator_registry.get_all().len(), 0);
}
