//! Xberg adapter for Wave 2 benchmark harness.
//!
//! Provides subprocess-based extraction via xberg with support for:
//! - Native, layout, PaddleOCR, Sceptre, and candle OCR pipelines
//! - Single-file and batch extraction modes
//! - JSON envelope parsing (ExtractEnvelope and BatchEnvelope)
//! - PDF backend selection (`XbergPdfBackend`): `Native` (default) or `Pdfium` (opt-in only,
//!   named explicitly via `--frameworks`; the pdfium shared library is not bundled in benchmark
//!   images yet)

use crate::{
    adapter::{ExecutableBuildIdentity, declared_ocr_language_policy},
    adapters::{native::NativeAdapter, subprocess::SubprocessAdapter},
    error::Result,
    types::{BatchCapability, BatchEntryPoint, BatchTimingScope, OutputFormat, XbergPdfBackend, XbergPipeline},
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use which::which;

mod auto_ocr;
pub(crate) use auto_ocr::auto_ocr_adapter_with_identity;
pub use auto_ocr::{AutoOcrBackend, auto_ocr_framework_name, create_xberg_auto_ocr_adapter, is_auto_ocr_framework};

const XBERG_CLI_BINARY_ENV_VAR: &str = "XBERG_CLI_BINARY";
const SCEPTRE_ORT_OPTIONS_JSON: &str = r#"{"model":{"backend":"ort"}}"#;
const SCEPTRE_TRACT_OPTIONS_JSON: &str = r#"{"model":{"backend":"tract"}}"#;

/// Environment variable that requests per-stage cold-start timing from the xberg CLI (must
/// match `crates/xberg-cli/src/commands/extract.rs::STAGE_TIMING_ENV_VAR`).
///
/// Passed unconditionally to every xberg subprocess invocation this adapter spawns. This is
/// cheap for the CLI to check (a single `std::env::var` read gated behind an `if`) and makes
/// `xberg extract --format json` include a `stage_timings` object in its stdout.
///
/// Known gap: nothing consumes that object yet. `SubprocessAdapter::parse_output`
/// (`adapters/subprocess.rs`) does not extract it, and no benchmark result type carries it, so
/// [`crate::types::StageTimings`] is currently only exercised by its own round-trip tests. The
/// data is emitted and discarded; cold-start attribution needs the parse, a result field, and
/// reporting wired together before it is available.
const STAGE_TIMING_ENV_VAR: &str = "XBERG_EMIT_STAGE_TIMING";
const NATIVE_BENCHMARK_CONFIG_JSON: &str = r#"{"extraction_timeout_secs":1740,"use_cache":false}"#;
const TESSERACT_BENCHMARK_CONFIG_JSON: &str = r#"{
    "extraction_timeout_secs":1740,
    "use_cache":false,
    "force_ocr":true,
    "ocr":{
        "enabled":true,
        "backend":"tesseract"
    }
}"#;

fn benchmark_config_json(tesseract_ocr_enabled: bool) -> &'static str {
    if tesseract_ocr_enabled {
        TESSERACT_BENCHMARK_CONFIG_JSON
    } else {
        NATIVE_BENCHMARK_CONFIG_JSON
    }
}

fn benchmark_base_args(batch: bool, content_format: &str, tesseract_ocr_enabled: bool) -> Vec<String> {
    base_args_with_config(batch, content_format, benchmark_config_json(tesseract_ocr_enabled))
}

fn base_args_with_config(batch: bool, content_format: &str, config_json: &str) -> Vec<String> {
    let subcommand = if batch { "batch" } else { "extract" };
    vec![
        subcommand.to_string(),
        "--no-config-discovery".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--content-format".to_string(),
        content_format.to_string(),
        "--config-json".to_string(),
        config_json.to_string(),
    ]
}

fn push_layout_args(args: &mut Vec<String>) {
    args.extend([
        "--layout".to_string(),
        "true".to_string(),
        "--use-layout-for-markdown".to_string(),
    ]);
}

