mod helpers;

use async_trait::async_trait;
use helpers::extract_uri_document_blocking;
use serial_test::serial;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use xberg::Result;
use xberg::core::config::{ExtractInput, ExtractionConfig};
use xberg::core::pipeline::clear_processor_cache;
use xberg::plugins::registry::{get_document_extractor_registry, get_post_processor_registry, get_validator_registry};
use xberg::plugins::{DocumentExtractor, Plugin, PostProcessor, ProcessingStage, Validator};
use xberg::types::ExtractedDocument;

const EXTRACTOR_NAME: &str = "issue-2032-counting-extractor";
const INITIAL_PROCESSOR_NAME: &str = "issue-2032-initial-processor";
const INITIAL_VALIDATOR_NAME: &str = "issue-2032-initial-validator";
const PROCESSOR_NAME: &str = "issue-2032-appending-processor";
const VALIDATOR_NAME: &str = "issue-2032-tracking-validator";
const INITIAL_MARKER: &str = " [INITIAL PROCESSOR RAN]";
const MARKER: &str = " [LATE PROCESSOR RAN]";
const REPLACEMENT_MARKER: &str = " [REPLACEMENT PROCESSOR RAN]";

struct CountingExtractor {
    calls: Arc<AtomicUsize>,
}

impl Plugin for CountingExtractor {
    fn name(&self) -> &str {
        EXTRACTOR_NAME
    }
}

#[async_trait]
impl DocumentExtractor for CountingExtractor {
    async fn extract(&self, input: ExtractInput, _config: &ExtractionConfig) -> Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut document = ExtractedDocument::default();
        document.content = "cached content".to_string();
        document.mime_type = input.mime_type.map(Cow::Owned).unwrap_or(Cow::Borrowed("text/plain"));
        Ok(document)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/plain"]
    }

    fn priority(&self) -> i32 {
        100
    }
}

struct AppendingProcessor {
    name: &'static str,
    marker: &'static str,
    calls: Arc<AtomicUsize>,
}

struct RegisteringProcessor {
    calls: Arc<AtomicUsize>,
    late_calls: Arc<AtomicUsize>,
    registered: AtomicBool,
}

impl Plugin for RegisteringProcessor {
    fn name(&self) -> &str {
        INITIAL_PROCESSOR_NAME
    }
}

#[async_trait]
impl PostProcessor for RegisteringProcessor {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        result.content.push_str(INITIAL_MARKER);
        if !self.registered.swap(true, Ordering::SeqCst) {
            get_post_processor_registry()
                .write()
                .register(Arc::new(AppendingProcessor {
                    name: PROCESSOR_NAME,
                    marker: MARKER,
                    calls: Arc::clone(&self.late_calls),
                }))?;
        }
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }
}

impl Plugin for AppendingProcessor {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait]
impl PostProcessor for AppendingProcessor {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        result.content.push_str(self.marker);
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }
}

struct TrackingValidator {
    name: &'static str,
    required_suffix: &'static str,
    calls: Arc<AtomicUsize>,
}

