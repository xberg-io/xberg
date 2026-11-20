//! `run`: executes benchmarks across one or more registered framework adapters.

use crate::cli::CliMode;
use benchmark_harness::adapters::{create_xberg_adapter, create_xberg_steady_adapter};
use benchmark_harness::provenance::{CoverageGateProvenance, FrameworkCoverageProvenance};
use benchmark_harness::types::ErrorKind;
use benchmark_harness::{
    AdapterRegistry, BenchmarkConfig, BenchmarkMode, BenchmarkRunner, ModelProvenance, OutputFormat, Result,
    XbergPdfBackend, XbergPipeline, write_by_extension_analysis, write_json,
};
use clap::Args;
use std::path::PathBuf;
use std::str::FromStr as _;
use std::sync::Arc;

/// `run` subcommand arguments -- pulled out of the `Commands` enum (via `#[command(flatten)]`
/// through the tuple-variant `Run(RunArgs)`) purely to keep `Commands` under the file's
/// type-length limit; the flag grammar itself is unchanged.
#[derive(Args)]
pub(crate) struct RunArgs {
    /// Fixture directory, or a single fixture when --cohort is omitted
    #[arg(short, long)]
    pub(crate) fixtures: PathBuf,

    /// Exact ordered cohort manifest. Absolute paths are used directly; existing
    /// relative paths use the current directory; others use the fixture directory.
    #[arg(long)]
    pub(crate) cohort: Option<PathBuf>,

    /// Require complete native batches of exactly this many documents
    #[arg(long)]
    pub(crate) batch_size: Option<usize>,

    /// Frameworks to benchmark (comma-separated)
    #[arg(short = 'F', long, value_delimiter = ',')]
    pub(crate) frameworks: Vec<String>,

    /// Output directory for results
    #[arg(short, long, default_value = "results")]
    pub(crate) output: PathBuf,

    /// Maximum concurrent extractions
    #[arg(short = 'c', long)]
    pub(crate) max_concurrent: Option<usize>,

    /// Xberg's configured extraction thread budget.
    ///
    /// Single-file mode otherwise uses Xberg's automatic budget; batch mode
    /// otherwise defaults to --max-concurrent. Does not affect other frameworks.
    #[arg(long)]
    pub(crate) xberg_max_threads: Option<usize>,

    /// Timeout in seconds
    #[arg(short = 't', long)]
    pub(crate) timeout: Option<u64>,

    /// Benchmark mode: single-file (sequential) or batch (concurrent)
    #[arg(short = 'm', long, value_enum, default_value = "batch")]
    pub(crate) mode: CliMode,

    /// Number of warmup iterations (discarded from statistics)
    #[arg(short = 'w', long, default_value = "1")]
    pub(crate) warmup: usize,

    /// Number of benchmark iterations for statistical analysis
    #[arg(short = 'i', long, default_value = "3")]
    pub(crate) iterations: usize,

    /// Enable OCR for image extraction
    #[arg(long, default_value = "false")]
    pub(crate) ocr: bool,

    /// Enable quality assessment
    #[arg(long, default_value = "false")]
    pub(crate) measure_quality: bool,

    /// Output format for extraction: markdown, plaintext, or both (default: markdown)
    #[arg(long, default_value = "markdown")]
    pub(crate) output_format: String,

    /// Run only a subset of fixtures (format: INDEX/TOTAL, e.g. 1/3 for first of 3 shards)
    #[arg(long)]
    pub(crate) shard: Option<String>,

    /// Model identity as FRAMEWORK=OWNER/REPOSITORY@REVISION#DIGEST (repeatable)
    #[arg(long = "model-id")]
    pub(crate) model_ids: Vec<String>,

    /// Minimum fraction of attempted, supported documents that must extract successfully.
    /// Unsupported formats are filtered before execution. Infrastructure failures are
    /// reported but excluded from this framework-quality rate. Defaults to 1.0.
    #[arg(long, default_value = "1.0", value_parser = parse_success_rate)]
    pub(crate) min_success_rate: f64,

    /// PDF backends to benchmark (comma-separated: native, pdfium). Only the `baseline`
    /// xberg pipeline honors more than `native` -- the pdfium engine has no OCR fallback
    /// path, so every other pipeline always runs `native` regardless of this flag. Defaults
    /// to `native` only, so existing committed results and framework names stay unchanged
    /// unless a caller opts in. This is the supported way to request the pdfium leg: it
    /// does not require also naming `-pdfium`-suffixed framework names via `--frameworks`.
    #[arg(long, value_delimiter = ',')]
    pub(crate) pdf_backends: Vec<String>,
}

