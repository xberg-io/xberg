//! Shared test doubles and helpers used across the runner test suite.
#![allow(dead_code)]

use crate::adapter::FrameworkAdapter;
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::registry::AdapterRegistry;
use crate::runner::BenchmarkRunner;
use crate::types::{
    BatchCapability, BatchTimingScope, BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat,
    PerformanceMetrics,
};
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub(super) struct SequenceAdapter {
    pub(super) calls: AtomicUsize,
}

pub(super) struct TeardownAdapter {
    pub(super) name: &'static str,
    pub(super) setup_calls: Arc<AtomicUsize>,
    pub(super) teardown_calls: Arc<AtomicUsize>,
    pub(super) fail_setup: bool,
    pub(super) fail_teardown: bool,
}

pub(super) struct FailedWarmupAdapter {
    pub(super) teardown_calls: Arc<AtomicUsize>,
}

/// A non-xberg framework whose one-time warmup extraction fails, then succeeds on every
/// subsequent call. Used to exercise the non-fatal warmup-failure branch (`adapter.name()`
/// not starting with `"xberg"`) at runner.rs ~1159-1177, which was previously untested
/// (Defect S6) — the only existing warmup-failure test (`FailedWarmupAdapter` above) uses an
/// adapter literally named "xberg-failed-warmup" and only exercises the fatal branch.
pub(super) struct NonFatalWarmupFailureAdapter {
    pub(super) calls: AtomicUsize,
}

pub(super) struct TaskErrorAdapter {
    pub(super) calls: AtomicUsize,
}

pub(super) struct StatefulWarmAdapter {
    pub(super) calls: Arc<AtomicUsize>,
    pub(super) timeout_on_call: Option<usize>,
}

pub(super) struct RecordingBatchAdapter {
    pub(super) batches: Arc<std::sync::Mutex<Vec<Vec<String>>>>,
}

pub(super) struct FailingWarmupBatchAdapter {
    pub(super) calls: Arc<AtomicUsize>,
}