impl Plugin for TrackingValidator {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait]
impl Validator for TrackingValidator {
    async fn validate(&self, result: &ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        assert!(result.content.contains(self.required_suffix));
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

static FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

struct LifecycleHarness {
    _directory: tempfile::TempDir,
    path: std::path::PathBuf,
    config: ExtractionConfig,
    extractor_calls: Arc<AtomicUsize>,
    initial_processor_calls: Arc<AtomicUsize>,
    initial_validator_calls: Arc<AtomicUsize>,
    late_processor_calls: Arc<AtomicUsize>,
    late_validator_calls: Arc<AtomicUsize>,
    replacement_processor_calls: Arc<AtomicUsize>,
    replacement_validator_calls: Arc<AtomicUsize>,
}

impl LifecycleHarness {
    fn new() -> Self {
        let harness = Self::with_fixture();
        harness.register_extractor();
        harness.register_initial(INITIAL_MARKER);
        harness
    }

    fn with_registration_during_processing() -> Self {
        let harness = Self::with_fixture();
        harness.register_extractor();
        get_post_processor_registry()
            .write()
            .register(Arc::new(RegisteringProcessor {
                calls: Arc::clone(&harness.initial_processor_calls),
                late_calls: Arc::clone(&harness.late_processor_calls),
                registered: AtomicBool::new(false),
            }))
            .unwrap();
        harness
    }

    fn register_extractor(&self) {
        get_document_extractor_registry()
            .write()
            .register(Arc::new(CountingExtractor {
                calls: Arc::clone(&self.extractor_calls),
            }))
            .unwrap();
    }

    fn with_fixture() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cached.txt");
        let id = FIXTURE_ID.fetch_add(1, Ordering::SeqCst);
        std::fs::write(&path, format!("issue-2032-{}-{id}", std::process::id())).unwrap();
        Self {
            _directory: directory,
            path,
            config: ExtractionConfig::default(),
            extractor_calls: Arc::new(AtomicUsize::new(0)),
            initial_processor_calls: Arc::new(AtomicUsize::new(0)),
            initial_validator_calls: Arc::new(AtomicUsize::new(0)),
            late_processor_calls: Arc::new(AtomicUsize::new(0)),
            late_validator_calls: Arc::new(AtomicUsize::new(0)),
            replacement_processor_calls: Arc::new(AtomicUsize::new(0)),
            replacement_validator_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn register_initial(&self, marker: &'static str) {
        let processor_calls = if marker == INITIAL_MARKER {
            &self.initial_processor_calls
        } else {
            &self.replacement_processor_calls
        };
        let validator_calls = if marker == INITIAL_MARKER {
            &self.initial_validator_calls
        } else {
            &self.replacement_validator_calls
        };
        self.register_hooks(
            INITIAL_PROCESSOR_NAME,
            INITIAL_VALIDATOR_NAME,
            marker,
            processor_calls,
            validator_calls,
        );
    }

    fn register_late(&self) {
        self.register_hooks(
            PROCESSOR_NAME,
            VALIDATOR_NAME,
            MARKER,
            &self.late_processor_calls,
            &self.late_validator_calls,
        );
    }

    fn register_hooks(
        &self,
        processor_name: &'static str,
        validator_name: &'static str,
        marker: &'static str,
        processor_calls: &Arc<AtomicUsize>,
        validator_calls: &Arc<AtomicUsize>,
    ) {
        get_post_processor_registry()
            .write()
            .register(Arc::new(AppendingProcessor {
                name: processor_name,
                marker,
                calls: Arc::clone(processor_calls),
            }))
            .unwrap();
        get_validator_registry()
            .write()
            .register(Arc::new(TrackingValidator {
                name: validator_name,
                required_suffix: marker,
                calls: Arc::clone(validator_calls),
            }))
            .unwrap();
    }

    fn extract(&self) -> ExtractedDocument {
        extract_uri_document_blocking(&self.path, Some("text/plain"), &self.config).unwrap()
    }

    fn remove_hooks(&self, processor_name: &str, validator_name: &str) {
        get_post_processor_registry().write().remove(processor_name).unwrap();
        get_validator_registry().write().remove(validator_name).unwrap();
    }
}

impl Drop for LifecycleHarness {
    fn drop(&mut self) {
        let _ = get_document_extractor_registry().write().remove(EXTRACTOR_NAME);
        let _ = get_post_processor_registry().write().remove(PROCESSOR_NAME);
        let _ = get_post_processor_registry().write().remove(INITIAL_PROCESSOR_NAME);
        let _ = get_validator_registry().write().remove(VALIDATOR_NAME);
        let _ = get_validator_registry().write().remove(INITIAL_VALIDATOR_NAME);
        let _ = clear_processor_cache();
    }
}

#[test]
#[serial]
fn stable_lifecycle_registries_produce_a_cache_hit_without_rerunning_hooks() {
    let harness = LifecycleHarness::new();
    assert_eq!(harness.extract().content, format!("cached content{INITIAL_MARKER}"));
    assert_eq!(harness.extract().content, format!("cached content{INITIAL_MARKER}"));
    assert_eq!(harness.extractor_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.initial_processor_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.initial_validator_calls.load(Ordering::SeqCst), 1);
}

#[test]
#[serial]
fn adding_and_removing_hooks_invalidates_and_refreshes_the_cache() {
    let harness = LifecycleHarness::new();
    harness.extract();
    harness.register_late();

    assert_eq!(
        harness.extract().content,
        format!("cached content{INITIAL_MARKER}{MARKER}")
    );
    harness.extract();
    assert_eq!(harness.extractor_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.late_processor_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.late_validator_calls.load(Ordering::SeqCst), 1);

    harness.remove_hooks(PROCESSOR_NAME, VALIDATOR_NAME);
    assert_eq!(harness.extract().content, format!("cached content{INITIAL_MARKER}"));
    harness.extract();
    assert_eq!(harness.extractor_calls.load(Ordering::SeqCst), 3);
}

#[test]
#[serial]
fn same_name_hook_replacements_invalidate_and_refresh_the_cache() {
    let harness = LifecycleHarness::new();
    harness.extract();
    harness.remove_hooks(INITIAL_PROCESSOR_NAME, INITIAL_VALIDATOR_NAME);
    harness.register_initial(REPLACEMENT_MARKER);

    assert_eq!(harness.extract().content, format!("cached content{REPLACEMENT_MARKER}"));
    harness.extract();
    assert_eq!(harness.extractor_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.replacement_processor_calls.load(Ordering::SeqCst), 1);
    assert_eq!(harness.replacement_validator_calls.load(Ordering::SeqCst), 1);
}

#[test]
#[serial]
fn registration_during_processing_cannot_stamp_an_incomplete_cache_entry() {
    let harness = LifecycleHarness::with_registration_during_processing();

    assert_eq!(harness.extract().content, format!("cached content{INITIAL_MARKER}"));
    assert_eq!(harness.late_processor_calls.load(Ordering::SeqCst), 0);

    assert_eq!(
        harness.extract().content,
        format!("cached content{INITIAL_MARKER}{MARKER}")
    );
    assert_eq!(harness.extractor_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.initial_processor_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.late_processor_calls.load(Ordering::SeqCst), 1);

    harness.extract();
    assert_eq!(harness.extractor_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.initial_processor_calls.load(Ordering::SeqCst), 2);
    assert_eq!(harness.late_processor_calls.load(Ordering::SeqCst), 1);
}