pub(crate) fn normalize_run_frameworks(frameworks: &[String], batch_mode: bool) -> Vec<String> {
    let mut normalized = Vec::with_capacity(frameworks.len());
    for framework in frameworks {
        let name = if batch_mode && framework.starts_with("xberg-") && !framework.ends_with("-batch") {
            format!("{framework}-batch")
        } else {
            framework.clone()
        };
        if !normalized.contains(&name) {
            normalized.push(name);
        }
    }
    normalized
}

pub(crate) fn parse_success_rate(value: &str) -> std::result::Result<f64, String> {
    let rate = value
        .parse::<f64>()
        .map_err(|error| format!("invalid success rate {value:?}: {error}"))?;
    if rate.is_finite() && (0.0..=1.0).contains(&rate) {
        Ok(rate)
    } else {
        Err(format!("success rate must be within 0.0..=1.0, got {value}"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FrameworkCoverage {
    pub(crate) framework: String,
    pub(crate) successful: usize,
    pub(crate) framework_failures: usize,
    pub(crate) infrastructure_failures: usize,
    pub(crate) success_rate: f64,
}

impl FrameworkCoverage {
    /// Extractions that count toward the success rate: framework successes and framework-fault
    /// failures. Infrastructure failures are excluded because they are not the framework's fault.
    pub(crate) fn accountable(&self) -> usize {
        self.successful + self.framework_failures
    }

    /// A framework with zero accountable results never demonstrated any success, regardless of
    /// `min_success_rate`, so it always fails the gate rather than dividing by zero into a
    /// misleadingly clean `0.0 >= 0.0`.
    pub(crate) fn below_gate(&self, min_success_rate: f64) -> bool {
        self.accountable() == 0 || self.success_rate < min_success_rate
    }
}

/// Compute per-framework success-rate accounting for every framework present in `results`.
///
/// This never fails: every framework that produced at least one result is represented, including
/// ones that fell below any success-rate threshold. Threshold enforcement is a separate step
/// ([`FrameworkCoverage::below_gate`]) so callers can persist the full accounting before deciding
/// whether to fail the run.
pub(crate) fn compute_framework_coverage<'a>(
    results: impl IntoIterator<Item = (&'a str, bool, ErrorKind)>,
) -> Vec<FrameworkCoverage> {
    let mut counts = std::collections::BTreeMap::<String, (usize, usize, usize)>::new();
    for (framework, success, error_kind) in results {
        let entry = counts.entry(framework.to_string()).or_default();
        if success {
            entry.0 += 1;
        } else if matches!(
            error_kind,
            ErrorKind::FrameworkError | ErrorKind::EmptyContent | ErrorKind::ZeroOverlap | ErrorKind::Timeout
        ) {
            entry.1 += 1;
        } else {
            entry.2 += 1;
        }
    }

    counts
        .into_iter()
        .map(
            |(framework, (successful, framework_failures, infrastructure_failures))| {
                let accountable = successful + framework_failures;
                let success_rate = if accountable == 0 {
                    0.0
                } else {
                    successful as f64 / accountable as f64
                };
                FrameworkCoverage {
                    framework,
                    successful,
                    framework_failures,
                    infrastructure_failures,
                    success_rate,
                }
            },
        )
        .collect()
}

pub(crate) fn validate_framework_result_cardinality<'a, 'b>(
    result_frameworks: impl IntoIterator<Item = &'a str>,
    expected_frameworks: impl IntoIterator<Item = (&'b str, usize)>,
) -> std::result::Result<(), String> {
    let mut expected = std::collections::BTreeMap::new();
    for (framework, eligible_documents) in expected_frameworks {
        if expected.insert(framework.to_string(), eligible_documents).is_some() {
            return Err(format!("duplicate eligible provenance entry for {framework}"));
        }
    }

    let mut actual = std::collections::BTreeMap::<String, usize>::new();
    for framework in result_frameworks {
        if !expected.contains_key(framework) {
            return Err(format!(
                "{framework} produced results without an eligible provenance entry"
            ));
        }
        *actual.entry(framework.to_string()).or_default() += 1;
    }

    for (framework, eligible_documents) in expected {
        let produced = actual.get(&framework).copied().unwrap_or(0);
        if produced != eligible_documents {
            return Err(format!(
                "{framework} produced {produced} result(s), expected {eligible_documents} eligible document result(s)"
            ));
        }
    }
    Ok(())
}

/// Writes `results.json`, `by-extension.json`, and `provenance.json` for a completed run, then
/// evaluates the minimum-success-rate gate.
///
/// Artifacts are always written before the gate is checked: a run that falls below
/// `min_success_rate` still leaves a complete, inspectable record on disk (including every other
/// framework's results), instead of the whole artifact silently vanishing on the first
/// underperforming framework. The gate outcome — threshold, per-framework accounting, and which
/// frameworks failed it — is recorded in `provenance.coverage` before the provenance file is
/// written. The gate itself is unchanged: a below-threshold run still returns `Err`, so the
/// process still exits non-zero.
pub(crate) fn write_run_artifacts_and_check_gate(
    results: &[benchmark_harness::BenchmarkResult],
    mut provenance: benchmark_harness::RunProvenance,
    output: &std::path::Path,
    min_success_rate: f64,
) -> Result<()> {
    let coverage = compute_framework_coverage(
        results
            .iter()
            .map(|result| (result.framework.as_str(), result.success, result.error_kind)),
    );
    let failing_frameworks: Vec<String> = coverage
        .iter()
        .filter(|framework| framework.below_gate(min_success_rate))
        .map(|framework| framework.framework.clone())
        .collect();
    let passed = failing_frameworks.is_empty();

    provenance.coverage = Some(CoverageGateProvenance {
        min_success_rate,
        frameworks: coverage
            .iter()
            .map(|framework| FrameworkCoverageProvenance {
                framework: framework.framework.clone(),
                successful: framework.successful,
                framework_failures: framework.framework_failures,
                infrastructure_failures: framework.infrastructure_failures,
                success_rate: framework.success_rate,
            })
            .collect(),
        passed,
        failing_frameworks: failing_frameworks.clone(),
    });

    for framework in &coverage {
        if framework.framework_failures > 0 || framework.infrastructure_failures > 0 {
            println!(
                "  {}: {}/{} supported extractions succeeded ({:.1}% >= {:.1}% required); \
                 {} infrastructure failure(s) excluded",
                framework.framework,
                framework.successful,
                framework.accountable(),
                framework.success_rate * 100.0,
                min_success_rate * 100.0,
                framework.infrastructure_failures,
            );
        }
    }

    let output_file = output.join("results.json");
    write_json(results, &output_file)?;
    println!("\nResults written to: {}", output_file.display());

    let by_ext_file = output.join("by-extension.json");
    write_by_extension_analysis(results, &by_ext_file)?;
    println!("Per-extension analysis written to: {}", by_ext_file.display());

    let provenance_file = output.join("provenance.json");
    benchmark_harness::write_run_provenance(&provenance, &provenance_file)?;
    println!("Run provenance written to: {}", provenance_file.display());

    if !passed {
        return Err(benchmark_harness::Error::Benchmark(format!(
            "{} of {} framework(s) fell below the required minimum success rate {min_success_rate:.3}: {}",
            failing_frameworks.len(),
            coverage.len(),
            failing_frameworks.join(", ")
        )));
    }

    Ok(())
}

pub(crate) fn selected_frameworks_use_tesseract(frameworks: &[String]) -> bool {
    frameworks.is_empty()
        || frameworks.iter().any(|framework| {
            framework.starts_with("xberg-")
                && !framework.contains("paddle")
                && !framework.contains("sceptre")
                && (framework.contains("-baseline") || framework.contains("-layout"))
        })
}

pub(crate) const XBERG_RUN_PIPELINES: [XbergPipeline; 10] = [
    XbergPipeline::Baseline,
    XbergPipeline::Layout,
    XbergPipeline::LayoutV3,
    XbergPipeline::PaddleOcr,
    XbergPipeline::BaselinePaddle,
    XbergPipeline::LayoutPaddle,
    XbergPipeline::SceptreOrt,
    XbergPipeline::SceptreOrtLayout,
    XbergPipeline::SceptreOrtAutoRotate,
    XbergPipeline::SceptreTract,
];

pub(crate) fn should_register_xberg_pipeline(pipeline: XbergPipeline, has_explicit_frameworks: bool) -> bool {
    !matches!(
        pipeline,
        XbergPipeline::LayoutV3 | XbergPipeline::PaddleOcr | XbergPipeline::SceptreTract
    ) || has_explicit_frameworks
}

/// Parses `run --pdf-backends`. An empty list (the flag omitted) means "unchanged default
/// behavior" and is handled by the caller, not here -- this function only validates values a
/// caller actually supplied, and rejects duplicates so a typo like `native,native` cannot be
/// mistaken for `native,pdfium`.
pub(crate) fn parse_pdf_backends(values: &[String]) -> Result<Vec<XbergPdfBackend>> {
    let mut backends = Vec::with_capacity(values.len());
    for value in values {
        let backend = value
            .parse::<XbergPdfBackend>()
            .map_err(|error| benchmark_harness::Error::Config(format!("invalid --pdf-backends value: {error}")))?;
        if backends.contains(&backend) {
            return Err(benchmark_harness::Error::Config(format!(
                "--pdf-backends lists '{backend}' more than once"
            )));
        }
        backends.push(backend);
    }
    Ok(backends)
}

pub(crate) fn parse_model_provenance(values: &[String]) -> Result<Vec<ModelProvenance>> {
    values
        .iter()
        .map(|value| {
            let (framework, identifier) = value.split_once('=').ok_or_else(|| {
                benchmark_harness::Error::Config(format!(
                    "invalid model identity '{value}': expected FRAMEWORK=OWNER/REPOSITORY@REVISION#DIGEST"
                ))
            })?;
            let valid_component = |value: &str| {
                !value.is_empty()
                    && value
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
            };
            let structured = identifier
                .split_once('#')
                .and_then(|(repository_revision, digest)| {
                    repository_revision
                        .split_once('@')
                        .map(|(repository, revision)| (repository, revision, digest))
                })
                .is_some_and(|(repository, revision, digest)| {
                    let mut repository_parts = repository.split('/');
                    valid_component(repository_parts.next().unwrap_or_default())
                        && valid_component(repository_parts.next().unwrap_or_default())
                        && repository_parts.next().is_none()
                        && valid_component(revision)
                        && !digest.is_empty()
                });
            if framework.is_empty() || !structured {
                return Err(benchmark_harness::Error::Config(format!(
                    "invalid model identity '{value}': expected path-free FRAMEWORK=OWNER/REPOSITORY@REVISION#DIGEST"
                )));
            }
            Ok(ModelProvenance {
                framework: framework.to_string(),
                identifier: identifier.to_string(),
            })
        })
        .collect()
}

/// Shared, read-only state for registering `xberg-*` adapter permutations -- bundled into one
/// struct purely to keep the registration helpers' parameter counts down; carries no behavior
/// of its own.
struct XbergRegistration<'a> {
    config: &'a BenchmarkConfig,
    batch_mode: bool,
    has_explicit_frameworks: bool,
    ocr: bool,
    format: OutputFormat,
    should_init: &'a dyn Fn(&str) -> bool,
}

/// Registers one `xberg-*` adapter permutation, if `should_init` selects it.
fn register_one_xberg_adapter(
    registry: &mut AdapterRegistry,
    context: &XbergRegistration<'_>,
    pipeline: XbergPipeline,
    pdf_backend: XbergPdfBackend,
    xberg_count: &mut usize,
) {
    let format_slug = match context.format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plaintext",
    };
    let pdf_backend_suffix = match pdf_backend {
        XbergPdfBackend::Native => "",
        XbergPdfBackend::Pdfium => "-pdfium",
    };
    let base_name = format!("xberg-{}-{}{}", format_slug, pipeline.as_str(), pdf_backend_suffix);
    let framework_name = if context.batch_mode {
        format!("{base_name}-batch")
    } else {
        base_name
    };
    if !(context.should_init)(&framework_name) {
        return;
    }
    match create_xberg_adapter(pipeline, context.format, context.batch_mode, context.ocr, pdf_backend)
        .map(|adapter| adapter.with_batch_workers(context.config.max_concurrent))
        .map(|adapter| match context.config.xberg_max_threads {
            Some(max_threads) => adapter.with_xberg_max_threads(max_threads),
            None => adapter,
        }) {
        Ok(adapter) => {
            if let Err(err) = registry.register(Arc::new(adapter)) {
                tracing::warn!(
                    framework = %framework_name,
                    error = %err,
                    "adapter registration failed"
                );
            } else {
                tracing::info!(framework = %framework_name, "adapter registered");
                *xberg_count += 1;
            }
        }
        Err(err) => {
            tracing::warn!(
                framework = %framework_name,
                error = %err,
                "adapter initialization failed"
            );
        }
    }
}

fn register_steady_xberg_adapter(
    registry: &mut AdapterRegistry,
    context: &XbergRegistration<'_>,
    pipeline: XbergPipeline,
    pdf_backend: XbergPdfBackend,
) -> bool {
    if !context.has_explicit_frameworks
        || !matches!(
            pipeline,
            XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
        )
    {
        return false;
    }
    let format_slug = match context.format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plaintext",
    };
    let backend_suffix = if pdf_backend == XbergPdfBackend::Pdfium {
        "-pdfium"
    } else {
        ""
    };
    let batch_suffix = if context.batch_mode { "-batch" } else { "" };
    let steady_name = format!(
        "xberg-{format_slug}-{}{backend_suffix}-steady-state{batch_suffix}",
        pipeline.as_str()
    );
    if !(context.should_init)(&steady_name) {
        return false;
    }
    match create_xberg_steady_adapter(
        pipeline,
        context.format,
        context.batch_mode,
        context.ocr,
        pdf_backend,
        context.config.xberg_max_threads,
    ) {
        Ok(adapter) => match registry.register(Arc::new(adapter)) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(framework = %steady_name, %error, "steady-state adapter registration failed");
                false
            }
        },
        Err(error) => {
            tracing::warn!(framework = %steady_name, %error, "steady-state adapter initialization failed");
            false
        }
    }
}