fn push_layout_v3_args(args: &mut Vec<String>) {
    push_layout_args(args);
    args.extend([
        "--layout-model".to_string(),
        xberg::LayoutDetectionModel::PpDocLayoutV3.to_string(),
    ]);
}

fn push_ocr_args(args: &mut Vec<String>, backend: &str) {
    args.extend([
        "--ocr".to_string(),
        "true".to_string(),
        "--ocr-backend".to_string(),
        backend.to_string(),
        "--force-ocr".to_string(),
        "true".to_string(),
    ]);
}

fn push_sceptre_args(args: &mut Vec<String>, engine_options: &str) {
    push_ocr_args(args, "sceptre");
    args.extend(["--ocr-backend-options".to_string(), engine_options.to_string()]);
}

/// Whether the pipeline runs layout detection, so a failed layout pass must fail the row rather
/// than be scored as "layout made no difference".
pub fn pipeline_runs_layout(pipeline: XbergPipeline) -> bool {
    matches!(
        pipeline,
        XbergPipeline::Layout | XbergPipeline::LayoutV3 | XbergPipeline::LayoutPaddle | XbergPipeline::SceptreOrtLayout
    )
}

fn pipeline_requires_ocr(pipeline: XbergPipeline) -> bool {
    !matches!(
        pipeline,
        XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
    )
}

fn format_slug(output_format: OutputFormat) -> &'static str {
    match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plaintext",
    }
}

fn pdf_backend_suffix(pdf_backend: XbergPdfBackend) -> &'static str {
    match pdf_backend {
        XbergPdfBackend::Native => "",
        XbergPdfBackend::Pdfium => "-pdfium",
    }
}

/// Builds the framework name for a pipeline/format/batch/pdf-backend combination.
///
/// `XbergPdfBackend::Native` must keep producing exactly today's name (no suffix) so existing
/// committed results under `tools/benchmark-harness/results/` stay comparable. Only a non-default
/// backend earns a `-pdfium` suffix, inserted before the `-batch` suffix.
pub fn xberg_framework_name(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    pdf_backend: XbergPdfBackend,
) -> String {
    let batch_suffix = if batch { "-batch" } else { "" };
    format!(
        "xberg-{}-{}{}{batch_suffix}",
        format_slug(output_format),
        pipeline.as_str(),
        pdf_backend_suffix(pdf_backend),
    )
}

/// Framework name of the in-process steady-state variant of a pipeline.
pub fn xberg_steady_framework_name(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    pdf_backend: XbergPdfBackend,
) -> String {
    format!(
        "{}-steady-state{}",
        xberg_framework_name(pipeline, output_format, false, pdf_backend),
        if batch { "-batch" } else { "" }
    )
}

/// Push the flags that force OCR through one named backend.
fn push_forced_ocr_args(args: &mut Vec<String>, backend: &str) {
    args.push("--ocr".to_string());
    args.push("true".to_string());
    args.push("--ocr-backend".to_string());
    args.push(backend.to_string());
    args.push("--force-ocr".to_string());
    args.push("true".to_string());
}

