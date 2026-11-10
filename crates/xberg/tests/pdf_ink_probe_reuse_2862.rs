#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
};
use xberg::plugins::{ConfidenceSemantics, OcrBackend, OcrBackendType, Plugin};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat};

const ARABIC: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic.pdf");
const THEN_SCANNED: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic-then-scanned-english.pdf");
const TWO: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic-and-bengali.pdf");
const PROSE: &str = "The committee met on Tuesday to review the plan for the new library. Members asked about the schedule and the budget for the parking area. The architect said that building work would start after the winter. A quiet queue of residents waited to speak about the design of the reading rooms and the garden. ";
const BACKEND: &str = "tesseract";

type ObservedInput = (String, String, (u32, u32), serde_json::Value);

struct Counted {
    calls: AtomicUsize,
    inputs: Mutex<Vec<ObservedInput>>,
    failures: usize,
    rejected_psm: AtomicI32,
    low_quality: AtomicBool,
    policy_sensitive: AtomicBool,
    include_table: AtomicBool,
    model_changes: AtomicBool,
    physical_model_changes: AtomicBool,
    models: tempfile::TempDir,
    text: String,
    cancel: Option<xberg::cancellation::CancellationToken>,
}

impl Plugin for Counted {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        if self.model_changes.load(Ordering::SeqCst) && self.calls.load(Ordering::SeqCst) > 0 {
            "2.0.0".into()
        } else {
            "1.0.0".into()
        }
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for Counted {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn confidence_semantics(&self) -> ConfidenceSemantics {
        ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }
    async fn process_image(&self, image: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let pixels = image::load_from_memory(image).unwrap().to_rgb8();
        self.inputs.lock().unwrap().push((
            blake3::hash(image).to_hex().to_string(),
            blake3::hash(pixels.as_raw()).to_hex().to_string(),
            pixels.dimensions(),
            serde_json::to_value(config).unwrap(),
        ));
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        let rejected_psm = config.tesseract_config.as_ref().and_then(|config| config.psm)
            == Some(self.rejected_psm.load(Ordering::SeqCst));
        if rejected_psm && !self.low_quality.load(Ordering::SeqCst) {
            return Err(xberg::XbergError::ocr("selected stage refused"));
        }
        if call < self.failures {
            return Err(xberg::XbergError::ocr("counted backend failure"));
        }
        if self.physical_model_changes.load(Ordering::SeqCst) && call == 0 {
            std::fs::write(self.models.path().join("eng.traineddata"), b"fake model b").unwrap();
        }
        let mut document = ExtractedDocument::default();
        document.content = if (self.policy_sensitive.load(Ordering::SeqCst)
            && !config
                .quality_thresholds
                .as_ref()
                .is_some_and(|thresholds| thresholds.discard_suspected_ocr_noise))
            || ((self.model_changes.load(Ordering::SeqCst) || self.physical_model_changes.load(Ordering::SeqCst))
                && call > 0)
        {
            self.text.replace("Tuesday", "Thursday")
        } else {
            self.text.clone()
        };
        if self.include_table.load(Ordering::SeqCst) {
            let day = if document.content.contains("Thursday") {
                "Thursday"
            } else {
                "Tuesday"
            };
            document.tables.push(xberg::types::Table {
                cells: vec![
                    vec!["Day".into(), "Meeting".into()],
                    vec![day.into(), "Committee".into()],
                ],
                markdown: format!("| Day | Meeting |\n| --- | --- |\n| {day} | Committee |"),
                page_number: 1,
                ..Default::default()
            });
        }
        document.metadata.additional.insert(
            "mean_text_conf".into(),
            serde_json::json!(if rejected_psm { 40 } else { 95 }),
        );
        document.metadata.additional.insert(
            "word_count".into(),
            serde_json::json!(self.text.split_whitespace().count()),
        );
        Ok(document)
    }
}

struct Registered(Arc<Counted>);
impl Drop for Registered {
    fn drop(&mut self) {
        xberg::plugins::unregister_ocr_backend(BACKEND).unwrap();
    }
}

fn backend(failures: usize, text: String, cancel: Option<xberg::cancellation::CancellationToken>) -> Registered {
    let models = tempfile::tempdir().unwrap();
    std::fs::write(models.path().join("eng.traineddata"), b"fake model a").unwrap();
    let backend = Arc::new(Counted {
        calls: AtomicUsize::new(0),
        inputs: Mutex::new(Vec::new()),
        failures,
        rejected_psm: AtomicI32::new(0),
        low_quality: AtomicBool::new(false),
        policy_sensitive: AtomicBool::new(false),
        include_table: AtomicBool::new(false),
        model_changes: AtomicBool::new(false),
        physical_model_changes: AtomicBool::new(false),
        models,
        text,
        cancel,
    });
    xberg::plugins::register_ocr_backend(backend.clone()).unwrap();
    Registered(backend)
}

fn config(registered: &Registered) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: BACKEND.into(),
            tessdata_path: Some(registered.0.models.path().into()),
            pipeline: Some(
                serde_json::from_value(serde_json::json!({"stages": [
                    {"backend": BACKEND, "priority": 100, "language": ["eng"]}
                ]}))
                .unwrap(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract(source: &[u8], config: &ExtractionConfig) -> xberg::Result<ExtractedDocument> {
    helpers::extract_bytes_document_blocking(source, "application/pdf", config)
}

#[test]
#[serial_test::serial]
fn custom_backend_reads_each_encoded_input_and_keeps_words_and_confidence() {
    for (source, pages, expected_calls) in [(ARABIC, 1, 2), (TWO, 2, 4), (THEN_SCANNED, 2, 3)] {
        let registered = backend(0, PROSE.repeat(4), None);
        let result = extract(source, &config(&registered)).unwrap();
        let output = result.pages.as_ref().unwrap();
        assert_eq!(output.len(), pages);
        for page in output {
            assert_eq!(
                page.content.split_whitespace().collect::<Vec<_>>(),
                PROSE.repeat(4).split_whitespace().collect::<Vec<_>>()
            );
            let confidence = page.ocr_confidence.as_ref().unwrap();
            assert_eq!(confidence.score, Some(0.95));
            assert_eq!(confidence.word_count as usize, PROSE.split_whitespace().count() * 4);
        }
        assert_eq!(
            registered.0.calls.load(Ordering::SeqCst),
            expected_calls,
            "custom backends must receive each distinct encoded image: {:?}",
            registered.0.inputs.lock().unwrap()
        );
    }
}

#[test]
#[serial_test::serial]
fn a_failed_probe_retains_native_text_and_does_not_reuse_a_failed_read() {
    let registered = backend(usize::MAX, PROSE.repeat(4), None);
    let result = extract(ARABIC, &config(&registered)).unwrap();
    assert!(registered.0.calls.load(Ordering::SeqCst) > 0);
    assert_eq!(result.extraction_method, Some(xberg::types::ExtractionMethod::Native));
    assert!(result.content.chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)));
}

#[test]
#[serial_test::serial]
fn custom_backend_reads_do_not_survive_an_extraction() {
    for text in [PROSE.repeat(4), PROSE.replace("Tuesday", "Thursday").repeat(4)] {
        let registered = backend(0, text.clone(), None);
        let result = extract(ARABIC, &config(&registered)).unwrap();
        assert_eq!(
            result.content.split_whitespace().collect::<Vec<_>>(),
            text.split_whitespace().collect::<Vec<_>>()
        );
        assert_eq!(registered.0.calls.load(Ordering::SeqCst), 2);
    }
}

#[test]
#[serial_test::serial]
fn cancellation_after_the_probe_does_not_publish_its_accepted_read() {
    let token = xberg::cancellation::CancellationToken::new();
    let registered = backend(0, PROSE.repeat(4), Some(token.clone()));
    let config = ExtractionConfig {
        cancel_token: Some(token),
        ..config(&registered)
    };
    assert!(extract(ARABIC, &config).is_err());
    assert_eq!(registered.0.calls.load(Ordering::SeqCst), 1);
}

#[test]
#[serial_test::serial]
fn probe_and_production_configs_can_return_different_source_words() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.policy_sensitive.store(true, Ordering::SeqCst);
    let mut config = config(&registered);
    config.ocr.as_mut().unwrap().pipeline = None;
    let result = extract(ARABIC, &config).unwrap();
    assert_eq!(
        result.content.split_whitespace().collect::<Vec<_>>(),
        PROSE
            .replace("Tuesday", "Thursday")
            .repeat(4)
            .split_whitespace()
            .collect::<Vec<_>>()
    );
    let inputs = registered.0.inputs.lock().unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].1, inputs[1].1, "decoded pixels are equal");
    assert_eq!(inputs[0].3["quality_thresholds"]["discard_suspected_ocr_noise"], true);
    assert_eq!(inputs[1].3["quality_thresholds"], serde_json::Value::Null);
}

