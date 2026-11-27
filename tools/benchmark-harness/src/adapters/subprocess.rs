//! Subprocess-based adapter for language bindings
//!
//! This adapter provides a base for running extraction via subprocess.
//! It's used by Python, Node.js, and Ruby adapters to execute extraction
//! in separate processes while monitoring resource usage.

use crate::types::{BatchCapability, OcrStatus, OutputFormat, PerformanceMetrics};
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

mod adapter_impl;
mod args;
mod batch_exec;
mod ocr_args;
mod process_exec;
mod single_exec;
mod support;
#[cfg(test)]
mod tests;

/// Base adapter for subprocess-based extraction
///
/// This adapter spawns a subprocess to perform extraction and monitors
/// its resource usage. Subclasses implement the specific command construction
/// for each language binding.
pub struct SubprocessAdapter {
    name: String,
    command: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
    /// The part of `env` added to match xberg's shipped launchers, recorded in provenance.
    launcher_env: Vec<(String, String)>,
    batch_capability: Option<BatchCapability>,
    working_dir: Option<PathBuf>,
    supported_formats: Vec<String>,
    max_timeout: Option<Duration>,
    skip_files: Vec<String>,
    /// When true, append --format=<output_format> to subprocess args
    format_aware: bool,
    supported_output_formats: Vec<OutputFormat>,
    /// Single-file command arguments for adapters whose batch command uses a
    /// different subcommand. Used by warmup and mixed per-file OCR fallback.
    single_file_args: Option<Vec<String>>,
    /// OCR mode requested by an external adapter when its output does not
    /// report whether OCR ran. Xberg adapters leave this unset and use their
    /// emitted per-document metadata.
    configured_ocr_status: Option<OcrStatus>,
    /// Reject an item whose extraction reports a failed layout pass, so a benchmark row can
    /// never be scored as a layout-pipeline result when the requested detector never ran. ~keep
    reject_layout_failures: bool,
    /// Do not upgrade the request to forced OCR when a fixture requires OCR. For adapters whose
    /// point is that the extractor decides per page whether to OCR. ~keep
    ignore_fixture_force_ocr: bool,
    /// Worker limit passed to native batch implementations.
    batch_workers: usize,
    /// Resolved executable used by a specialized native batch path.
    native_batch_command: Option<PathBuf>,
    /// Per-adapter sequence used to distinguish repeated batch invocations.
    batch_sequence: AtomicU64,
    /// Explicit `--max-threads` budget passed to Xberg in either mode.
    ///
    /// When unset, single-file mode preserves Xberg's automatic budget while
    /// native batch mode falls back to [`Self::batch_workers`].
    xberg_max_threads: Option<usize>,
    /// Tesseract model set passed to Xberg as `--ocr-tessdata-model` when it is not the default.
    tessdata_model: xberg::TessdataModel,
    /// CLI flag an external wrapper uses to receive the fixture's OCR language,
    /// forwarded in canonical Tesseract form (e.g. `eng+kor`, `jpn_vert`). The
    /// wrapper maps it onto its own engine's codes. `None` means the framework
    /// exposes no explicit OCR-language selection, so the language is not
    /// forwarded and parity is not assumed on its behalf.
    ocr_language_arg: Option<String>,
    ocr_language_policy: crate::adapter::OcrLanguagePolicy,
    executable_build_identity: Option<crate::adapter::ExecutableBuildIdentity>,
}