/// Registers every eligible `xberg-*` adapter permutation (pipeline x pdf backend) for `run`.
fn register_xberg_pipelines(
    registry: &mut AdapterRegistry,
    context: &XbergRegistration<'_>,
    effective_pdf_backends: &[XbergPdfBackend],
    has_explicit_frameworks: bool,
) -> usize {
    // `Pdfium` is a distinct cohort dimension, not a pipeline: it is opt-in only (must be
    // requested explicitly via `--pdf-backends pdfium`) and restricted to `Baseline`, since the
    // pdfium engine has no OCR fallback path of its own. Every other pipeline always runs
    // `native`, regardless of `--pdf-backends`. ~keep
    let native_only_backend = [XbergPdfBackend::Native];
    let mut xberg_count = 0;
    for pipeline in &XBERG_RUN_PIPELINES {
        if !should_register_xberg_pipeline(*pipeline, has_explicit_frameworks) {
            continue;
        }
        if !context.ocr
            && matches!(
                pipeline,
                XbergPipeline::PaddleOcr
                    | XbergPipeline::BaselinePaddle
                    | XbergPipeline::LayoutPaddle
                    | XbergPipeline::SceptreOrt
                    | XbergPipeline::SceptreOrtLayout
                    | XbergPipeline::SceptreOrtAutoRotate
                    | XbergPipeline::SceptreTract
            )
        {
            continue;
        }
        let pdf_backends: &[XbergPdfBackend] = if matches!(pipeline, XbergPipeline::Baseline) {
            effective_pdf_backends
        } else {
            &native_only_backend
        };
        for pdf_backend in pdf_backends {
            register_one_xberg_adapter(registry, context, *pipeline, *pdf_backend, &mut xberg_count);
            if register_steady_xberg_adapter(registry, context, *pipeline, *pdf_backend) {
                xberg_count += 1;
            }
        }
    }
    xberg_count
}