/// Append the CLI flags that distinguish one pipeline from the baseline.
fn push_pipeline_args(args: &mut Vec<String>, pipeline: XbergPipeline) {
    match pipeline {
        XbergPipeline::Baseline => {}
        XbergPipeline::Layout => push_layout_args(args),
        XbergPipeline::LayoutV3 => push_layout_v3_args(args),
        XbergPipeline::PaddleOcr | XbergPipeline::BaselinePaddle => push_forced_ocr_args(args, "paddle-ocr"),
        XbergPipeline::LayoutPaddle => {
            push_layout_args(args);
            push_forced_ocr_args(args, "paddle-ocr");
        }
        XbergPipeline::SceptreOrt => push_sceptre_args(args, SCEPTRE_ORT_OPTIONS_JSON),
        XbergPipeline::SceptreOrtLayout => {
            push_layout_args(args);
            push_sceptre_args(args, SCEPTRE_ORT_OPTIONS_JSON);
        }
        XbergPipeline::SceptreOrtAutoRotate => {
            push_sceptre_args(args, SCEPTRE_ORT_OPTIONS_JSON);
            args.extend(["--ocr-auto-rotate".to_string(), "true".to_string()]);
        }
        XbergPipeline::SceptreTract => push_sceptre_args(args, SCEPTRE_TRACT_OPTIONS_JSON),
        XbergPipeline::CandleTrocr => push_forced_ocr_args(args, "candle-trocr"),
        XbergPipeline::CandlePaddleocrVl | XbergPipeline::CandlePaddleocrVl15 => {
            push_forced_ocr_args(args, "candle-paddleocr-vl");
        }
        XbergPipeline::CandleGlmOcr => push_forced_ocr_args(args, "candle-glm-ocr"),
        XbergPipeline::CandleDeepseekOcr => push_forced_ocr_args(args, "candle-deepseek-ocr"),
    }
}

/// Creates a Xberg adapter for the given pipeline and configuration.
///
/// # Arguments
/// * `pipeline` - The pipeline variant (baseline, layout, paddle-ocr)
/// * `output_format` - Output format for extraction (markdown or plaintext)
/// * `batch` - Whether to use batch extraction mode
/// * `pdf_backend` - PDF engine to select via `--pdf-backend`. `Native` reproduces today's
///   behavior and framework name exactly; `Pdfium` appends a `-pdfium` suffix to the framework
///   name so it never collides with (or silently replaces) an existing `Native` cohort.
///
/// # Returns
/// * `Ok(SubprocessAdapter)` - Configured adapter ready for extraction
/// * `Err(Error)` - If xberg cannot be located
pub fn create_xberg_adapter(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
) -> Result<SubprocessAdapter> {
    if !ocr_enabled && pipeline_requires_ocr(pipeline) {
        return Err(crate::Error::Config(format!(
            "xberg pipeline '{}' requires OCR, but OCR is disabled",
            pipeline.as_str()
        )));
    }

    let cli_path = locate_xberg_cli()?;
    let build_id = probe_xberg_build_id(&cli_path)?;
    let build_identity = ExecutableBuildIdentity {
        build_id,
        path: cli_path,
    };
    adapter_with_identity(pipeline, output_format, batch, ocr_enabled, pdf_backend, build_identity)
}

/// Builds the adapter for a located executable; split from [`create_xberg_adapter`] so the
/// argument and policy wiring is testable without an xberg binary.
pub(crate) fn adapter_with_identity(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
    build_identity: ExecutableBuildIdentity,
) -> Result<SubprocessAdapter> {
    let content_format = match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plain",
    };

    let tesseract_ocr_enabled = ocr_enabled
        && matches!(
            pipeline,
            XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
        );
    let mut args = benchmark_base_args(batch, content_format, tesseract_ocr_enabled);

    push_pipeline_args(&mut args, pipeline);

    args.push("--pdf-backend".to_string());
    args.push(pdf_backend.as_str().to_string());

    let framework_name = xberg_framework_name(pipeline, output_format, batch, pdf_backend);
    assemble_adapter(
        framework_name,
        build_identity,
        args,
        output_format,
        batch,
        pipeline_runs_layout(pipeline),
    )
}