impl RecordingBatchAdapter {
    fn success(file_path: &Path, output_format: OutputFormat) -> BenchmarkResult {
        BenchmarkResult {
            framework: "recording".to_string(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size: 1,
            success: true,
            error_message: None,
            error_kind: ErrorKind::None,
            duration: Duration::from_millis(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: Some("ok".to_string()),
            system_load: None,
            fixture_family: None,
        }
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for StatefulWarmAdapter {
    fn name(&self) -> &str {
        "stateful-warm"
    }

    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::WarmInProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let timed_out = self.timeout_on_call == Some(call);
        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size: 1,
            success: !timed_out,
            error_message: timed_out.then(|| "intentional timeout".to_string()),
            error_kind: if timed_out { ErrorKind::Timeout } else { ErrorKind::None },
            duration: Duration::from_millis(1),
            extraction_duration: Some(Duration::from_millis(1)),
            subprocess_overhead: Some(Duration::ZERO),
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities {
                timing_regime: crate::types::TimingRegime::WarmInProcess,
                ..Default::default()
            },
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: (!timed_out).then(|| format!("call-{call}")),
            system_load: None,
            fixture_family: None,
        })
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for RecordingBatchAdapter {
    fn name(&self) -> &str {
        "recording"
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        Ok(Self::success(file_path, output_format))
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        _timeout: Duration,
        _force_ocr: &[bool],
        _ocr_languages: &[Option<String>],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        let mut batches = self.batches.lock().unwrap();
        batches.push(
            file_paths
                .iter()
                .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
                .collect(),
        );
        let sample_id = format!("recording-batch-{}", batches.len());
        drop(batches);
        Ok(file_paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let mut result = Self::success(path, output_format);
                result.framework_capabilities.batch_sample_id = Some(sample_id.clone());
                result.framework_capabilities.batch_performance_sample = Some(index == 0);
                result
            })
            .collect())
    }

    fn batch_capability(&self) -> Option<BatchCapability> {
        Some(BatchCapability {
            entry_point: crate::types::BatchEntryPoint::XbergCliExtractBatch,
            timing_scope: BatchTimingScope::WarmSteadyState,
            per_item_timing: false,
        })
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for FailingWarmupBatchAdapter {
    fn name(&self) -> &str {
        "failing-warmup-batch"
    }

    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::WarmInProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        Ok(RecordingBatchAdapter::success(file_path, output_format))
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        _timeout: Duration,
        _force_ocr: &[bool],
        _ocr_languages: &[Option<String>],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(file_paths
            .iter()
            .map(|path| {
                let mut result = RecordingBatchAdapter::success(path, output_format);
                if call == 0 {
                    result.success = false;
                    result.error_kind = ErrorKind::FrameworkError;
                    result.error_message = Some("intentional discarded warmup failure".to_string());
                }
                result
            })
            .collect())
    }

    fn batch_capability(&self) -> Option<BatchCapability> {
        Some(BatchCapability {
            entry_point: crate::types::BatchEntryPoint::XbergRustEngineExtractBatch,
            timing_scope: BatchTimingScope::WarmSteadyState,
            per_item_timing: false,
        })
    }
}

/// Adapter that always reports success with a fixed, caller-supplied extracted text —
/// used to drive the `run()` quality-scoring loop's silent-zero reclassification with a
/// controlled extracted-text/ground-truth pairing.
pub(super) struct ScriptedTextAdapter {
    pub(super) text: String,
}

#[async_trait::async_trait]
impl FrameworkAdapter for ScriptedTextAdapter {
    fn name(&self) -> &str {
        "scripted-text"
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size: 1,
            success: true,
            error_message: None,
            error_kind: ErrorKind::None,
            duration: Duration::from_millis(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: Some(self.text.clone()),
            system_load: None,
            fixture_family: None,
        })
    }
}

/// Runs `runner.run()` end-to-end for a single "document.pdf" fixture whose ground-truth
/// text is `ground_truth_text`, extracted by [`ScriptedTextAdapter`] returning
/// `extracted_text`. Mirrors the fixture/ground-truth setup used by
/// `standard_single_and_batch_runners_populate_numeric_tf1_and_sf1`.
pub(super) async fn run_scripted_quality_case(ground_truth_text: &str, extracted_text: &str) -> BenchmarkResult {
    run_scripted_quality_case_in_family(ground_truth_text, extracted_text, None).await
}

/// [`run_scripted_quality_case`] for a fixture that declares `metadata.family` when `family` is set.
pub(super) async fn run_scripted_quality_case_in_family(
    ground_truth_text: &str,
    extracted_text: &str,
    family: Option<&str>,
) -> BenchmarkResult {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("document.pdf"), b"pdf").unwrap();
    std::fs::write(temp.path().join("ground_truth.txt"), ground_truth_text).unwrap();
    let fixture_path = temp.path().join("fixture.json");
    let mut fixture = serde_json::json!({
        "document": "document.pdf",
        "file_type": "pdf",
        "file_size": 3,
        "ground_truth": {
            "text_file": "ground_truth.txt",
            "source": "manual"
        }
    });
    if let Some(family) = family {
        fixture["metadata"] = serde_json::json!({ "family": family });
    }
    std::fs::write(&fixture_path, fixture.to_string()).unwrap();

    let mut registry = AdapterRegistry::new();
    registry
        .register(Arc::new(ScriptedTextAdapter {
            text: extracted_text.to_string(),
        }))
        .unwrap();
    let config = BenchmarkConfig {
        benchmark_mode: BenchmarkMode::SingleFile,
        measure_quality: true,
        warmup_iterations: 0,
        benchmark_iterations: 1,
        ..Default::default()
    };
    let mut runner = BenchmarkRunner::new(config, registry);
    runner.load_fixtures(&fixture_path).unwrap();