impl SubprocessAdapter {
    /// Create a new subprocess adapter
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "xberg-python")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "xberg"])
    /// * `env` - Environment variables
    /// * `supported_formats` - List of file extensions this framework can process (e.g., ["pdf", "docx"])
    pub fn new(
        name: impl Into<String>,
        command: impl Into<PathBuf>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        supported_formats: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            env,
            launcher_env: Vec::new(),
            batch_capability: None,
            working_dir: None,
            supported_formats,
            max_timeout: None,
            skip_files: vec![],
            format_aware: false,
            supported_output_formats: vec![OutputFormat::Markdown],
            single_file_args: None,
            configured_ocr_status: None,
            reject_layout_failures: false,
            ignore_fixture_force_ocr: false,
            batch_workers: 1,
            native_batch_command: None,
            batch_sequence: AtomicU64::new(0),
            xberg_max_threads: None,
            tessdata_model: xberg::TessdataModel::Fast,
            ocr_language_arg: None,
            ocr_language_policy: crate::adapter::OcrLanguagePolicy::DefaultOnly,
            executable_build_identity: None,
        }
    }
    /// Create a new subprocess adapter with batch support
    ///
    /// This adapter will call `extract_batch()` with all files at once,
    /// allowing the subprocess to use its native batch API for parallel processing.
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "xberg-python-batch")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "xberg"])
    /// * `env` - Environment variables
    /// * `supported_formats` - List of file extensions this framework can process
    pub(crate) fn with_batch_capability(
        name: impl Into<String>,
        command: impl Into<PathBuf>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        supported_formats: Vec<String>,
        batch_capability: BatchCapability,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            env,
            launcher_env: Vec::new(),
            batch_capability: Some(batch_capability),
            working_dir: None,
            supported_formats,
            max_timeout: None,
            skip_files: vec![],
            format_aware: false,
            supported_output_formats: vec![OutputFormat::Markdown],
            single_file_args: None,
            configured_ocr_status: None,
            reject_layout_failures: false,
            ignore_fixture_force_ocr: false,
            batch_workers: 1,
            native_batch_command: None,
            batch_sequence: AtomicU64::new(0),
            xberg_max_threads: None,
            tessdata_model: xberg::TessdataModel::Fast,
            ocr_language_arg: None,
            ocr_language_policy: crate::adapter::OcrLanguagePolicy::DefaultOnly,
            executable_build_identity: None,
        }
    }
    /// Set a maximum timeout for this adapter, overriding the global config timeout
    /// if the adapter's max is lower.
    /// Rejects any item whose `processing_warnings` report a failed layout pass.
    pub fn with_layout_failure_rejection(mut self) -> Self {
        self.reject_layout_failures = true;
        self
    }

    /// Keep OCR optional for fixtures that require it, leaving the decision to the extractor.
    pub fn with_ignored_fixture_force_ocr(mut self) -> Self {
        self.ignore_fixture_force_ocr = true;
        self
    }

    pub fn with_max_timeout(mut self, timeout: Duration) -> Self {
        self.max_timeout = Some(timeout);
        self
    }
    /// Set files to skip for this adapter.
    pub fn with_skip_files(mut self, files: Vec<String>) -> Self {
        self.skip_files = files;
        self
    }
    /// Enable format awareness: append --format=<output_format> to subprocess args
    pub fn with_format_aware(mut self, enabled: bool) -> Self {
        self.format_aware = enabled;
        if enabled {
            self.supported_output_formats = vec![OutputFormat::Plaintext, OutputFormat::Markdown];
        }
        self
    }
    pub fn with_supported_output_formats(mut self, formats: Vec<OutputFormat>) -> Self {
        self.supported_output_formats = formats;
        self
    }
    pub fn with_single_file_args(mut self, args: Vec<String>) -> Self {
        self.single_file_args = Some(args);
        self
    }
    /// Record the OCR mode requested from an external framework. This is used
    /// only when the framework does not emit per-document OCR metadata.
    pub fn with_configured_ocr(mut self, enabled: bool) -> Self {
        self.configured_ocr_status = Some(if enabled { OcrStatus::Used } else { OcrStatus::NotUsed });
        self
    }
    /// Configure the CLI flag an external wrapper uses to receive the fixture's
    /// OCR language. Set this only for frameworks that expose explicit
    /// OCR-language selection; the forwarded value is the canonical Tesseract
    /// form (`eng+kor`, `jpn_vert`) and the wrapper maps it to its own engine.
    pub fn with_ocr_language_arg(mut self, flag: impl Into<String>) -> Self {
        self.ocr_language_arg = Some(flag.into());
        self.ocr_language_policy = crate::adapter::OcrLanguagePolicy::AnyPerDocument;
        self
    }
    pub fn with_ocr_language_policy(mut self, policy: crate::adapter::OcrLanguagePolicy) -> Self {
        self.ocr_language_policy = policy;
        self
    }
    pub(crate) fn with_executable_build_identity(mut self, identity: crate::adapter::ExecutableBuildIdentity) -> Self {
        self.executable_build_identity = Some(identity);
        self
    }
    /// Add the environment xberg's shipped launchers set to every process this adapter spawns,
    /// and record it as injected.
    pub(crate) fn with_launcher_env(mut self, launcher_env: Vec<(String, String)>) -> Self {
        self.env.extend(launcher_env.iter().cloned());
        self.launcher_env = launcher_env;
        self
    }
    #[cfg(test)]
    pub(crate) fn env(&self) -> &[(String, String)] {
        &self.env
    }
    /// Set the bounded worker count used by native batch implementations.
    pub fn with_batch_workers(mut self, workers: usize) -> Self {
        self.batch_workers = workers.max(1);
        self
    }
    pub(crate) fn with_native_batch_command(mut self, command: PathBuf) -> Self {
        self.native_batch_command = Some(command);
        self
    }
    /// Set Xberg's configured thread budget independently of batch workers.
    pub fn with_xberg_max_threads(mut self, max_threads: usize) -> Self {
        self.xberg_max_threads = Some(max_threads.max(1));
        self
    }
    /// Set the Tesseract model set Xberg resolves. Ignored by other frameworks.
    pub fn with_tessdata_model(mut self, model: xberg::TessdataModel) -> Self {
        self.tessdata_model = model;
        self
    }
    /// Set the working directory for subprocess execution
    ///
    /// # Arguments
    /// * `dir` - Directory path to change to before running the command
    pub fn set_working_dir(&mut self, dir: PathBuf) {
        self.working_dir = Some(dir);
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            baseline_memory_bytes: 0,
            peak_memory_bytes: 0,
            memory_method: crate::types::MemoryMethod::default(),
            self_reported_peak_memory_bytes: None,
            peak_memory_delta_bytes: 0,
            avg_cpu_percent: 0.0,
            cpu_seconds: 0.0,
            throughput_bytes_per_sec: 0.0,
            p50_memory_bytes: 0,
            p95_memory_bytes: 0,
            p99_memory_bytes: 0,
        }
    }
}