/// Registers the open-source competitor adapters selected by `should_init`. Batch mode only
/// registers frameworks with a verified native batch API; single-file mode registers the full
/// external roster.
fn register_external_adapters(
    registry: &mut AdapterRegistry,
    config: &BenchmarkConfig,
    ocr: bool,
    batch_mode: bool,
    should_init: &impl Fn(&str) -> bool,
) -> usize {
    let mut external_count = 0;

    macro_rules! try_register {
        ($name:expr, $create_fn:expr, $count:expr) => {
            if should_init($name) {
                match $create_fn() {
                    Ok(adapter) => {
                        if let Err(err) = registry.register(Arc::new(adapter)) {
                            tracing::warn!(
                                framework = $name,
                                error = %err,
                                "adapter registration failed"
                            );
                        } else {
                            tracing::info!(framework = $name, "adapter registered");
                            $count += 1;
                        }
                    }
                    Err(err) => {
                        tracing::warn!(
                            framework = $name,
                            error = %err,
                            "adapter initialization failed"
                        );
                    }
                }
            }
        };
    }

    if !batch_mode {
        use benchmark_harness::adapters::{
            create_docling_adapter, create_liteparse_adapter, create_markitdown_adapter, create_mineru_adapter,
            create_pymupdf4llm_adapter, create_tika_adapter, create_unstructured_adapter,
        };

        try_register!("docling", || create_docling_adapter(ocr), external_count);
        try_register!("markitdown", || create_markitdown_adapter(ocr), external_count);
        try_register!("unstructured", || create_unstructured_adapter(ocr), external_count);
        try_register!("tika", || create_tika_adapter(ocr), external_count);
        try_register!("pymupdf4llm", || create_pymupdf4llm_adapter(ocr), external_count);
        try_register!("mineru", || create_mineru_adapter(ocr), external_count);
        try_register!(
            "liteparse",
            || create_liteparse_adapter(ocr).map(|adapter| adapter.with_batch_workers(config.max_concurrent)),
            external_count
        );
    } else {
        use benchmark_harness::adapters::{create_docling_adapter, create_liteparse_adapter};
        try_register!("docling", || create_docling_adapter(ocr), external_count);
        try_register!(
            "liteparse",
            || create_liteparse_adapter(ocr).map(|adapter| adapter.with_batch_workers(config.max_concurrent)),
            external_count
        );
        tracing::debug!(
            docling_api = "docling_jobkit.convert_documents",
            docling_execution = "cold end-to-end subprocess",
            liteparse_api = "batch-parse",
            "verified batch APIs"
        );
        tracing::debug!(
            reason = "native batch behavior is unverified",
            "other external frameworks skipped"
        );
    }

    external_count
}