#[test]
#[serial_test::serial]
fn a_changed_backend_model_is_read_again_after_the_accepted_probe() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.model_changes.store(true, Ordering::SeqCst);
    let result = extract(ARABIC, &config(&registered)).unwrap();
    assert_eq!(registered.0.version(), "2.0.0");
    assert_eq!(
        result.content.split_whitespace().collect::<Vec<_>>(),
        PROSE
            .replace("Tuesday", "Thursday")
            .repeat(4)
            .split_whitespace()
            .collect::<Vec<_>>()
    );
    let inputs = registered.0.inputs.lock().unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].1, inputs[1].1);
    assert_eq!(
        inputs[0].3, inputs[1].3,
        "same config with changed model is not a duplicate read"
    );
}

#[test]
#[serial_test::serial]
fn production_keeps_the_table_payload_of_its_configured_read() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.policy_sensitive.store(true, Ordering::SeqCst);
    registered.0.include_table.store(true, Ordering::SeqCst);
    let mut config = config(&registered);
    config.ocr.as_mut().unwrap().pipeline = None;
    let result = extract(ARABIC, &config).unwrap();
    assert_eq!(result.tables.len(), 1);
    assert_eq!(
        result.tables[0].cells,
        vec![vec!["Day", "Meeting"], vec!["Thursday", "Committee"]]
    );
    assert_eq!(registered.0.calls.load(Ordering::SeqCst), 2);
}