fn assemble_adapter(
    framework_name: String,
    build_identity: ExecutableBuildIdentity,
    args: Vec<String>,
    output_format: OutputFormat,
    batch: bool,
    reject_layout_failures: bool,
) -> Result<SubprocessAdapter> {
    let cli_path = build_identity.path.clone();
    // Derive the benchmarkable format list from xberg's own MIME registry so it never drifts from
    // what xberg can actually extract (a stale hardcoded list silently filtered out fixtures for
    // formats like fb2/eml/msg/rst/org/latex/typst/docbook/tsv, which xberg fully supports).
    let supported_formats = xberg::list_supported_formats()
        .into_iter()
        .map(|format| format.extension)
        .collect();

    let env = vec![(STAGE_TIMING_ENV_VAR.to_string(), "1".to_string())];

    let single_file_args = batch.then(|| {
        let mut single_args = args.clone();
        single_args[0] = "extract".to_string();
        single_args
    });

    let adapter = if batch {
        SubprocessAdapter::with_batch_capability(
            &framework_name,
            cli_path,
            args,
            env,
            supported_formats,
            BatchCapability {
                entry_point: BatchEntryPoint::XbergCliExtractBatch,
                timing_scope: BatchTimingScope::ColdEndToEndSubprocess,
                per_item_timing: true,
            },
        )
    } else {
        SubprocessAdapter::new(&framework_name, cli_path, args, env, supported_formats)
    }
    .with_supported_output_formats(vec![output_format])
    .with_ocr_language_policy(declared_ocr_language_policy(&framework_name))
    .with_executable_build_identity(build_identity);
    // A layout row must prove the detector ran: a failed layout pass otherwise yields plausible
    // text that the harness would score as layout output. ~keep
    let mut adapter = if reject_layout_failures {
        adapter.with_layout_failure_rejection()
    } else {
        adapter
    };
    if let Some(single_args) = single_file_args {
        adapter = adapter.with_single_file_args(single_args);
    }

    Ok(adapter)
}

/// Creates an in-process Xberg adapter for steady-state latency measurements. ~keep
///
/// Only baseline and the two layout pipelines are exposed initially. Single-file mode retains a paired subprocess
/// adapter solely for its cold CLI probe. Batch mode has no truthful equivalent probe and retains
/// only one Rust [`xberg::Engine`] across discarded warmups and measured invocations. ~keep
pub fn create_xberg_steady_adapter(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
    max_threads: Option<usize>,
) -> Result<NativeAdapter> {
    if !matches!(
        pipeline,
        XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
    ) {
        return Err(crate::Error::Config(format!(
            "steady-state Xberg adapter does not yet support pipeline '{}'",
            pipeline.as_str()
        )));
    }

    let config = steady_extraction_config(pipeline, output_format, ocr_enabled, pdf_backend, max_threads)?;
    let name = xberg_steady_framework_name(pipeline, output_format, batch, pdf_backend);
    let reject_layout_failures = pipeline_runs_layout(pipeline);
    if batch {
        let adapter = NativeAdapter::with_identity(name, config);
        return Ok(if reject_layout_failures {
            adapter.reject_layout_failures()
        } else {
            adapter
        });
    }
    let cold_adapter = match max_threads {
        Some(max_threads) => create_xberg_adapter(pipeline, output_format, false, ocr_enabled, pdf_backend)?
            .with_xberg_max_threads(max_threads),
        None => create_xberg_adapter(pipeline, output_format, false, ocr_enabled, pdf_backend)?,
    };
    let adapter = NativeAdapter::with_identity_and_cold_adapter(name, config, cold_adapter);
    Ok(if reject_layout_failures {
        adapter.reject_layout_failures()
    } else {
        adapter
    })
}

fn steady_extraction_config(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
    max_threads: Option<usize>,
) -> Result<xberg::ExtractionConfig> {
    let mut config: xberg::ExtractionConfig = serde_json::from_str(benchmark_config_json(ocr_enabled))?;
    if let Some(ocr) = config.ocr.as_mut()
        && ocr.backend == "tesseract"
    {
        let tesseract = ocr.tesseract_config.get_or_insert_with(Default::default);
        tesseract.language.clone_from(&ocr.language);
        tesseract.use_cache = false;
    }
    config.output_format = match output_format {
        OutputFormat::Markdown => xberg::OutputFormat::Markdown,
        OutputFormat::Plaintext => xberg::OutputFormat::Plain,
    };
    config.pdf_options.get_or_insert_with(Default::default).backend = match pdf_backend {
        XbergPdfBackend::Native => xberg::PdfBackend::Native,
        XbergPdfBackend::Pdfium => xberg::PdfBackend::Pdfium,
    };
    if let Some(max_threads) = max_threads {
        config.concurrency.get_or_insert_with(Default::default).max_threads = Some(max_threads);
    }
    match pipeline {
        XbergPipeline::Layout => {
            config.layout = Some(xberg::LayoutDetectionConfig::default());
            config.use_layout_for_markdown = true;
        }
        XbergPipeline::LayoutV3 => {
            config.layout = Some(xberg::LayoutDetectionConfig {
                model: xberg::LayoutDetectionModel::PpDocLayoutV3,
                ..Default::default()
            });
            config.use_layout_for_markdown = true;
        }
        _ => {}
    }

    Ok(config)
}