/// Builds and validates the `AdapterRegistry` for `run`: every requested (or, if none were named,
/// every default) xberg pipeline permutation, plus every eligible external framework.
fn build_registry(
    args: &RunArgs,
    config: &BenchmarkConfig,
    frameworks: &[String],
    parsed_format: OutputFormat,
    batch_mode: bool,
    requested_pdf_backends: &[XbergPdfBackend],
) -> Result<AdapterRegistry> {
    let mut registry = AdapterRegistry::new();
    let should_init = |name: &str| -> bool { frameworks.is_empty() || frameworks.iter().any(|f| f == name) };

    let has_explicit_frameworks = !frameworks.is_empty();
    // An omitted `--pdf-backends` means "unchanged default behavior": native only, for every
    // pipeline, exactly as before this flag existed.
    let native_only_backend = [XbergPdfBackend::Native];
    let effective_pdf_backends: &[XbergPdfBackend] = if requested_pdf_backends.is_empty() {
        &native_only_backend
    } else {
        requested_pdf_backends
    };

    let context = XbergRegistration {
        config,
        batch_mode,
        has_explicit_frameworks,
        ocr: args.ocr,
        format: parsed_format,
        should_init: &should_init,
    };
    let xberg_count =
        register_xberg_pipelines(&mut registry, &context, effective_pdf_backends, has_explicit_frameworks);

    let total_requested = if frameworks.is_empty() {
        if args.ocr { 4 } else { 2 }
    } else {
        frameworks.iter().filter(|f| f.contains("xberg")).count()
    };
    tracing::info!(
        available = xberg_count,
        requested = total_requested,
        "Xberg adapters available"
    );

    let external_count = register_external_adapters(&mut registry, config, args.ocr, batch_mode, &should_init);
    tracing::info!(
        available = external_count,
        supported = 7,
        "open source extraction adapters available"
    );
    tracing::info!(available = xberg_count + external_count, "total adapters available");

    Ok(registry)
}