fn retry_config(registered: &Registered) -> ExtractionConfig {
    let mut config = config(registered);
    config.ocr.as_mut().unwrap().pipeline = Some(
        serde_json::from_value(
            serde_json::json!({"quality_thresholds": {"pipeline_min_quality": 0.9}, "stages": [
                {"backend": BACKEND, "priority": 100, "language": ["eng"], "tesseract_config": {"psm": 3}},
                {"backend": BACKEND, "priority": 50, "language": ["eng"], "tesseract_config": {"psm": 6}}
            ]}),
        )
        .unwrap(),
    );
    config
}

#[test]
#[serial_test::serial]
fn a_failed_custom_stage_and_its_fallback_run_on_each_encoded_input() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.rejected_psm.store(3, Ordering::SeqCst);
    let result = extract(ARABIC, &retry_config(&registered)).unwrap();
    assert!(result.content.contains("committee met on Tuesday"));
    let inputs = registered.0.inputs.lock().unwrap();
    let modes: Vec<_> = inputs
        .iter()
        .map(|input| input.3["tesseract_config"]["psm"].as_i64().unwrap())
        .collect();
    assert_eq!(modes, vec![3, 6, 3, 6], "the failed stage must execute on both routes");
}

#[test]
#[serial_test::serial]
fn a_quality_retry_still_runs_and_keeps_the_confident_fallback() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.rejected_psm.store(3, Ordering::SeqCst);
    registered.0.low_quality.store(true, Ordering::SeqCst);
    let result = extract(ARABIC, &retry_config(&registered)).unwrap();
    assert!(result.content.contains("committee met on Tuesday"));
    assert_eq!(
        result.pages.as_ref().unwrap()[0].ocr_confidence.as_ref().unwrap().score,
        Some(0.95)
    );
    let inputs = registered.0.inputs.lock().unwrap();
    let modes: Vec<_> = inputs
        .iter()
        .map(|input| input.3["tesseract_config"]["psm"].as_i64().unwrap())
        .collect();
    assert_eq!(modes, vec![3, 6, 3, 6]);
}

#[test]
#[serial_test::serial]
fn custom_backend_keeps_the_table_payload_after_a_changed_encoding() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.include_table.store(true, Ordering::SeqCst);
    let result = extract(ARABIC, &config(&registered)).unwrap();
    assert_eq!(result.tables.len(), 1);
    assert_eq!(
        result.tables[0].cells,
        vec![vec!["Day", "Meeting"], vec!["Tuesday", "Committee"]]
    );
    assert_eq!(registered.0.calls.load(Ordering::SeqCst), 2);
}

#[test]
#[serial_test::serial]
fn changed_traineddata_is_not_reused_even_when_backend_version_is_unchanged() {
    let registered = backend(0, PROSE.repeat(4), None);
    registered.0.physical_model_changes.store(true, Ordering::SeqCst);
    let result = extract(ARABIC, &config(&registered)).unwrap();
    assert_eq!(registered.0.version(), "1.0.0");
    assert_eq!(
        result.content.split_whitespace().collect::<Vec<_>>(),
        PROSE
            .replace("Tuesday", "Thursday")
            .repeat(4)
            .split_whitespace()
            .collect::<Vec<_>>()
    );
    assert_eq!(registered.0.calls.load(Ordering::SeqCst), 2);
}