fn probe_xberg_build_id(path: &Path) -> Result<String> {
    let output = std::process::Command::new(path)
        .args(["version", "--format", "json"])
        .output()
        .map_err(|error| {
            crate::Error::Benchmark(format!(
                "failed to query Xberg build identity from `{}`: {error}",
                path.display()
            ))
        })?;
    if !output.status.success() {
        return Err(crate::Error::Benchmark(format!(
            "Xberg build identity query failed for `{}` with status {}",
            path.display(),
            output.status
        )));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        crate::Error::Benchmark(format!(
            "Xberg build identity query returned invalid JSON for `{}`: {error}",
            path.display()
        ))
    })?;
    value
        .get("build_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|build_id| !build_id.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            crate::Error::Benchmark(format!(
                "Xberg build identity query returned no build_id for `{}`",
                path.display()
            ))
        })
}

/// Locates the xberg executable.
///
/// Searches in priority order:
/// 1. The executable file specified by `XBERG_CLI_BINARY`
/// 2. `target/release/xberg`
/// 3. `target/debug/xberg`
/// 4. `which xberg`
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the executable
/// * `Err(Error)` - If the override is invalid or xberg cannot be found
fn locate_xberg_cli() -> Result<PathBuf> {
    let binary_override = std::env::var_os(XBERG_CLI_BINARY_ENV_VAR);
    locate_xberg_cli_with_override(binary_override.as_deref())
}

fn locate_xberg_cli_with_override(binary_override: Option<&OsStr>) -> Result<PathBuf> {
    if let Some(binary_override) = binary_override {
        let path = PathBuf::from(binary_override);
        validate_xberg_cli_override(&path)?;
        return Ok(path);
    }

    let release_path = PathBuf::from("target/release/xberg");
    if release_path.exists() {
        return Ok(release_path);
    }

    let debug_path = PathBuf::from("target/debug/xberg");
    if debug_path.exists() {
        return Ok(debug_path);
    }

    if let Ok(path) = which("xberg") {
        return Ok(path);
    }

    Err(crate::Error::Benchmark(
        "xberg binary not found. Build with: cargo build --release -p xberg-cli --features all".to_string(),
    ))
}

fn validate_xberg_cli_override(path: &Path) -> Result<()> {
    let metadata = path.metadata().map_err(|error| {
        crate::Error::Benchmark(format!(
            "{XBERG_CLI_BINARY_ENV_VAR} points to `{}` but its metadata could not be read: {error}",
            path.display()
        ))
    })?;

    if !metadata.is_file() {
        return Err(crate::Error::Benchmark(format!(
            "{XBERG_CLI_BINARY_ENV_VAR} must point to an executable file, but `{}` is not a file",
            path.display()
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(crate::Error::Benchmark(format!(
                "{XBERG_CLI_BINARY_ENV_VAR} must point to an executable file, but `{}` has no execute permission",
                path.display()
            )));
        }
    }

    #[cfg(windows)]
    {
        let extension = path.extension().and_then(OsStr::to_str).map(str::to_ascii_lowercase);
        let is_executable = matches!(extension.as_deref(), Some("exe" | "com" | "cmd" | "bat"));
        if !is_executable {
            return Err(crate::Error::Benchmark(format!(
                "{XBERG_CLI_BINARY_ENV_VAR} must point to an executable file, but `{}` has no executable extension",
                path.display()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