/// Validates `--frameworks` names, checks every requested one was actually registered, and
/// returns the diagnostic list (empty means every requested framework is available).
fn missing_frameworks(registry: &AdapterRegistry, frameworks: &[String]) -> Vec<String> {
    // NOTE: This check must run AFTER all adapters (xberg + external) are registered
    let mut failed_frameworks = Vec::new();
    for name in frameworks {
        if !registry.contains(name) {
            failed_frameworks.push(name.clone());
        }
    }
    failed_frameworks
}

/// Resolves the fixed native batch size from the cohort manifest and/or `--batch-size`,
/// validating that an explicit `--batch-size` (if any) agrees with the cohort's own.
fn resolve_fixed_batch_size(
    batch_mode: bool,
    batch_size: Option<usize>,
    cohort_manifest: Option<&benchmark_harness::CohortManifest>,
) -> Result<Option<usize>> {
    if batch_size.is_some() && !batch_mode {
        return Err(benchmark_harness::Error::Config(
            "fixed batch sizing requires --mode batch".to_string(),
        ));
    }
    if !batch_mode {
        return Ok(None);
    }
    match (cohort_manifest, batch_size) {
        (Some(manifest), Some(requested)) if requested != manifest.batch_size => {
            Err(benchmark_harness::Error::Config(format!(
                "--batch-size {requested} does not match cohort batch_size {}",
                manifest.batch_size
            )))
        }
        (Some(manifest), _) => Ok(Some(manifest.batch_size)),
        (None, requested) => Ok(requested),
    }
}