    let mut results = runner.run(&["scripted-text".to_string()]).await.unwrap();
    results.remove(0)
}
impl SequenceAdapter {
    fn result(&self, file_path: &Path, output_format: OutputFormat) -> BenchmarkResult {
        let success = self.calls.fetch_add(1, Ordering::SeqCst) % 2 == 1;
        BenchmarkResult {
            framework: "sequence".to_string(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size: 10,
            success,
            error_message: (!success).then(|| "measured iteration failed".to_string()),
            error_kind: if success {
                ErrorKind::None
            } else {
                ErrorKind::FrameworkError
            },
            duration: Duration::from_millis(10),
            extraction_duration: Some(Duration::from_millis(5)),
            subprocess_overhead: Some(Duration::from_millis(5)),
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: success.then(|| "successful payload".to_string()),
            system_load: None,
            fixture_family: None,
        }
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for SequenceAdapter {
    fn name(&self) -> &str {
        "sequence"
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, _file_type: &str) -> bool {
        true
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        Ok(self.result(file_path, output_format))
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        _timeout: Duration,
        _force_ocr: &[bool],
        _ocr_languages: &[Option<String>],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        Ok(file_paths.iter().map(|path| self.result(path, output_format)).collect())
    }

    fn batch_capability(&self) -> Option<crate::types::BatchCapability> {
        Some(crate::types::BatchCapability {
            entry_point: crate::types::BatchEntryPoint::XbergCliExtractBatch,
            timing_scope: crate::types::BatchTimingScope::WarmSteadyState,
            per_item_timing: false,
        })
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for TeardownAdapter {
    fn name(&self) -> &str {
        self.name
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, _file_type: &str) -> bool {
        true
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        _file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        _output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        Err(Error::Benchmark("unused test extraction".to_string()))
    }

    async fn setup(&self) -> Result<()> {
        self.setup_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_setup {
            Err(Error::Benchmark(format!("{} setup failed", self.name)))
        } else {
            Ok(())
        }
    }

    async fn teardown(&self) -> Result<()> {
        self.teardown_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_teardown {
            Err(Error::Benchmark(format!("{} teardown failed", self.name)))
        } else {
            Ok(())
        }
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for FailedWarmupAdapter {
    fn name(&self) -> &str {
        "xberg-failed-warmup"
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size: 1,
            success: false,
            error_message: Some("intentional warmup failure".to_string()),
            error_kind: ErrorKind::FrameworkError,
            duration: Duration::from_millis(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
            system_load: None,
            fixture_family: None,
        })
    }

    async fn teardown(&self) -> Result<()> {
        self.teardown_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for NonFatalWarmupFailureAdapter {
    fn name(&self) -> &str {
        "docling-flaky-warmup"
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        // The first call is the one-time framework-level warmup (see `BenchmarkRunner::run`);
        // every later call is a real measured iteration. ~keep
        let is_warmup_call = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size: 1,
            success: !is_warmup_call,
            error_message: is_warmup_call.then(|| "intentional non-fatal warmup failure".to_string()),
            error_kind: if is_warmup_call {
                ErrorKind::FrameworkError
            } else {
                ErrorKind::None
            },
            duration: Duration::from_millis(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: if is_warmup_call {
                OcrStatus::Unknown
            } else {
                OcrStatus::NotUsed
            },
            extracted_text: if is_warmup_call { None } else { Some("ok".to_string()) },
            system_load: None,
            fixture_family: None,
        })
    }
}

#[async_trait::async_trait]
impl FrameworkAdapter for TaskErrorAdapter {
    fn name(&self) -> &str {
        "task-error"
    }
    fn timing_regime(&self) -> crate::types::TimingRegime {
        crate::types::TimingRegime::ColdProcess
    }

    fn supports_format(&self, file_type: &str) -> bool {
        file_type == "pdf"
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        _timeout: Duration,
        _force_ocr: bool,
        _ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(RecordingBatchAdapter::success(file_path, output_format))
        } else {
            Err(Error::Benchmark("intentional task error".to_string()))
        }
    }
}
pub(super) fn write_ordered_cohort(temp: &Path, file_types: &[&str]) -> PathBuf {
    let fixtures = ["d.json", "b.json", "a.json", "c.json"];
    for (fixture_name, file_type) in fixtures.iter().zip(file_types) {
        // The document extension must agree with the declared file_type: Fixture::validate
        // rejects a fixture whose file_type names a different format than its document's
        // own extension resolves to.
        let document_name = fixture_name.replace(".json", &format!(".{file_type}"));
        std::fs::write(temp.join(&document_name), b"x").unwrap();
        std::fs::write(
            temp.join(fixture_name),
            serde_json::json!({
                "document": document_name,
                "file_type": file_type,
                "file_size": 1
            })
            .to_string(),
        )
        .unwrap();
    }
    let manifest = temp.join("cohort.json");
    std::fs::write(
        &manifest,
        serde_json::json!({
            "schema_version": 1,
            "name": "ordered",
            "batch_size": 2,
            "fixtures": fixtures
        })
        .to_string(),
    )
    .unwrap();
    manifest
}