/// Parses and applies `--shard INDEX/TOTAL`, when present.
fn apply_shard(runner: &mut BenchmarkRunner, shard: Option<&str>) -> Result<()> {
    let Some(shard_spec) = shard else {
        return Ok(());
    };
    let parts: Vec<&str> = shard_spec.split('/').collect();
    if parts.len() != 2 {
        return Err(benchmark_harness::Error::Config(format!(
            "Invalid shard format '{}': expected INDEX/TOTAL (e.g. 1/3)",
            shard_spec
        )));
    }
    let index: usize = parts[0].parse().map_err(|_| {
        benchmark_harness::Error::Config(format!("Invalid shard index '{}': must be a number", parts[0]))
    })?;
    let total: usize = parts[1].parse().map_err(|_| {
        benchmark_harness::Error::Config(format!("Invalid shard total '{}': must be a number", parts[1]))
    })?;
    if index < 1 || index > total || total < 1 {
        return Err(benchmark_harness::Error::Config(format!(
            "Invalid shard {}/{}: index must be 1..=total",
            index, total
        )));
    }
    let total_before = runner.fixture_count();
    runner.apply_shard(index, total);
    println!(
        "Shard {}/{}: {} of {} fixtures",
        index,
        total,
        runner.fixture_count(),
        total_before
    );
    Ok(())
}

fn validate_framework_names(frameworks: &[String]) -> Result<()> {
    for framework in frameworks {
        if !framework.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(benchmark_harness::Error::Benchmark(format!(
                "Invalid framework name '{}': must contain only alphanumeric characters, hyphens, or underscores",
                framework
            )));
        }
    }
    Ok(())
}

fn build_config(args: &RunArgs) -> BenchmarkConfig {
    BenchmarkConfig {
        output_dir: args.output.clone(),
        max_concurrent: args.max_concurrent.unwrap_or_else(num_cpus::get),
        xberg_max_threads: args.xberg_max_threads,
        timeout: std::time::Duration::from_secs(args.timeout.unwrap_or(1800)),
        benchmark_mode: args.mode.into(),
        warmup_iterations: args.warmup,
        benchmark_iterations: args.iterations,
        measure_quality: args.measure_quality,
        ocr_enabled: args.ocr,
        ..Default::default()
    }
}

/// Everything [`execute`] needs after registry construction, fixture loading, and shard/batch
/// resolution -- the part of `run` that can fail before any benchmark actually executes.
struct RunSetup {
    runner: BenchmarkRunner,
    frameworks: Vec<String>,
    model_provenance: Vec<ModelProvenance>,
    cohort_manifest: Option<benchmark_harness::CohortManifest>,
    fixed_batch_size: Option<usize>,
    failed_frameworks: Vec<String>,
}

async fn setup(args: &RunArgs) -> Result<RunSetup> {
    let requested_pdf_backends = parse_pdf_backends(&args.pdf_backends)?;
    validate_framework_names(&args.frameworks)?;

    let config = build_config(args);
    config.validate()?;

    let parsed_format = OutputFormat::from_str(&args.output_format).map_err(benchmark_harness::Error::Config)?;
    let batch_mode = matches!(config.benchmark_mode, BenchmarkMode::Batch);
    let frameworks = normalize_run_frameworks(&args.frameworks, batch_mode);
    let model_provenance = parse_model_provenance(&args.model_ids)?;

    let registry = build_registry(
        args,
        &config,
        &frameworks,
        parsed_format,
        batch_mode,
        &requested_pdf_backends,
    )?;
    let failed_frameworks = missing_frameworks(&registry, &frameworks);
    if !failed_frameworks.is_empty() {
        return Err(benchmark_harness::Error::Config(format!(
            "{} requested framework(s) are unavailable: {}",
            failed_frameworks.len(),
            failed_frameworks.join(", ")
        )));
    }

    let mut runner = BenchmarkRunner::with_output_format(config, registry, parsed_format);
    let cohort_manifest = if let Some(manifest_path) = args.cohort.as_deref() {
        Some(runner.load_cohort(&args.fixtures, manifest_path)?)
    } else {
        runner.load_fixtures(&args.fixtures)?;
        None
    };

    // Fail fast if any fixture pins an OCR language whose Tesseract pack is not installed
    // locally: xberg would otherwise download it inside the timed extraction and corrupt the
    // measurement.
    benchmark_harness::ocr_preflight::run(
        &args.fixtures,
        args.cohort.as_deref(),
        args.ocr,
        selected_frameworks_use_tesseract(&frameworks),
    )?;

    let fixed_batch_size = resolve_fixed_batch_size(batch_mode, args.batch_size, cohort_manifest.as_ref())?;
    if let Some(size) = fixed_batch_size {
        runner.set_fixed_batch_size(size)?;
    }

    if (args.cohort.is_some() || fixed_batch_size.is_some()) && args.shard.is_some() {
        return Err(benchmark_harness::Error::Config(
            "--shard cannot be combined with exact cohort or fixed batch sizing".to_string(),
        ));
    }
    apply_shard(&mut runner, args.shard.as_deref())?;

    Ok(RunSetup {
        runner,
        frameworks,
        model_provenance,
        cohort_manifest,
        fixed_batch_size,
        failed_frameworks,
    })
}

fn print_run_summary(results: &[benchmark_harness::BenchmarkResult]) {
    println!("\nCompleted {} benchmark(s)", results.len());

    let mut success_count = 0;
    let mut failure_count = 0;
    for result in results {
        if result.success {
            success_count += 1;
        } else {
            failure_count += 1;
        }
    }

    println!("\nSummary:");
    println!("  Successful: {}", success_count);
    println!("  Failed: {}", failure_count);
    println!("  Total: {}", results.len());
}

fn finalize_run(
    results: &[benchmark_harness::BenchmarkResult],
    provenance: benchmark_harness::RunProvenance,
    failed_frameworks: &[String],
    args: &RunArgs,
) -> Result<()> {
    if !failed_frameworks.is_empty() {
        return Err(benchmark_harness::Error::Benchmark(format!(
            "Requested framework(s) failed to initialize: {}",
            failed_frameworks.join(", ")
        )));
    }

    if results.is_empty() {
        return Err(benchmark_harness::Error::Benchmark(
            "No benchmark results were produced".to_string(),
        ));
    }

    // Provenance records the exact supported-document count before execution. Comparing it
    // before publication prevents an omitted task from becoming a valid-looking partial
    // artifact. ~keep
    validate_framework_result_cardinality(
        results.iter().map(|result| result.framework.as_str()),
        provenance
            .frameworks
            .iter()
            .map(|framework| (framework.name.as_str(), framework.eligible_documents)),
    )
    .map_err(benchmark_harness::Error::Benchmark)?;

    // Writes results.json, by-extension.json, and provenance.json (recording the gate outcome
    // for every framework) BEFORE evaluating min_success_rate. A run that falls below the
    // threshold still leaves a complete, inspectable artifact on disk; only then does the
    // process exit non-zero. ~keep
    write_run_artifacts_and_check_gate(results, provenance, &args.output, args.min_success_rate)
}

pub(crate) async fn execute(args: RunArgs) -> Result<()> {
    let RunSetup {
        mut runner,
        frameworks,
        model_provenance,
        cohort_manifest,
        fixed_batch_size,
        failed_frameworks,
    } = setup(&args).await?;

    println!("Loaded {} fixture(s)", runner.fixture_count());
    println!("Frameworks: {:?}", frameworks);
    println!("Configuration: {:?}", runner.config());

    if runner.fixture_count() == 0 {
        println!("No fixtures to benchmark");
        return Ok(());
    }

    let provenance = runner.capture_provenance(
        &frameworks,
        &args.fixtures,
        cohort_manifest.as_ref(),
        args.cohort.as_deref(),
        fixed_batch_size,
        &model_provenance,
    )?;

    println!("\nRunning benchmarks...");
    let results = runner.run(&frameworks).await?;
    print_run_summary(&results);

    finalize_run(&results, provenance, &failed_frameworks, &args)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
